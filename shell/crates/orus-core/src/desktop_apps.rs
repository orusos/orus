//! Programs installed with the system's package manager, from their `.desktop` files.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A program the launcher can start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopApp {
    pub name: String,
    /// The command and its arguments, with the `%f`-style field codes removed.
    pub argv: Vec<String>,
}

/// The app a `.desktop` file describes, or `None` for one the launcher should not show:
/// not an application, hidden, run in a terminal, or restricted to other desktops.
pub fn parse(text: &str) -> Option<DesktopApp> {
    let (mut name, mut exec, mut try_exec) = (None, None, None);
    let mut in_entry = false;
    for line in text.lines().map(str::trim) {
        if let Some(section) = line.strip_prefix('[') {
            in_entry = section == "Desktop Entry]";
            continue;
        }
        let Some((key, value)) = line.split_once('=').filter(|_| in_entry) else {
            continue;
        };
        match key.trim() {
            "Type" if value != "Application" => return None,
            "NoDisplay" | "Hidden" | "Terminal" if value == "true" => return None,
            "OnlyShowIn" => return None,
            "Name" => name = Some(value.trim().to_string()),
            "Exec" => exec = Some(value.to_string()),
            "TryExec" => try_exec = Some(value.trim().to_string()),
            _ => {}
        }
    }
    if try_exec.is_some_and(|program| !on_path(&program)) {
        return None;
    }
    let argv = split_exec(&exec?);
    (!argv.is_empty()).then_some(DesktopApp {
        name: name.filter(|n| !n.is_empty())?,
        argv,
    })
}

/// Split an `Exec` value into words, honoring double quotes, and drop the field codes
/// (`%f`, `%U`, ...). `%%` is a percent sign.
fn split_exec(exec: &str) -> Vec<String> {
    let mut words = Vec::new();
    let (mut word, mut started, mut quoted) = (String::new(), false, false);
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted => {
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            '%' => match chars.next() {
                Some('%') => word.push('%'),
                Some(_) | None => {}
            },
            c if c.is_whitespace() && !quoted => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words.retain(|w| !w.is_empty());
    words
}

fn on_path(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

/// The `applications` folders to read, the user's first so it wins over the system's.
pub fn default_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    home.into_iter()
        .chain(system.split(':').map(PathBuf::from))
        .map(|d| d.join("applications"))
        .collect()
}

/// The apps in `dirs`, by name. A file name seen in an earlier folder hides the same name
/// in a later one, and `skip` drops programs the launcher already has a tile for.
pub fn list(dirs: &[PathBuf], skip: impl Fn(&DesktopApp) -> bool) -> Vec<DesktopApp> {
    let mut seen = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "desktop") {
                continue;
            }
            let id = entry.file_name();
            if seen.contains_key(&id) {
                continue;
            }
            seen.insert(id, fs::read_to_string(&path).ok().and_then(|t| parse(&t)));
        }
    }
    let mut apps: Vec<DesktopApp> = seen.into_values().flatten().filter(|a| !skip(a)).collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}
