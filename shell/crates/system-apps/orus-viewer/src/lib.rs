//! Orus OS Viewer — sandboxed gallery browser and previewer for images and PDFs.
//!
//! Scans a folder for image files (PNG, JPG, WebP, GIF, SVG) and PDFs, displays
//! them in a thumbnail grid (Gallery mode), and opens a full-size preview
//! on selection (Preview mode). A PDF shows one page at a time.

pub mod pdf;
mod render;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;

use jag_draw::TextProvider;

pub use render::TOOLBAR_HEIGHT;

/// File extensions the viewer opens: images and PDFs.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "svg", "pdf"];

/// Longest side, in pixels, of a PDF thumbnail's first page.
const THUMB_PX: u32 = 256;
/// Preview sizes are rounded up to a multiple of this, so a small resize reuses the
/// page already rendered.
const PREVIEW_STEP_PX: f32 = 256.0;
const PREVIEW_MIN_PX: u32 = 512;
const PREVIEW_MAX_PX: u32 = 3072;
/// Share of the preview's width, at each side, that turns the page when clicked.
const PAGE_TURN_ZONE: f32 = 0.25;

// ── Input types ────────────────────────────────────────────────

/// Input events the viewer handles.
#[derive(Debug, Clone)]
pub enum ViewerInput {
    Click { x: f32, y: f32 },
    KeyPress(ViewerKey),
    Scroll { dy: f32 },
    Resize { w: f32, h: f32 },
}

/// Keyboard events the viewer cares about.
#[derive(Debug, Clone)]
pub enum ViewerKey {
    Escape,
    Enter,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
}

// ── Internal types ─────────────────────────────────────────────

/// Current view mode.
#[derive(Debug, Clone)]
enum ViewMode {
    FolderPicker,
    Gallery,
    Preview { index: usize },
}

/// An entry in the folder picker (directory or image file).
#[derive(Debug, Clone)]
struct PickerEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

/// How a file is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Image,
    Svg,
    Pdf,
}

/// A single image or PDF discovered in the current folder.
#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: EntryKind,
}

/// One rendered PDF page: which file, which page, and the pixel size it was asked at.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RenderKey {
    path: PathBuf,
    page: usize,
    box_px: u32,
}

#[derive(Debug, Clone)]
enum RenderState {
    Pending,
    Ready(PathBuf),
    Failed(String),
}

struct RenderDone {
    key: RenderKey,
    result: Result<pdf::Rendered, String>,
}

/// Renders PDF pages one at a time on a thread of its own, until the sender is dropped.
fn spawn_renderer(
    renderer: &'static pdf::Renderer,
    done: mpsc::Sender<RenderDone>,
) -> mpsc::Sender<RenderKey> {
    let (jobs, queue) = mpsc::channel::<RenderKey>();
    std::thread::spawn(move || {
        let dir = pdf::cache_dir();
        for key in queue {
            let result = renderer.render_page(&key.path, key.page, key.box_px, &dir);
            if done.send(RenderDone { key, result }).is_err() {
                return;
            }
        }
    });
    jobs
}

// ── App state ──────────────────────────────────────────────────

