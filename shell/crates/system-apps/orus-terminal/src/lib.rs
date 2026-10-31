pub mod config;
mod core;
pub mod grid;
mod pty;

pub use crate::core::{TerminalCore, TerminalKey, TerminalState};
pub use config::TerminalConfig;
pub use grid::{Cell, CellAttrs, Grid, Row};
pub mod renderer;
pub use renderer::{CellMetrics, TerminalRenderer};

use std::path::PathBuf;

pub fn username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

pub fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .or_else(|_| {
            std::process::Command::new("hostname")
                .arg("-s")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|_| "localhost".to_string())
}

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Shorten a path by replacing $HOME with ~.
pub fn shorten_path(path: &std::path::Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        let home_path = std::path::Path::new(&home);
        if let Ok(rest) = path.strip_prefix(home_path) {
            if rest.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

/// Snapshot the environment from an interactive shell once at startup.
/// This captures PATH, aliases-exported-as-functions, etc. without paying
/// the interactive-shell cost on every command.
pub fn snapshot_env() -> Vec<(String, String)> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let out = std::process::Command::new(&shell)
        .args(["-i", "-c", "env"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Find the longest common prefix among a list of strings.
pub fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = &strings[0];
    let mut len = first.len();
    for s in &strings[1..] {
        len = len.min(s.len());
        for (i, (a, b)) in first.chars().zip(s.chars()).enumerate() {
            if a != b {
                len = len.min(i);
                break;
            }
        }
    }
    first[..len].to_string()
}

/// Simple line buffer with persistent working directory.
pub struct LineBuffer {
    pub lines: Vec<(String, [u8; 4])>,
    pub input: String,
    pub cwd: PathBuf,
    pub user: String,
    pub host: String,
    pub sigil: &'static str,
    /// Scroll offset from bottom (0 = pinned to bottom).
    pub scroll_offset: usize,
    /// Environment snapshot from interactive shell (captured once).
    env: Vec<(String, String)>,
}

impl LineBuffer {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let env = snapshot_env();
        Self {
            lines: vec![("Welcome to Detir Terminal".to_string(), [0, 255, 136, 255])],
            input: String::new(),
            cwd,
            user: username(),
            host: hostname(),
            sigil: if is_root() { "#" } else { "$" },
            scroll_offset: 0,
            env,
        }
    }

    pub fn prompt(&self) -> String {
        let dir = shorten_path(&self.cwd);
        format!("{}@{} {} {} ", self.user, self.host, dir, self.sigil)
    }

    pub fn push_char(&mut self, ch: char) {
        self.input.push(ch);
        self.scroll_offset = 0;
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn scroll_up(&mut self, lines: usize, max_visible: usize) {
        let total = self.lines.len() + 1;
        let max_offset = total.saturating_sub(max_visible);
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn commit(&mut self) {
        let prompt = self.prompt();
        let line = format!("{}{}", prompt, self.input);
        self.lines.push((line, [204, 204, 204, 255]));
        self.scroll_offset = 0;

        let cmd = self.input.trim().to_string();
        if cmd == "clear" {
            self.lines.clear();
        } else if !cmd.is_empty() {
            self.run_command(&cmd);
        }

        self.input.clear();
    }

    /// Tab-complete the last word in `self.input` as a path.
    pub fn tab_complete(&mut self) {
        // Find the last whitespace-delimited word.
        let last_space = self.input.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let partial = &self.input[last_space..];
        if partial.is_empty() {
            return;
        }

        // Expand ~ at the start of the partial.
        let (dir, prefix) = if partial == "~" || partial.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_default();
            let expanded = partial.replacen('~', &home, 1);
            let p = PathBuf::from(&expanded);
            if p.is_dir() {
                // User typed exactly a dir like "~/foo" — list its children.
                (p.clone(), expanded)
            } else {
                let parent = p.parent().unwrap_or(&self.cwd).to_path_buf();
                (parent, expanded)
            }
        } else {
            let p = self.cwd.join(partial);
            if p.is_dir() {
                (p, partial.to_string())
            } else {
                let parent = p.parent().unwrap_or(&self.cwd).to_path_buf();
                (parent, partial.to_string())
            }
        };

        // Read directory entries matching the prefix.
        let needle = PathBuf::from(&prefix);
        let needle_name = needle
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // If the partial is exactly a directory, list its contents for display.
        let search_dir = if PathBuf::from(&prefix).is_dir() || self.cwd.join(partial).is_dir() {
            if self.cwd.join(partial).is_dir() {
                self.cwd.join(partial)
            } else {
                PathBuf::from(&prefix)
            }
        } else {
            dir
        };

        let is_listing = search_dir.to_string_lossy().ends_with('/')
            || self.cwd.join(partial).is_dir()
            || partial.ends_with('/');

        let entries: Vec<String> = match std::fs::read_dir(&search_dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if is_listing || name.starts_with(&needle_name) {
                        let suffix = if e.path().is_dir() { "/" } else { "" };
                        Some(format!("{name}{suffix}"))
                    } else {
                        None
                    }
                })
                .collect(),
            Err(_) => return,
        };

        if entries.is_empty() {
            return;
        }

        if entries.len() == 1 {
            // Single match — complete it.
            let completion = &entries[0];
            if is_listing {
                // Append the entry to the partial.
                let sep = if partial.ends_with('/') { "" } else { "/" };
                let new_word = format!("{partial}{sep}{completion}");
                self.input.replace_range(last_space.., &new_word);
            } else {
                // Replace the filename portion.
                let base = needle
                    .parent()
                    .map(|p| {
                        let s = p.to_string_lossy().to_string();
                        if s.is_empty() { s } else { format!("{s}/") }
                    })
                    .unwrap_or_default();
                let new_word = format!("{base}{completion}");
                self.input.replace_range(last_space.., &new_word);
            }
        } else {
            // Multiple matches — find common prefix and show candidates.
            let common = longest_common_prefix(&entries);
            if common.len() > needle_name.len() && !is_listing {
                let base = needle
                    .parent()
                    .map(|p| {
                        let s = p.to_string_lossy().to_string();
                        if s.is_empty() { s } else { format!("{s}/") }
                    })
                    .unwrap_or_default();
                let new_word = format!("{base}{common}");
                self.input.replace_range(last_space.., &new_word);
            } else {
                // Show candidates as output.
                let listing = entries.join("  ");
                self.lines.push((listing, [150, 180, 255, 255]));
            }
        }
        self.scroll_offset = 0;
    }

    fn run_command(&mut self, cmd: &str) {
        // Handle cd as a built-in (child process cd can't affect parent).
        if cmd == "cd" || cmd.starts_with("cd ") {
            let target = cmd.strip_prefix("cd").unwrap().trim();
            let path = if target.is_empty() || target == "~" {
                std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| self.cwd.clone())
            } else if let Some(rest) = target.strip_prefix("~/") {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(rest))
                    .unwrap_or_else(|_| self.cwd.join(target))
            } else if target == "-" {
                // cd - not tracked (would need OLDPWD); just stay.
                self.lines.push((
                    "  cd -: OLDPWD not tracked yet".to_string(),
                    [255, 200, 50, 255],
                ));
                return;
            } else {
                self.cwd.join(target)
            };

            match path.canonicalize() {
                Ok(resolved) if resolved.is_dir() => {
                    self.cwd = resolved;
                }
                Ok(resolved) => {
                    self.lines.push((
                        format!("  cd: not a directory: {}", resolved.display()),
                        [255, 85, 85, 255],
                    ));
                }
                Err(e) => {
                    self.lines
                        .push((format!("  cd: {target}: {e}"), [255, 85, 85, 255]));
                }
            }
            return;
        }

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut child = std::process::Command::new(&shell);
        child.args(["-c", cmd]);
        child.current_dir(&self.cwd);
        child.stdin(std::process::Stdio::null());
        child.stdout(std::process::Stdio::piped());
        child.stderr(std::process::Stdio::piped());
        // Apply the env snapshot so commands see the same PATH, etc.
        // as the user's interactive shell without the startup cost.
        child.env_clear();
        for (k, v) in &self.env {
            child.env(k, v);
        }
        // Override PWD to match our tracked cwd.
        child.env("PWD", &self.cwd);
        match child.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stdout.lines() {
                    self.lines.push((line.to_string(), [204, 204, 204, 255]));
                }
                for line in stderr.lines() {
                    self.lines.push((line.to_string(), [255, 85, 85, 255]));
                }
            }
            Err(e) => {
                self.lines
                    .push((format!("  error: {e}"), [255, 85, 85, 255]));
            }
        }
    }
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::new()
    }
}
