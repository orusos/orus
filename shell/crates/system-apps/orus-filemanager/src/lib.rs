//! Orus OS Files: tabs, a sidebar of places and devices, a path bar, and the
//! folder's contents as a grid or a list. Browses the whole disk from the home folder.

pub mod fsinfo;
pub mod icons;
pub mod layout;
mod listing;
pub mod model;
pub mod places;

mod input;
mod ops;
mod render;

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jag_draw::{Rect, TextProvider};

use model::{DirEntry, Tab, ViewMode};
use places::Places;

/// Input events the file manager handles.
#[derive(Debug, Clone)]
pub enum FileManagerInput {
    Click { x: f32, y: f32 },
    DoubleClick { x: f32, y: f32 },
    RightClick { x: f32, y: f32 },
    KeyPress(FileManagerKey),
    Scroll { dy: f32 },
    Resize { w: f32, h: f32 },
}

/// Keyboard events the file manager cares about.
#[derive(Debug, Clone)]
pub enum FileManagerKey {
    Escape,
    Enter,
    Delete,
    Backspace,
    F2,
    Char(char),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

/// Interaction mode for inline editing.
#[derive(Debug, Clone, PartialEq)]
enum EditMode {
    None,
    Renaming { index: usize, buffer: String },
    CreatingFile { buffer: String },
    CreatingFolder { buffer: String },
    ConfirmDelete { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextAction {
    Rename,
    Delete,
    NewFolder,
    NewFile,
}

/// Right-click menu: on an entry (Rename, Delete) or on empty space (New folder, New file).
#[derive(Debug, Clone)]
struct ContextMenu {
    x: f32,
    y: f32,
    target: Option<usize>,
}

/// How long to keep redrawing for thumbnails to appear. An image that takes longer
/// to decode shows once something else redraws the window.
const THUMBNAIL_WAIT: Duration = Duration::from_secs(3);

pub struct FileManagerApp {
    home: PathBuf,
    places: Places,
    tabs: Vec<Tab>,
    active: usize,
    view: ViewMode,
    /// The active tab's folder, sorted.
    entries: Vec<DirEntry>,
    /// Indexes into `entries` that the search lets through, in order.
    visible: Vec<usize>,
    /// The search text, while the path pill is in search mode.
    search: Option<String>,
    available: Option<u64>,
    /// Thumbnails decode in the background and the canvas does not say when they are
    /// done, so the window redraws until this time after the images shown change.
    thumbs_until: Option<Instant>,
    viewport_w: f32,
    viewport_h: f32,
    edit_mode: EditMode,
    context_menu: Option<ContextMenu>,
    /// Text provider for measuring.
    pub provider: Arc<dyn TextProvider + Send + Sync>,
    cursor_epoch: Instant,
    /// Content rect top-left in canvas coordinates.
    origin_x: f32,
    origin_y: f32,
    /// Where each path bar segment was last drawn, for clicks.
    crumb_hits: RefCell<Vec<(Rect, PathBuf)>>,
}

impl FileManagerApp {
    /// Open on `home`, which is also what the path bar calls "Home".
    pub fn new(home: PathBuf, provider: Arc<dyn TextProvider + Send + Sync>) -> Self {
        let mut app = Self {
            tabs: vec![Tab::new(home.clone())],
            places: Places::default(),
            home,
            active: 0,
            view: ViewMode::Grid,
            entries: Vec::new(),
            visible: Vec::new(),
            search: None,
            available: None,
            thumbs_until: None,
            viewport_w: 910.0,
            viewport_h: 570.0,
            edit_mode: EditMode::None,
            context_menu: None,
            provider,
            cursor_epoch: Instant::now(),
            origin_x: 0.0,
            origin_y: 0.0,
            crumb_hits: RefCell::new(Vec::new()),
        };
        app.reload();
        app
    }

    pub fn update_viewport(&mut self, w: f32, h: f32) {
        self.viewport_w = w;
        self.viewport_h = h;
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin_x = x;
        self.origin_y = y;
    }

    /// The folder the active tab shows.
    pub fn current_dir(&self) -> &std::path::Path {
        &self.tab().dir
    }

    /// Names of the entries shown, in order (after the search).
    pub fn visible_names(&self) -> Vec<&str> {
        self.visible
            .iter()
            .map(|&i| self.entries[i].name.as_str())
            .collect()
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.tab().selected.map(|i| self.entries[i].name.as_str())
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    /// Read the active tab's folder again: entries, search, devices, free space.
    fn reload(&mut self) {
        let dir = self.tab().dir.clone();
        self.entries = listing::read_dir_sorted(&dir);
        self.apply_search();
        self.available = fsinfo::available_bytes(&dir);
        let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
        self.places = places::discover(&self.home, &mounts);
        let count = self.entries.len();
        if self.tab().selected.is_some_and(|s| s >= count) {
            self.tab_mut().selected = None;
        }
    }

    fn apply_search(&mut self) {
        self.visible = match &self.search {
            Some(query) => model::filter_entries(&self.entries, query),
            None => (0..self.entries.len()).collect(),
        };
        let has_images = self
            .visible
            .iter()
            .any(|&i| icons::is_image(&self.entries[i].name));
        self.thumbs_until = has_images.then(|| Instant::now() + THUMBNAIL_WAIT);
    }

    /// Whether the window needs redrawing continuously (the text cursor blinks).
    pub fn needs_redraw(&self) -> bool {
        self.search.is_some()
            || self.thumbs_until.is_some_and(|t| Instant::now() < t)
            || matches!(
                self.edit_mode,
                EditMode::Renaming { .. }
                    | EditMode::CreatingFile { .. }
                    | EditMode::CreatingFolder { .. }
            )
    }
}
