//! Follows `appearance.json`: reloads it when another program (System Settings,
//! `orus-appearance`) changes it.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use orus_core::appearance::{self, Appearance};

pub struct AppearanceWatch {
    dir: PathBuf,
    modified: Option<SystemTime>,
    pub current: Appearance,
}

fn modified(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir.join("appearance.json"))
        .and_then(|m| m.modified())
        .ok()
}

impl AppearanceWatch {
    /// Load the setting from `dir`; an unreadable file starts as the default
    /// appearance, with the reason logged.
    pub fn new(dir: &Path) -> Self {
        let current = appearance::load(dir).unwrap_or_else(|e| {
            log::warn!("{e}; using the default appearance");
            Appearance::default()
        });
        Self {
            dir: dir.to_path_buf(),
            modified: modified(dir),
            current,
        }
    }

    /// Reload if the file changed since the last look. Returns true when the
    /// appearance in use is now different. A file that does not parse keeps the
    /// previous appearance, and is logged once.
    pub fn poll(&mut self) -> bool {
        let now = modified(&self.dir);
        if now == self.modified {
            return false;
        }
        self.modified = now;
        match appearance::load(&self.dir) {
            Ok(next) if next != self.current => {
                self.current = next;
                true
            }
            Ok(_) => false,
            Err(e) => {
                log::warn!("{e}; keeping the current appearance");
                false
            }
        }
    }
}
