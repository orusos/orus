//! What the file manager remembers: its tabs, each with its own place in the
//! folder tree and its own back and forward history.

use std::path::{Path, PathBuf};

/// A single directory entry for display.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Local>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Grid,
    List,
}

/// One tab: a folder, and where the tab has been.
#[derive(Debug, Clone)]
pub struct Tab {
    pub dir: PathBuf,
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
    pub selected: Option<usize>,
    pub scroll_y: f32,
}

impl Tab {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            back: Vec::new(),
            forward: Vec::new(),
            selected: None,
            scroll_y: 0.0,
        }
    }

    /// Go to `dir`, keeping the folder left behind for Back. A new place clears Forward.
    pub fn go(&mut self, dir: PathBuf) {
        if dir == self.dir {
            return;
        }
        self.back.push(std::mem::replace(&mut self.dir, dir));
        self.forward.clear();
        self.selected = None;
        self.scroll_y = 0.0;
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Step back; returns the folder now shown.
    pub fn go_back(&mut self) -> Option<&Path> {
        let previous = self.back.pop()?;
        self.forward
            .push(std::mem::replace(&mut self.dir, previous));
        self.selected = None;
        self.scroll_y = 0.0;
        Some(&self.dir)
    }

    /// Step forward; returns the folder now shown.
    pub fn go_forward(&mut self) -> Option<&Path> {
        let next = self.forward.pop()?;
        self.back.push(std::mem::replace(&mut self.dir, next));
        self.selected = None;
        self.scroll_y = 0.0;
        Some(&self.dir)
    }

    /// The tab's title: the folder's name, or "Home" for the home folder.
    pub fn title(&self, home: &Path) -> String {
        if self.dir == home {
            return "Home".to_string();
        }
        self.dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "System".to_string())
    }
}

/// Indexes of the entries whose name contains `query`, ignoring case. An empty
/// query keeps all of them.
pub fn filter_entries(entries: &[DirEntry], query: &str) -> Vec<usize> {
    let needle = query.to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.name.to_lowercase().contains(&needle))
        .map(|(i, _)| i)
        .collect()
}

/// The path bar's segments for `dir`: from "Home" when `dir` is under `home`,
/// otherwise from "System" (the root).
pub fn breadcrumbs(home: &Path, dir: &Path) -> Vec<(String, PathBuf)> {
    let (first, base) = match dir.strip_prefix(home) {
        Ok(_) => ("Home", home),
        Err(_) => ("System", Path::new("/")),
    };
    let mut path = base.to_path_buf();
    let mut segments = vec![(first.to_string(), path.clone())];
    if let Ok(rest) = dir.strip_prefix(base) {
        for part in rest.components() {
            path.push(part);
            segments.push((
                part.as_os_str().to_string_lossy().into_owned(),
                path.clone(),
            ));
        }
    }
    segments
}