/// The viewer application state.
pub struct ViewerApp {
    /// Root directory (sandbox boundary).
    root: PathBuf,
    /// Current directory being browsed.
    current_dir: PathBuf,
    /// Image entries discovered in `current_dir`.
    entries: Vec<ImageEntry>,
    /// Current view mode (gallery or single-image preview).
    mode: ViewMode,
    /// Selected thumbnail index in Gallery mode.
    selected: Option<usize>,
    /// Vertical scroll offset in pixels (Gallery mode).
    scroll_y: f32,
    /// Viewport width.
    viewport_w: f32,
    /// Viewport height.
    viewport_h: f32,
    /// Text provider for measuring and rendering text.
    pub provider: Arc<dyn TextProvider + Send + Sync>,
    /// Origin offset (content rect top-left in canvas coordinates).
    origin_x: f32,
    /// Origin offset (content rect top-left in canvas coordinates).
    origin_y: f32,
    /// Whether we're loading images from a folder (async scan in progress).
    loading: bool,
    /// Folder picker: directories in the current picker directory.
    picker_entries: Vec<PickerEntry>,
    /// Folder picker: current browsed directory.
    picker_dir: PathBuf,
    /// Folder picker: selected index.
    picker_selected: Option<usize>,
    /// Folder picker: scroll offset.
    picker_scroll_y: f32,
    /// Receiver for async folder scan results.
    scan_rx: Option<mpsc::Receiver<Vec<ImageEntry>>>,
    /// Pending file to preview once async scan completes.
    pending_file: Option<PathBuf>,
    /// PDF pages asked for, and what became of them.
    renders: HashMap<RenderKey, RenderState>,
    /// Pages in each PDF seen so far.
    page_counts: HashMap<PathBuf, usize>,
    /// The page of a PDF shown in Preview mode (0-based).
    preview_page: usize,
    render_done: mpsc::Receiver<RenderDone>,
    /// Thumbnails and the preview page render on separate threads, so a folder of PDFs
    /// does not hold up the page being read.
    thumb_jobs: mpsc::Sender<RenderKey>,
    page_jobs: mpsc::Sender<RenderKey>,
}

impl ViewerApp {
    /// Create a new viewer with no folder loaded.
    ///
    /// The shell should present a folder picker before the first render.
    pub fn new(root: PathBuf, provider: Arc<dyn TextProvider + Send + Sync>) -> Self {
        let picker_dir = root.clone();
        let picker_entries = scan_picker_entries(&picker_dir);
        let (done_tx, render_done) = mpsc::channel();
        let renderer = pdf::Renderer::shared();
        log::info!("PDF pages are drawn by {}", renderer.engine());
        Self {
            root,
            current_dir: PathBuf::new(),
            entries: Vec::new(),
            mode: ViewMode::FolderPicker,
            selected: None,
            scroll_y: 0.0,
            viewport_w: 800.0,
            viewport_h: 600.0,
            provider,
            origin_x: 0.0,
            origin_y: 0.0,
            loading: false,
            picker_entries,
            picker_dir,
            picker_selected: None,
            picker_scroll_y: 0.0,
            scan_rx: None,
            pending_file: None,
            renders: HashMap::new(),
            page_counts: HashMap::new(),
            preview_page: 0,
            render_done,
            thumb_jobs: spawn_renderer(renderer, done_tx.clone()),
            page_jobs: spawn_renderer(renderer, done_tx),
        }
    }

    /// Create an viewer opened directly to a folder.
    pub fn new_with_folder(
        root: PathBuf,
        folder: PathBuf,
        provider: Arc<dyn TextProvider + Send + Sync>,
    ) -> Self {
        let mut app = Self::new(root, provider);
        app.mode = ViewMode::Gallery;
        app.start_async_scan(folder);
        app
    }

