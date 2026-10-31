//! What the window does: move between folders and tabs, and change files.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::model::Tab;
use crate::{ContextAction, EditMode, FileManagerApp};

/// A name the user typed for a new or renamed entry: one path component.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/')
}

impl FileManagerApp {
    /// Show `dir` in the active tab. Does nothing if it is not a folder.
    pub(crate) fn navigate(&mut self, dir: &Path) {
        if !dir.is_dir() {
            return;
        }
        self.tab_mut().go(dir.to_path_buf());
        self.after_navigation();
    }

    pub(crate) fn go_back(&mut self) {
        if self.tab_mut().go_back().is_some() {
            self.after_navigation();
        }
    }

    pub(crate) fn go_forward(&mut self) {
        if self.tab_mut().go_forward().is_some() {
            self.after_navigation();
        }
    }

    pub(crate) fn go_up(&mut self) {
        if let Some(parent) = self.tab().dir.parent().map(Path::to_path_buf) {
            self.navigate(&parent);
        }
    }

    fn after_navigation(&mut self) {
        self.search = None;
        self.edit_mode = EditMode::None;
        self.context_menu = None;
        self.reload();
    }

    /// Open a new tab on the home folder and switch to it.
    pub(crate) fn new_tab(&mut self) {
        self.tabs.push(Tab::new(self.home.clone()));
        self.active = self.tabs.len() - 1;
        self.after_navigation();
    }

    pub(crate) fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() && index != self.active {
            self.active = index;
            self.after_navigation();
        }
    }

    /// Close a tab. The last tab stays open.
    pub(crate) fn close_tab(&mut self, index: usize) {
        if self.tabs.len() < 2 || index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.active > index || self.active >= self.tabs.len() {
            self.active = self.active.saturating_sub(1);
        }
        self.after_navigation();
    }

    pub(crate) fn open_entry(&mut self, index: usize) {
        if let Some(entry) = self.entries.get(index)
            && entry.is_dir
        {
            let target = self.tab().dir.join(&entry.name);
            self.navigate(&target);
        }
    }

    pub(crate) fn entry_path(&self, index: usize) -> Option<PathBuf> {
        self.entries
            .get(index)
            .map(|e| self.tab().dir.join(&e.name))
    }

    pub(crate) fn start_edit(&mut self, mode: EditMode) {
        self.edit_mode = mode;
        self.cursor_epoch = Instant::now();
    }

    pub(crate) fn run_context_action(&mut self, action: ContextAction, target: Option<usize>) {
        match (action, target) {
            (ContextAction::Rename, Some(index)) => {
                let buffer = self.entries[index].name.clone();
                self.start_edit(EditMode::Renaming { index, buffer });
            }
            (ContextAction::Delete, Some(index)) => {
                self.edit_mode = EditMode::ConfirmDelete { index };
            }
            (ContextAction::NewFolder, _) => self.start_edit(EditMode::CreatingFolder {
                buffer: String::new(),
            }),
            (ContextAction::NewFile, _) => self.start_edit(EditMode::CreatingFile {
                buffer: String::new(),
            }),
            _ => {}
        }
    }

    /// Keys while a name is being typed or a delete is being confirmed. Returns
    /// false if no such mode is active.
    pub(crate) fn edit_key(&mut self, key: &crate::FileManagerKey) -> bool {
        use crate::FileManagerKey as K;
        let mut mode = std::mem::replace(&mut self.edit_mode, EditMode::None);
        match (&mut mode, key) {
            (EditMode::None, _) => return false,
            (_, K::Escape) => return true,
            (EditMode::ConfirmDelete { index }, K::Enter) => self.delete(*index),
            (EditMode::Renaming { index, buffer }, K::Enter) => self.rename(*index, buffer),
            (EditMode::CreatingFile { buffer }, K::Enter) => self.create(buffer, false),
            (EditMode::CreatingFolder { buffer }, K::Enter) => self.create(buffer, true),
            (
                EditMode::Renaming { buffer, .. }
                | EditMode::CreatingFile { buffer }
                | EditMode::CreatingFolder { buffer },
                _,
            ) => {
                match key {
                    K::Char(c) => buffer.push(*c),
                    K::Backspace => {
                        buffer.pop();
                    }
                    _ => {}
                }
                self.cursor_epoch = Instant::now();
                self.edit_mode = mode;
            }
            (EditMode::ConfirmDelete { .. }, _) => self.edit_mode = mode,
        }
        true
    }

    fn rename(&mut self, index: usize, new_name: &str) {
        let Some(old) = self.entry_path(index) else {
            return;
        };
        let new = self.tab().dir.join(new_name);
        if valid_name(new_name) && !new.exists() {
            let _ = std::fs::rename(old, new);
        }
        self.reload();
    }

    fn create(&mut self, name: &str, folder: bool) {
        let path = self.tab().dir.join(name);
        if valid_name(name) && !path.exists() {
            let _ = if folder {
                std::fs::create_dir(&path)
            } else {
                std::fs::File::create(&path).map(drop)
            };
        }
        self.reload();
    }

    fn delete(&mut self, index: usize) {
        if let (Some(path), Some(entry)) = (self.entry_path(index), self.entries.get(index)) {
            let _ = if entry.is_dir {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
        }
        self.reload();
    }
}