    /// Create an viewer opened to a specific file.
    ///
    /// Opens the file's parent folder and switches to Preview mode
    /// for the given file. The scan runs async; preview mode is set
    /// once results arrive.
    pub fn new_with_file(
        root: PathBuf,
        file: PathBuf,
        provider: Arc<dyn TextProvider + Send + Sync>,
    ) -> Self {
        let folder = file
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root.clone());
        let mut app = Self::new(root, provider);
        app.mode = ViewMode::Gallery;
        app.pending_file = Some(file);
        app.start_async_scan(folder);
        app
    }

    /// Open a folder for browsing. The folder must be within the sandbox root.
    pub fn open_folder(&mut self, path: &Path) {
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => return,
        };
        let root_canonical = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        if !canonical.starts_with(&root_canonical) {
            return; // sandbox violation
        }
        self.mode = ViewMode::Gallery;
        self.selected = None;
        self.scroll_y = 0.0;
        self.start_async_scan(canonical);
    }

    /// Start an async folder scan on a background thread.
    fn start_async_scan(&mut self, folder: PathBuf) {
        self.current_dir = folder.clone();
        self.entries.clear();
        self.loading = true;
        let (tx, rx) = mpsc::channel();
        self.scan_rx = Some(rx);
        std::thread::spawn(move || {
            let results = scan_images(&folder);
            let _ = tx.send(results);
        });
    }

    /// Poll for finished folder scans and PDF pages, and ask for the pages now in view.
    /// Returns true if something arrived and the window needs redrawing.
    pub fn poll_scan(&mut self) -> bool {
        let scanned = self.poll_folder_scan();
        let rendered = self.collect_renders();
        self.request_renders();
        scanned || rendered
    }

    fn poll_folder_scan(&mut self) -> bool {
        let rx = match &self.scan_rx {
            Some(rx) => rx,
            None => return false,
        };
        match rx.try_recv() {
            Ok(entries) => {
                self.entries = entries;
                self.loading = false;
                self.scan_rx = None;

                // If opened with a specific file, switch to preview mode
                if let Some(file) = self.pending_file.take()
                    && let Some(idx) = self.entries.iter().position(|e| e.path == file)
                {
                    self.open_preview(idx);
                }
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.loading = false;
                self.scan_rx = None;
                true
            }
        }
    }

    /// Show the entry at `index` in Preview mode, from its first page.
    fn open_preview(&mut self, index: usize) {
        self.mode = ViewMode::Preview { index };
        self.selected = Some(index);
        self.preview_page = 0;
    }

    /// Pixels the preview page is rendered to fit: the longer side of the area it is
    /// drawn in, in steps.
    fn preview_box_px(&self) -> u32 {
        let area = (self.viewport_w - 2.0 * render::PREVIEW_PAD)
            .max(self.viewport_h - render::TOOLBAR_HEIGHT - 2.0 * render::PREVIEW_PAD);
        let stepped = (area / PREVIEW_STEP_PX).ceil() * PREVIEW_STEP_PX;
        (stepped as u32).clamp(PREVIEW_MIN_PX, PREVIEW_MAX_PX)
    }

    /// Take the PDF pages that finished since the last call.
    fn collect_renders(&mut self) -> bool {
        let mut changed = false;
        while let Ok(done) = self.render_done.try_recv() {
            let state = match done.result {
                Ok(rendered) => {
                    self.page_counts
                        .insert(done.key.path.clone(), rendered.page_count);
                    RenderState::Ready(rendered.png)
                }
                Err(message) => RenderState::Failed(message),
            };
            self.renders.insert(done.key, state);
            changed = true;
        }
        changed
    }

    /// Ask for the PDF pages on screen that are not rendered or on their way.
    fn request_renders(&mut self) {
        let mut wanted: Vec<(RenderKey, bool)> = Vec::new();
        match &self.mode {
            ViewMode::Gallery => {
                let cols = self.gallery_cols().max(1);
                let first_row = (self.scroll_y / render::CELL_SIZE) as usize;
                let rows_shown = ((self.viewport_h - render::TOOLBAR_HEIGHT) / render::CELL_SIZE)
                    .ceil() as usize
                    + 1;
                let range =
                    first_row * cols..((first_row + rows_shown) * cols).min(self.entries.len());
                for entry in self.entries.get(range).unwrap_or_default() {
                    if entry.kind == EntryKind::Pdf {
                        let key = RenderKey {
                            path: entry.path.clone(),
                            page: 0,
                            box_px: THUMB_PX,
                        };
                        wanted.push((key, false));
                    }
                }
            }
            ViewMode::Preview { index } => {
                if let Some(entry) = self.entries.get(*index)
                    && entry.kind == EntryKind::Pdf
                {
                    let key = RenderKey {
                        path: entry.path.clone(),
                        page: self.preview_page,
                        box_px: self.preview_box_px(),
                    };
                    wanted.push((key, true));
                }
            }
            ViewMode::FolderPicker => {}
        }
        for (key, is_page) in wanted {
            if self.renders.contains_key(&key) {
                continue;
            }
            let jobs = if is_page {
                &self.page_jobs
            } else {
                &self.thumb_jobs
            };
            if jobs.send(key.clone()).is_ok() {
                self.renders.insert(key, RenderState::Pending);
            }
        }
    }

    /// What became of page `page` of `path` at `box_px`; `None` before it was asked for.
    fn render_state(&self, path: &Path, page: usize, box_px: u32) -> Option<&RenderState> {
        self.renders.get(&RenderKey {
            path: path.to_path_buf(),
            page,
            box_px,
        })
    }

    /// Pages in the PDF at `path`, once one page of it has been rendered.
    fn page_count(&self, path: &Path) -> Option<usize> {
        self.page_counts.get(path).copied()
    }

    /// Update the cached viewport dimensions.
    pub fn update_viewport(&mut self, w: f32, h: f32) {
        self.viewport_w = w;
        self.viewport_h = h;
    }

    /// Set the origin offset for coordinate translation.
    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin_x = x;
        self.origin_y = y;
    }

    /// Handle an input event. Returns true if a redraw is needed.
    pub fn handle_input(&mut self, input: ViewerInput) -> bool {
        match input {
            ViewerInput::Click { x, y } => self.handle_click(x, y),
            ViewerInput::KeyPress(key) => self.handle_key(key),
            ViewerInput::Scroll { dy } => self.handle_scroll(dy),
            ViewerInput::Resize { w, h } => {
                self.update_viewport(w, h);
                true
            }
        }
    }

    // ── Click handling ─────────────────────────────────────────

    fn handle_click(&mut self, x: f32, y: f32) -> bool {
        match &self.mode {
            ViewMode::FolderPicker => self.handle_picker_click(x, y),
            ViewMode::Gallery => self.handle_gallery_click(x, y),
            ViewMode::Preview { .. } => self.handle_preview_click(x, y),
        }
    }

    fn handle_gallery_click(&mut self, x: f32, y: f32) -> bool {
        // Toolbar region
        if y < render::TOOLBAR_HEIGHT {
            // "Change Folder" button on the right side of the toolbar
            let btn_x = self.viewport_w - render::CHANGE_FOLDER_BTN_W - 8.0;
            if x >= btn_x && x < btn_x + render::CHANGE_FOLDER_BTN_W {
                self.enter_folder_picker();
                return true;
            }
            // Breadcrumb click — navigate picker to that folder
            let dir = self.current_dir.clone();
            if let Some(path) = self.breadcrumb_hit(x, &dir) {
                self.picker_navigate(path.clone());
                self.mode = ViewMode::FolderPicker;
                return true;
            }
            return false;
        }

        // Thumbnail grid
        let cols = self.gallery_cols();
        if cols == 0 {
            return false;
        }
        let grid_y = y - render::TOOLBAR_HEIGHT + self.scroll_y;
        let col = (x / render::CELL_SIZE) as usize;
        let row = (grid_y / render::CELL_SIZE) as usize;
        if col < cols {
            let index = row * cols + col;
            if index < self.entries.len() {
                self.open_preview(index);
                return true;
            }
        }
        false
    }

    fn handle_preview_click(&mut self, x: f32, y: f32) -> bool {
        // Back button in toolbar region ("← Gallery" label area)
        if y < render::TOOLBAR_HEIGHT && x < render::BACK_BTN_HIT_W {
            self.mode = ViewMode::Gallery;
            if let Some(sel) = self.selected {
                self.ensure_visible(sel);
            }
            return true;
        }
        // On a PDF the outer quarters of the page area turn the page.
        if let ViewMode::Preview { index } = self.mode
            && self
                .entries
                .get(index)
                .is_some_and(|e| e.kind == EntryKind::Pdf)
        {
            if x < self.viewport_w * PAGE_TURN_ZONE {
                return self.step_preview(index, -1);
            }
            if x > self.viewport_w * (1.0 - PAGE_TURN_ZONE) {
                return self.step_preview(index, 1);
            }
        }
        false
    }

    // ── Key handling ───────────────────────────────────────────

    fn handle_key(&mut self, key: ViewerKey) -> bool {
        match &self.mode {
            ViewMode::FolderPicker => self.handle_picker_key(key),
            ViewMode::Gallery => self.handle_gallery_key(key),
            ViewMode::Preview { index } => {
                let idx = *index;
                self.handle_preview_key(key, idx)
            }
        }
    }

    fn handle_gallery_key(&mut self, key: ViewerKey) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let cols = self.gallery_cols();
        if cols == 0 {
            return false;
        }

        match key {
            ViewerKey::ArrowRight => {
                let sel = self.selected.unwrap_or(0);
                let next = (sel + 1).min(self.entries.len() - 1);
                self.selected = Some(next);
                self.ensure_visible(next);
                true
            }
            ViewerKey::ArrowLeft => {
                let sel = self.selected.unwrap_or(0);
                let next = sel.saturating_sub(1);
                self.selected = Some(next);
                self.ensure_visible(next);
                true
            }
            ViewerKey::ArrowDown => {
                let sel = self.selected.unwrap_or(0);
                let next = (sel + cols).min(self.entries.len() - 1);
                self.selected = Some(next);
                self.ensure_visible(next);
                true
            }
            ViewerKey::ArrowUp => {
                let sel = self.selected.unwrap_or(0);
                let next = sel.saturating_sub(cols);
                self.selected = Some(next);
                self.ensure_visible(next);
                true
            }
            ViewerKey::Enter => {
                if let Some(sel) = self.selected
                    && sel < self.entries.len()
                {
                    self.open_preview(sel);
                    return true;
                }
                false
            }
            ViewerKey::Escape => false,
        }
    }

    /// Move the preview one page (a PDF) or one file; past the first or last page it
    /// moves to the neighbouring file.
    fn step_preview(&mut self, index: usize, delta: isize) -> bool {
        let pages = self
            .entries
            .get(index)
            .filter(|e| e.kind == EntryKind::Pdf)
            .map(|e| self.page_count(&e.path).unwrap_or(1));
        if let Some(count) = pages {
            let page = self.preview_page as isize + delta;
            if (0..count as isize).contains(&page) {
                self.preview_page = page as usize;
                return true;
            }
        }
        let next = index as isize + delta;
        if (0..self.entries.len() as isize).contains(&next) {
            self.open_preview(next as usize);
            return true;
        }
        false
    }

    fn handle_preview_key(&mut self, key: ViewerKey, index: usize) -> bool {
        match key {
            ViewerKey::Escape => {
                self.mode = ViewMode::Gallery;
                if let Some(sel) = self.selected {
                    self.ensure_visible(sel);
                }
                true
            }
            // In a PDF the arrows turn pages; past either end they move to the
            // neighbouring file.
            ViewerKey::ArrowLeft | ViewerKey::ArrowUp => self.step_preview(index, -1),
            ViewerKey::ArrowRight | ViewerKey::ArrowDown => self.step_preview(index, 1),
            _ => false,
        }
    }

    // ── Scroll handling ────────────────────────────────────────

    fn handle_scroll(&mut self, dy: f32) -> bool {
        match &self.mode {
            ViewMode::FolderPicker => {
                let content_h = self.picker_entries.len() as f32 * render::PICKER_ROW_HEIGHT;
                let visible_h = self.viewport_h - render::TOOLBAR_HEIGHT;
                let max_scroll = (content_h - visible_h).max(0.0);
                self.picker_scroll_y = (self.picker_scroll_y - dy * 30.0).clamp(0.0, max_scroll);
                true
            }
            ViewMode::Gallery => {
                let cols = self.gallery_cols().max(1);
                let rows = self.entries.len().div_ceil(cols);
                let content_h = rows as f32 * render::CELL_SIZE;
                let visible_h = self.viewport_h - render::TOOLBAR_HEIGHT;
                let max_scroll = (content_h - visible_h).max(0.0);
                self.scroll_y = (self.scroll_y - dy * 30.0).clamp(0.0, max_scroll);
                true
            }
            // The wheel turns the pages of a PDF, as it would scroll them.
            ViewMode::Preview { index } => {
                let index = *index;
                let is_pdf = self
                    .entries
                    .get(index)
                    .is_some_and(|e| e.kind == EntryKind::Pdf);
                is_pdf && dy != 0.0 && self.step_within_pdf(index, if dy > 0.0 { -1 } else { 1 })
            }
        }
    }

    /// Turn the page of the PDF in the preview; unlike `step_preview` it stops at the
    /// ends, so a long wheel spin does not run through the folder.
    fn step_within_pdf(&mut self, index: usize, delta: isize) -> bool {
        let count = self
            .entries
            .get(index)
            .and_then(|e| self.page_count(&e.path))
            .unwrap_or(1);
        let page = self.preview_page as isize + delta;
        if (0..count as isize).contains(&page) {
            self.preview_page = page as usize;
            return true;
        }
        false
    }

    // ── Folder picker ─────────────────────────────────────────

    fn enter_folder_picker(&mut self) {
        self.picker_dir = self.root.clone();
        self.picker_entries = scan_picker_entries(&self.picker_dir);
        self.picker_selected = None;
        self.picker_scroll_y = 0.0;
        self.mode = ViewMode::FolderPicker;
    }

    fn picker_navigate(&mut self, dir: PathBuf) {
        self.picker_dir = dir;
        self.picker_entries = scan_picker_entries(&self.picker_dir);
        self.picker_selected = None;
        self.picker_scroll_y = 0.0;
    }

    fn picker_go_up(&mut self) {
        if self.picker_dir == self.root {
            return;
        }
        if let Some(parent) = self.picker_dir.parent() {
            let parent = parent.to_path_buf();
            if parent.starts_with(&self.root) || parent == self.root {
                self.picker_navigate(parent);
            }
        }
    }

    fn picker_select_current(&mut self) {
        self.open_folder(&self.picker_dir.clone());
    }

    fn handle_picker_click(&mut self, x: f32, y: f32) -> bool {
        if y < render::TOOLBAR_HEIGHT {
            // "Open Here" button on the right
            let btn_x = self.viewport_w - render::PICKER_OPEN_BTN_W - 8.0;
            if x >= btn_x {
                self.picker_select_current();
                return true;
            }
            // Breadcrumb click
            let dir = self.picker_dir.clone();
            if let Some(path) = self.breadcrumb_hit(x, &dir) {
                self.picker_navigate(path);
                return true;
            }
            return false;
        }

        // Row click
        let list_y = y - render::TOOLBAR_HEIGHT + self.picker_scroll_y;
        let row = (list_y / render::PICKER_ROW_HEIGHT) as usize;
        if row < self.picker_entries.len() {
            let entry = &self.picker_entries[row];
            if entry.is_dir {
                if self.picker_selected == Some(row) {
                    // Second click on same dir: navigate into it
                    let path = entry.path.clone();
                    self.picker_navigate(path);
                } else {
                    self.picker_selected = Some(row);
                }
            } else {
                // Image file: open in preview
                let file = entry.path.clone();
                self.pending_file = Some(file);
                self.open_folder(&self.picker_dir.clone());
            }
            return true;
        }
        false
    }

    fn handle_picker_key(&mut self, key: ViewerKey) -> bool {
        match key {
            ViewerKey::ArrowDown => {
                let sel = self.picker_selected.map_or(0, |s| s + 1);
                if sel < self.picker_entries.len() {
                    self.picker_selected = Some(sel);
                    self.picker_ensure_visible(sel);
                }
                true
            }
            ViewerKey::ArrowUp => {
                if let Some(sel) = self.picker_selected {
                    let next = sel.saturating_sub(1);
                    self.picker_selected = Some(next);
                    self.picker_ensure_visible(next);
                }
                true
            }
            ViewerKey::Enter => {
                if let Some(sel) = self.picker_selected
                    && sel < self.picker_entries.len()
                {
                    let entry = &self.picker_entries[sel];
                    if entry.is_dir {
                        let path = entry.path.clone();
                        self.picker_navigate(path);
                    } else {
                        // Image file: open in preview
                        let file = entry.path.clone();
                        self.pending_file = Some(file);
                        self.open_folder(&self.picker_dir.clone());
                    }
                } else {
                    self.picker_select_current();
                }
                true
            }
            ViewerKey::Escape => {
                self.picker_go_up();
                true
            }
            ViewerKey::ArrowRight => {
                // Navigate into selected directory
                if let Some(sel) = self.picker_selected
                    && sel < self.picker_entries.len()
                    && self.picker_entries[sel].is_dir
                {
                    let path = self.picker_entries[sel].path.clone();
                    self.picker_navigate(path);
                    return true;
                }
                false
            }
            ViewerKey::ArrowLeft => {
                self.picker_go_up();
                true
            }
        }
    }

    fn picker_ensure_visible(&mut self, index: usize) {
        let row_top = index as f32 * render::PICKER_ROW_HEIGHT;
        let row_bottom = row_top + render::PICKER_ROW_HEIGHT;
        let visible_h = self.viewport_h - render::TOOLBAR_HEIGHT;
        if row_top < self.picker_scroll_y {
            self.picker_scroll_y = row_top;
        } else if row_bottom > self.picker_scroll_y + visible_h {
            self.picker_scroll_y = row_bottom - visible_h;
        }
    }

    // ── Helpers ────────────────────────────────────────────────

    /// Hit-test breadcrumbs and return the path if one was clicked.
    fn breadcrumb_hit(&self, x: f32, dir: &Path) -> Option<PathBuf> {
        let segments = self.breadcrumb_segments(dir);
        let mut cx = render::BREADCRUMB_PAD;

        for (i, (label, path)) in segments.iter().enumerate() {
            // Home icon on first segment
            if i == 0 {
                cx += 20.0; // 16px icon + 4px gap
            }
            let label_w = Self::approx_text_w(label, render::TOOLBAR_FONT_SIZE);
            if x >= cx && x < cx + label_w {
                return Some(path.clone());
            }
            cx += label_w;
            if i + 1 < segments.len() {
                cx += 20.0; // 6px gap + 10px chevron + 4px gap
            }
        }
        None
    }

    /// Approximate text width for hit testing (same approach as FileManager).
    fn approx_text_w(text: &str, size: f32) -> f32 {
        text.len() as f32 * size * 0.55
    }

    /// Build breadcrumb segments from root to the given directory.
    fn breadcrumb_segments(&self, dir: &Path) -> Vec<(String, PathBuf)> {
        let root_canonical = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        let current = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());

        let mut segments = vec![("Home".to_string(), root_canonical.clone())];

        if let Ok(relative) = current.strip_prefix(&root_canonical) {
            let mut path = root_canonical.clone();
            for component in relative.components() {
                path = path.join(component);
                segments.push((
                    component.as_os_str().to_string_lossy().into_owned(),
                    path.clone(),
                ));
            }
        }
        segments
    }

    /// Number of columns that fit in the current viewport width.
    fn gallery_cols(&self) -> usize {
        (self.viewport_w / render::CELL_SIZE).max(1.0) as usize
    }

    /// Scroll the gallery to ensure the given index is visible.
    fn ensure_visible(&mut self, index: usize) {
        let cols = self.gallery_cols().max(1);
        let row = index / cols;
        let row_top = row as f32 * render::CELL_SIZE;
        let row_bottom = row_top + render::CELL_SIZE;
        let visible_h = self.viewport_h - render::TOOLBAR_HEIGHT;
        if row_top < self.scroll_y {
            self.scroll_y = row_top;
        } else if row_bottom > self.scroll_y + visible_h {
            self.scroll_y = row_bottom - visible_h;
        }
    }
}

// ── Directory scanning ─────────────────────────────────────────

/// Scan a directory for subdirectories and image files, sorted with dirs first.
fn scan_picker_entries(dir: &Path) -> Vec<PickerEntry> {
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut entries: Vec<PickerEntry> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            if path.is_dir() {
                return Some(PickerEntry {
                    name,
                    path,
                    is_dir: true,
                });
            }
            // Include image files
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())?;
            if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                Some(PickerEntry {
                    name,
                    path,
                    is_dir: false,
                })
            } else {
                None
            }
        })
        .collect();

    // Directories first, then files, alphabetical within each group
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

/// Scan a directory for images and PDFs, returning them sorted alphabetically.
fn scan_images(dir: &Path) -> Vec<ImageEntry> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut entries: Vec<ImageEntry> = read_dir
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())?;
            if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                return None;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let kind = match ext.as_str() {
                "svg" => EntryKind::Svg,
                "pdf" => EntryKind::Pdf,
                _ => EntryKind::Image,
            };
            Some(ImageEntry { path, name, kind })
        })
        .collect();

    entries.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    entries
}
