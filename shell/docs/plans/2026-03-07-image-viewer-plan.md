# Image Viewer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a built-in Orus OS Image Viewer app that displays images from folders under `~/.orus/`, with gallery and preview modes, supporting both interactive folder selection and programmatic file/folder opening.

**Architecture:** Standalone crate `orus-imageviewer` following the exact same pattern as `orus-filemanager`. Renders via Canvas API (`draw_image`, `draw_svg_styled`). Wired into the shell through `WindowContent::ImageViewer`, `AppId::ImageViewer`, and dedicated `AppInput` variants.

**Tech Stack:** Rust, engine-core (Canvas, TextProvider), detir-surface (draw_image, ImageFitMode), rfd (folder picker)

---

## Task 1: Create `orus-imageviewer` crate with types and folder scanning

**Files:**
- Create: `orus-universal/crates/system-apps/orus-imageviewer/Cargo.toml`
- Create: `orus-universal/crates/system-apps/orus-imageviewer/src/lib.rs`
- Modify: `orus-universal/Cargo.toml` (workspace members + dependency)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "orus-imageviewer"
edition.workspace = true
version.workspace = true

[dependencies]
engine-core = { workspace = true }
detir-surface = { workspace = true }
log = { workspace = true }
```

**Step 2: Add to workspace**

In `orus-universal/Cargo.toml`:
- Add `"crates/system-apps/orus-imageviewer"` to `[workspace] members`
- Add `orus-imageviewer = { path = "crates/system-apps/orus-imageviewer" }` to `[workspace.dependencies]`

**Step 3: Write `src/lib.rs` — core types, folder scanning, and state**

```rust
//! Orus OS Image Viewer — gallery and preview for images under `~/.orus/`.

mod render;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine_core::TextProvider;

pub use render::TOOLBAR_HEIGHT;

/// Supported image extensions.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "svg"];

/// Input events the image viewer handles.
#[derive(Debug, Clone)]
pub enum ImageViewerInput {
    Click { x: f32, y: f32 },
    KeyPress(ImageViewerKey),
    Scroll { dy: f32 },
    Resize { w: f32, h: f32 },
}

/// Keyboard events.
#[derive(Debug, Clone)]
pub enum ImageViewerKey {
    Escape,
    Enter,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
}

/// Display mode.
#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    /// Grid of thumbnails.
    Gallery,
    /// Full-size single image.
    Preview { index: usize },
}

/// A single image entry.
#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_svg: bool,
}

/// The image viewer application state.
pub struct ImageViewerApp {
    /// Sandbox root (must be ancestor of all accessed paths).
    root: PathBuf,
    /// Current folder being viewed.
    current_dir: PathBuf,
    /// Image entries in the folder.
    entries: Vec<ImageEntry>,
    /// Current view mode.
    mode: ViewMode,
    /// Selected thumbnail index in gallery mode.
    selected: Option<usize>,
    /// Scroll offset in pixels (gallery mode).
    scroll_y: f32,
    /// Viewport dimensions.
    viewport_w: f32,
    viewport_h: f32,
    /// Text provider for rendering.
    pub provider: Arc<dyn TextProvider + Send + Sync>,
    /// Origin offset (content rect top-left).
    origin_x: f32,
    origin_y: f32,
    /// Whether a folder picker should be shown on next main-thread render.
    pub needs_folder_picker: bool,
}

impl ImageViewerApp {
    /// Create with no folder — will trigger folder picker on launch.
    pub fn new(root: PathBuf, provider: Arc<dyn TextProvider + Send + Sync>) -> Self {
        Self {
            root,
            current_dir: PathBuf::new(),
            entries: Vec::new(),
            mode: ViewMode::Gallery,
            selected: None,
            scroll_y: 0.0,
            viewport_w: 800.0,
            viewport_h: 600.0,
            provider,
            origin_x: 0.0,
            origin_y: 0.0,
            needs_folder_picker: true,
        }
    }

    /// Create opened to a specific folder.
    pub fn new_with_folder(
        root: PathBuf,
        folder: PathBuf,
        provider: Arc<dyn TextProvider + Send + Sync>,
    ) -> Self {
        let mut app = Self::new(root, provider);
        app.needs_folder_picker = false;
        app.open_folder(&folder);
        app
    }

    /// Create opened to a specific file (preview mode).
    pub fn new_with_file(
        root: PathBuf,
        file: PathBuf,
        provider: Arc<dyn TextProvider + Send + Sync>,
    ) -> Self {
        let folder = file.parent().unwrap_or(&file).to_path_buf();
        let mut app = Self::new(root, provider);
        app.needs_folder_picker = false;
        app.open_folder(&folder);
        // Find the file in entries and switch to preview
        if let Some(idx) = app.entries.iter().position(|e| e.path == file) {
            app.mode = ViewMode::Preview { index: idx };
        }
        app
    }

    /// Open a folder and scan for images. Validates path is under root.
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
        self.current_dir = canonical;
        self.entries = scan_images(&self.current_dir);
        self.selected = if self.entries.is_empty() { None } else { Some(0) };
        self.scroll_y = 0.0;
        self.mode = ViewMode::Gallery;
    }

    pub fn update_viewport(&mut self, w: f32, h: f32) {
        self.viewport_w = w;
        self.viewport_h = h;
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin_x = x;
        self.origin_y = y;
    }

    /// Handle an input event. Returns true if a redraw is needed.
    pub fn handle_input(&mut self, input: ImageViewerInput) -> bool {
        match input {
            ImageViewerInput::Click { x, y } => self.handle_click(x, y),
            ImageViewerInput::KeyPress(key) => self.handle_key(key),
            ImageViewerInput::Scroll { dy } => self.handle_scroll(dy),
            ImageViewerInput::Resize { w, h } => {
                self.update_viewport(w, h);
                true
            }
        }
    }

    fn handle_click(&mut self, x: f32, y: f32) -> bool {
        match self.mode {
            ViewMode::Gallery => {
                // Check toolbar: "Change Folder" button (right side)
                if y < render::TOOLBAR_HEIGHT {
                    let btn_w = render::CHANGE_FOLDER_BTN_W;
                    let btn_x = self.viewport_w - btn_w - 12.0;
                    if x >= btn_x && x < btn_x + btn_w {
                        self.needs_folder_picker = true;
                        return true;
                    }
                    return false;
                }

                // Check thumbnail grid
                let grid_y = y - render::TOOLBAR_HEIGHT + self.scroll_y;
                let cols = self.gallery_cols();
                let col = (x / render::CELL_SIZE) as usize;
                let row = (grid_y / render::CELL_SIZE) as usize;
                if col < cols {
                    let idx = row * cols + col;
                    if idx < self.entries.len() {
                        self.mode = ViewMode::Preview { index: idx };
                        return true;
                    }
                }
                false
            }
            ViewMode::Preview { .. } => {
                // Back button in toolbar
                if y < render::TOOLBAR_HEIGHT && x < 60.0 {
                    self.mode = ViewMode::Gallery;
                    return true;
                }
                false
            }
        }
    }

    fn handle_key(&mut self, key: ImageViewerKey) -> bool {
        match &self.mode {
            ViewMode::Gallery => {
                let cols = self.gallery_cols();
                match key {
                    ImageViewerKey::ArrowRight => {
                        if let Some(sel) = self.selected {
                            if sel + 1 < self.entries.len() {
                                self.selected = Some(sel + 1);
                                self.ensure_visible(sel + 1);
                                return true;
                            }
                        } else if !self.entries.is_empty() {
                            self.selected = Some(0);
                            return true;
                        }
                        false
                    }
                    ImageViewerKey::ArrowLeft => {
                        if let Some(sel) = self.selected {
                            if sel > 0 {
                                self.selected = Some(sel - 1);
                                self.ensure_visible(sel - 1);
                                return true;
                            }
                        }
                        false
                    }
                    ImageViewerKey::ArrowDown => {
                        if let Some(sel) = self.selected {
                            let next = sel + cols;
                            if next < self.entries.len() {
                                self.selected = Some(next);
                                self.ensure_visible(next);
                                return true;
                            }
                        }
                        false
                    }
                    ImageViewerKey::ArrowUp => {
                        if let Some(sel) = self.selected {
                            if sel >= cols {
                                self.selected = Some(sel - cols);
                                self.ensure_visible(sel - cols);
                                return true;
                            }
                        }
                        false
                    }
                    ImageViewerKey::Enter => {
                        if let Some(sel) = self.selected {
                            if sel < self.entries.len() {
                                self.mode = ViewMode::Preview { index: sel };
                                return true;
                            }
                        }
                        false
                    }
                    ImageViewerKey::Escape => false,
                }
            }
            ViewMode::Preview { index } => {
                let idx = *index;
                match key {
                    ImageViewerKey::Escape => {
                        self.selected = Some(idx);
                        self.mode = ViewMode::Gallery;
                        true
                    }
                    ImageViewerKey::ArrowRight => {
                        if idx + 1 < self.entries.len() {
                            self.mode = ViewMode::Preview { index: idx + 1 };
                            return true;
                        }
                        false
                    }
                    ImageViewerKey::ArrowLeft => {
                        if idx > 0 {
                            self.mode = ViewMode::Preview { index: idx - 1 };
                            return true;
                        }
                        false
                    }
                    _ => false,
                }
            }
        }
    }

    fn handle_scroll(&mut self, dy: f32) -> bool {
        if self.mode != ViewMode::Gallery {
            return false;
        }
        let cols = self.gallery_cols();
        let rows = (self.entries.len() + cols - 1) / cols;
        let content_h = rows as f32 * render::CELL_SIZE;
        let visible_h = self.viewport_h - render::TOOLBAR_HEIGHT;
        let max_scroll = (content_h - visible_h).max(0.0);
        self.scroll_y = (self.scroll_y - dy * 30.0).clamp(0.0, max_scroll);
        true
    }

    fn gallery_cols(&self) -> usize {
        (self.viewport_w / render::CELL_SIZE).max(1.0) as usize
    }

    fn ensure_visible(&mut self, index: usize) {
        let cols = self.gallery_cols();
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

/// Scan a directory for image files, sorted alphabetically.
fn scan_images(dir: &Path) -> Vec<ImageEntry> {
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut entries: Vec<ImageEntry> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            let ext = path.extension()?.to_str()?.to_lowercase();
            if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                return None;
            }
            let name = path.file_name()?.to_string_lossy().into_owned();
            let is_svg = ext == "svg";
            Some(ImageEntry { path, name, is_svg })
        })
        .collect();

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries
}
```

**Step 4: Create stub `src/render.rs`**

```rust
//! Rendering for the image viewer — toolbar, gallery grid, and preview mode.

use crate::{ImageViewerApp, ViewMode};
use engine_core::{Brush, ColorLinPremul, FontStyle, Rect, RoundedRadii, RoundedRect};

pub const TOOLBAR_HEIGHT: f32 = 44.0;
pub const CELL_SIZE: f32 = 140.0;
pub const THUMB_SIZE: f32 = 120.0;
pub const THUMB_PADDING: f32 = (CELL_SIZE - THUMB_SIZE) / 2.0;
pub const CHANGE_FOLDER_BTN_W: f32 = 100.0;
pub const LABEL_FONT_SIZE: f32 = 11.0;

impl ImageViewerApp {
    pub fn render(&self, canvas: &mut detir_surface::Canvas, w: f32, h: f32, z_base: i32) {
        // Stub — will be implemented in Task 2
        let _ = (canvas, w, h, z_base);
    }
}
```

**Step 5: Verify build**

Run: `cd orus-universal && cargo build -p orus-imageviewer`
Expected: Compiles without errors.

**Step 6: Commit**

```bash
git add orus-universal/crates/system-apps/orus-imageviewer/ orus-universal/Cargo.toml
git commit -m "feat(imageviewer): create orus-imageviewer crate with types and folder scanning"
```

---

## Task 2: Implement rendering — gallery grid and preview mode

**Files:**
- Modify: `orus-universal/crates/system-apps/orus-imageviewer/src/render.rs`

**Step 1: Implement the full renderer**

Replace the stub `render.rs` with the complete implementation. Follow the FileManager's rendering pattern exactly (colors, transform, clip, z-ordering).

```rust
//! Rendering for the image viewer — toolbar, gallery grid, and preview mode.
//! macOS-inspired dark theme matching FileManager.

use crate::{ImageViewerApp, ViewMode};
use engine_core::{
    Brush, ColorLinPremul, FontStyle, Rect, RoundedRadii, RoundedRect, Transform2D,
};

// ── Layout constants ────────────────────────────────────────────
pub const TOOLBAR_HEIGHT: f32 = 44.0;
pub const CELL_SIZE: f32 = 140.0;
pub const THUMB_SIZE: f32 = 120.0;
pub const THUMB_PADDING: f32 = (CELL_SIZE - THUMB_SIZE) / 2.0;
pub const CHANGE_FOLDER_BTN_W: f32 = 100.0;
pub const LABEL_FONT_SIZE: f32 = 11.0;
pub const LABEL_HEIGHT: f32 = 16.0;
pub const THUMB_CORNER: f32 = 6.0;

// ── Colors (matching FileManager dark theme) ────────────────────
const BG: ColorLinPremul = ColorLinPremul {
    r: 0.0110, g: 0.0110, b: 0.0130, a: 1.0,
};
const TOOLBAR_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0150, g: 0.0150, b: 0.0170, a: 1.0,
};
const SEPARATOR: ColorLinPremul = ColorLinPremul {
    r: 0.0200, g: 0.0200, b: 0.0230, a: 1.0,
};
const TEXT_PRIMARY: ColorLinPremul = ColorLinPremul {
    r: 0.8500, g: 0.8500, b: 0.8700, a: 1.0,
};
const TEXT_SECONDARY: ColorLinPremul = ColorLinPremul {
    r: 0.2000, g: 0.2000, b: 0.2200, a: 1.0,
};
const SELECTED_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0180, g: 0.0450, b: 0.1700, a: 1.0,
};
const THUMB_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0170, g: 0.0170, b: 0.0190, a: 1.0,
};
const BTN_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0200, g: 0.0200, b: 0.0230, a: 1.0,
};
const PREVIEW_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0050, g: 0.0050, b: 0.0060, a: 1.0,
};

fn rounded_radii(r: f32) -> RoundedRadii {
    RoundedRadii { tl: r, tr: r, br: r, bl: r }
}

// ── Rendering ───────────────────────────────────────────────────

impl ImageViewerApp {
    /// Render the image viewer into the canvas.
    pub fn render(&self, canvas: &mut detir_surface::Canvas, w: f32, h: f32, z_base: i32) {
        canvas.push_transform(Transform2D::translate(self.origin_x, self.origin_y));

        // Background
        canvas.fill_rect(0.0, 0.0, w, h, Brush::Solid(BG), z_base);

        match &self.mode {
            ViewMode::Gallery => {
                self.render_gallery_toolbar(canvas, w, z_base + 10);
                self.render_gallery_grid(canvas, w, h, z_base + 30);
            }
            ViewMode::Preview { index } => {
                self.render_preview_toolbar(canvas, w, *index, z_base + 10);
                self.render_preview_image(canvas, w, h, *index, z_base + 5);
            }
        }

        canvas.pop_transform();
    }

    // ── Gallery toolbar ─────────────────────────────────────────

    fn render_gallery_toolbar(&self, canvas: &mut detir_surface::Canvas, w: f32, z: i32) {
        canvas.fill_rect(0.0, 0.0, w, TOOLBAR_HEIGHT, Brush::Solid(TOOLBAR_BG), z);
        canvas.fill_rect(0.0, TOOLBAR_HEIGHT - 1.0, w, 1.0, Brush::Solid(SEPARATOR), z + 1);

        // Folder path (left)
        let path_label = if self.current_dir.as_os_str().is_empty() {
            "No folder selected".to_string()
        } else {
            self.current_dir.display().to_string()
        };
        let text_y = (TOOLBAR_HEIGHT + 13.0) / 2.0 - 1.0;
        canvas.draw_text_run_styled(
            [12.0, text_y],
            path_label,
            13.0, 500.0, FontStyle::Normal, None,
            TEXT_PRIMARY,
            z + 2,
        );

        // "Change Folder" button (right)
        let btn_x = w - CHANGE_FOLDER_BTN_W - 12.0;
        let btn_y = (TOOLBAR_HEIGHT - 28.0) / 2.0;
        canvas.rounded_rect(
            RoundedRect {
                rect: Rect { x: btn_x, y: btn_y, w: CHANGE_FOLDER_BTN_W, h: 28.0 },
                radii: rounded_radii(6.0),
            },
            Brush::Solid(BTN_BG),
            z + 2,
        );
        let label = "Open Folder";
        let label_w = canvas.measure_text_width(label, 12.0);
        canvas.draw_text_run_styled(
            [btn_x + (CHANGE_FOLDER_BTN_W - label_w) / 2.0, btn_y + 18.0],
            label.to_string(),
            12.0, 500.0, FontStyle::Normal, None,
            TEXT_PRIMARY,
            z + 3,
        );

        // Image count
        let count = format!("{} images", self.entries.len());
        let count_w = canvas.measure_text_width(&count, 11.0);
        canvas.draw_text_run_styled(
            [btn_x - count_w - 16.0, text_y],
            count,
            11.0, 400.0, FontStyle::Normal, None,
            TEXT_SECONDARY,
            z + 2,
        );
    }

    // ── Gallery grid ────────────────────────────────────────────

    fn render_gallery_grid(
        &self,
        canvas: &mut detir_surface::Canvas,
        w: f32,
        h: f32,
        z: i32,
    ) {
        let grid_top = TOOLBAR_HEIGHT;
        let grid_h = h - grid_top;

        canvas.push_clip_rect(Rect { x: 0.0, y: grid_top, w, h: grid_h });

        if self.entries.is_empty() {
            let msg = if self.current_dir.as_os_str().is_empty() {
                "Select a folder to view images"
            } else {
                "No images found in this folder"
            };
            let msg_w = canvas.measure_text_width(msg, 14.0);
            canvas.draw_text_run_styled(
                [(w - msg_w) / 2.0, grid_top + 100.0],
                msg.to_string(),
                14.0, 400.0, FontStyle::Normal, None,
                TEXT_SECONDARY,
                z,
            );
            canvas.pop_clip();
            return;
        }

        let cols = self.gallery_cols();

        for (i, entry) in self.entries.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;

            let cell_x = col as f32 * CELL_SIZE;
            let cell_y = grid_top + row as f32 * CELL_SIZE - self.scroll_y;

            // Skip cells outside visible area
            if cell_y + CELL_SIZE < grid_top || cell_y > grid_top + grid_h {
                continue;
            }

            let thumb_x = cell_x + THUMB_PADDING;
            let thumb_y = cell_y + THUMB_PADDING / 2.0;

            // Selection highlight
            if self.selected == Some(i) {
                canvas.rounded_rect(
                    RoundedRect {
                        rect: Rect {
                            x: cell_x + 4.0,
                            y: cell_y + 2.0,
                            w: CELL_SIZE - 8.0,
                            h: CELL_SIZE - 4.0,
                        },
                        radii: rounded_radii(8.0),
                    },
                    Brush::Solid(SELECTED_BG),
                    z,
                );
            }

            // Thumbnail background (placeholder while image loads)
            canvas.rounded_rect(
                RoundedRect {
                    rect: Rect { x: thumb_x, y: thumb_y, w: THUMB_SIZE, h: THUMB_SIZE },
                    radii: rounded_radii(THUMB_CORNER),
                },
                Brush::Solid(THUMB_BG),
                z + 1,
            );

            // Draw the image thumbnail
            if entry.is_svg {
                canvas.draw_svg(
                    &entry.path,
                    [thumb_x, thumb_y],
                    [THUMB_SIZE, THUMB_SIZE],
                    z + 2,
                );
            } else {
                canvas.draw_image(
                    &entry.path,
                    [thumb_x, thumb_y],
                    [THUMB_SIZE, THUMB_SIZE],
                    detir_surface::ImageFitMode::Contain,
                    z + 2,
                );
            }

            // Filename label (truncated if too long)
            let label = if entry.name.len() > 16 {
                format!("{}\u{2026}", &entry.name[..15])
            } else {
                entry.name.clone()
            };
            let label_w = canvas.measure_text_width(&label, LABEL_FONT_SIZE);
            let label_x = cell_x + (CELL_SIZE - label_w) / 2.0;
            let label_y = thumb_y + THUMB_SIZE + LABEL_HEIGHT;
            canvas.draw_text_run_styled(
                [label_x, label_y],
                label,
                LABEL_FONT_SIZE, 400.0, FontStyle::Normal, None,
                TEXT_PRIMARY,
                z + 2,
            );
        }

        canvas.pop_clip();
    }

    // ── Preview toolbar ─────────────────────────────────────────

    fn render_preview_toolbar(
        &self,
        canvas: &mut detir_surface::Canvas,
        w: f32,
        index: usize,
        z: i32,
    ) {
        canvas.fill_rect(0.0, 0.0, w, TOOLBAR_HEIGHT, Brush::Solid(TOOLBAR_BG), z);
        canvas.fill_rect(0.0, TOOLBAR_HEIGHT - 1.0, w, 1.0, Brush::Solid(SEPARATOR), z + 1);

        // Back button: "< Gallery"
        let back_label = "\u{2190} Gallery";
        canvas.draw_text_run_styled(
            [12.0, (TOOLBAR_HEIGHT + 13.0) / 2.0 - 1.0],
            back_label.to_string(),
            13.0, 500.0, FontStyle::Normal, None,
            TEXT_PRIMARY,
            z + 2,
        );

        // Filename (centered)
        if let Some(entry) = self.entries.get(index) {
            let name_w = canvas.measure_text_width(&entry.name, 13.0);
            canvas.draw_text_run_styled(
                [(w - name_w) / 2.0, (TOOLBAR_HEIGHT + 13.0) / 2.0 - 1.0],
                entry.name.clone(),
                13.0, 600.0, FontStyle::Normal, None,
                TEXT_PRIMARY,
                z + 2,
            );

            // Position indicator (right): "3 / 12"
            let pos = format!("{} / {}", index + 1, self.entries.len());
            let pos_w = canvas.measure_text_width(&pos, 12.0);
            canvas.draw_text_run_styled(
                [w - pos_w - 12.0, (TOOLBAR_HEIGHT + 12.0) / 2.0 - 1.0],
                pos,
                12.0, 400.0, FontStyle::Normal, None,
                TEXT_SECONDARY,
                z + 2,
            );
        }
    }

    // ── Preview image ───────────────────────────────────────────

    fn render_preview_image(
        &self,
        canvas: &mut detir_surface::Canvas,
        w: f32,
        h: f32,
        index: usize,
        z: i32,
    ) {
        let img_area_y = TOOLBAR_HEIGHT;
        let img_area_h = h - img_area_y;

        // Dark preview background
        canvas.fill_rect(0.0, img_area_y, w, img_area_h, Brush::Solid(PREVIEW_BG), z);

        if let Some(entry) = self.entries.get(index) {
            // Center image in the available area with padding
            let padding = 20.0;
            let avail_w = w - padding * 2.0;
            let avail_h = img_area_h - padding * 2.0;

            if avail_w > 0.0 && avail_h > 0.0 {
                let img_x = padding;
                let img_y = img_area_y + padding;

                if entry.is_svg {
                    canvas.draw_svg(
                        &entry.path,
                        [img_x, img_y],
                        [avail_w, avail_h],
                        z + 1,
                    );
                } else {
                    canvas.draw_image(
                        &entry.path,
                        [img_x, img_y],
                        [avail_w, avail_h],
                        detir_surface::ImageFitMode::Contain,
                        z + 1,
                    );
                }
            }
        }
    }
}
```

**Step 2: Verify build**

Run: `cd orus-universal && cargo build -p orus-imageviewer`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add orus-universal/crates/system-apps/orus-imageviewer/src/render.rs
git commit -m "feat(imageviewer): gallery grid and preview mode rendering"
```

---

## Task 3: Wire into the Orus OS shell — WindowContent, AppInput, app_thread

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/app_thread.rs`
- Modify: `orus-universal/crates/orus-shell/src/window_manager.rs`
- Modify: `orus-universal/crates/orus-shell/Cargo.toml`

**Step 1: Add `orus-imageviewer` dependency to orus-shell**

In `orus-universal/crates/orus-shell/Cargo.toml`, add:
```toml
orus-imageviewer = { workspace = true }
```

**Step 2: Add `ImageViewerState` and `AppInput` variants to `app_thread.rs`**

Follow the exact FileManager pattern. Add to the `AppInput` enum:
```rust
ImageViewerClick { x: f32, y: f32 },
ImageViewerKey { key: orus_imageviewer::ImageViewerKey },
ImageViewerScroll { dy: f32 },
ImageViewerResize { w: f32, h: f32 },
```

Add state struct:
```rust
pub struct ImageViewerState {
    pub app: orus_imageviewer::ImageViewerApp,
}

// SAFETY: ImageViewerState is only accessed through Mutex, serializing all access.
unsafe impl Send for ImageViewerState {}
```

Add thread functions (copy FileManager thread pattern):
```rust
pub fn spawn_imageviewer_thread(
    state: Arc<Mutex<ImageViewerState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("orus-imageviewer".into())
        .spawn(move || imageviewer_thread_loop(state, flags, input_rx))
        .expect("failed to spawn imageviewer thread")
}

fn imageviewer_thread_loop(
    state: Arc<Mutex<ImageViewerState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) {
    loop {
        match input_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(AppInput::Shutdown) => return,
            Ok(input) => {
                let mut s = state.lock().unwrap();
                let consumed = match input {
                    AppInput::ImageViewerClick { x, y } => {
                        s.app.handle_input(orus_imageviewer::ImageViewerInput::Click { x, y })
                    }
                    AppInput::ImageViewerKey { key } => {
                        s.app.handle_input(orus_imageviewer::ImageViewerInput::KeyPress(key))
                    }
                    AppInput::ImageViewerScroll { dy } => {
                        s.app.handle_input(orus_imageviewer::ImageViewerInput::Scroll { dy })
                    }
                    AppInput::ImageViewerResize { w, h } => {
                        s.app.handle_input(orus_imageviewer::ImageViewerInput::Resize { w, h })
                    }
                    _ => false,
                };
                if consumed {
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

pub fn create_imageviewer(
    app: orus_imageviewer::ImageViewerApp,
) -> (Arc<Mutex<ImageViewerState>>, WindowContent) {
    let state = Arc::new(Mutex::new(ImageViewerState { app }));
    let flags = Arc::new(AppFlags::new());
    let (input_tx, input_rx) = mpsc::channel();

    let join_handle =
        spawn_imageviewer_thread(Arc::clone(&state), Arc::clone(&flags), input_rx);

    let content = WindowContent::ImageViewer {
        state: Arc::clone(&state),
        thread: AppThreadHandle {
            input_tx,
            flags,
            join_handle: Some(join_handle),
        },
    };

    (state, content)
}
```

**Step 3: Add `WindowContent::ImageViewer` variant to `window_manager.rs`**

Add to the `WindowContent` enum:
```rust
ImageViewer {
    state: Arc<Mutex<ImageViewerState>>,
    thread: AppThreadHandle,
},
```

Update all match arms in `WindowContent` methods:
- `label()` → return `"Image Viewer"`
- `check_redraw()` → access `thread.flags`
- `has_exited()` → access `thread.flags`
- `send_input()` → route via `thread.input_tx`

**Step 4: Verify build**

Run: `cd orus-universal && cargo build -p orus-shell`
Expected: Compiles (with warnings about unused WindowContent::ImageViewer — that's fine, runtime.rs hasn't been wired yet).

**Step 5: Commit**

```bash
git add orus-universal/crates/orus-shell/
git commit -m "feat(shell): wire ImageViewer into WindowContent and app_thread"
```

---

## Task 4: Wire into launcher and runtime — launch, render, input routing

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/launcher.rs`
- Modify: `orus-universal/crates/orus-shell/src/runtime.rs`

**Step 1: Add `AppId::ImageViewer` to launcher**

In `launcher.rs`, add `ImageViewer` to the `AppId` enum. Add an `AppEntry` for it in the app list (after FileManager), with a distinct icon color. Add the icon rendering case (use "IV" or a simple shape).

**Step 2: Add launch handler in `runtime.rs`**

Follow the FileManager launch pattern exactly:

```rust
Some(LauncherAction::Launch(AppId::ImageViewer)) => {
    launcher.visible = false;

    let lw = (size.width as f32 / scale_factor) as u32;
    let lh = (size.height as f32 / scale_factor) as u32;
    let viewport = ShellLayout::calculate(lw, lh).app_viewport;
    let bounds = window_bounds_from_viewport(viewport);
    let mut aw = AppWindow::new("Image Viewer", bounds);

    let root = sandbox.root().to_path_buf();
    let app = orus_imageviewer::ImageViewerApp::new(root, provider.clone());
    aw.toggle_maximize(viewport);
    let (_state, content) = app_thread::create_imageviewer(app);
    let id = wm.add("imageviewer", aw, content);
    Some(id)
}
```

**Step 3: Add rendering dispatch in runtime.rs**

Find the rendering section where `WindowContent::FileManager` is matched (around line 1522) and add immediately after:

```rust
WindowContent::ImageViewer { state, thread } => {
    let mut s = state.lock().unwrap();
    let cr = mw.app_window.content_rect();
    canvas.push_clip_rect(cr);
    canvas.set_text_provider(s.app.provider.clone());
    s.app.set_origin(cr.x, cr.y);
    s.app.update_viewport(cr.w, cr.h);
    s.app.render(&mut canvas, cr.w, cr.h, mw.app_window.z_base + 10);
    canvas.pop_clip();

    // Handle folder picker request (must run on main thread for macOS)
    if s.app.needs_folder_picker {
        s.app.needs_folder_picker = false;
        drop(s); // Release lock before blocking dialog
        let root = sandbox.root().to_path_buf();
        let picked = rfd::FileDialog::new()
            .set_directory(&root)
            .set_title("Select Image Folder")
            .pick_folder();
        if let Some(folder) = picked {
            let mut s = state.lock().unwrap();
            s.app.open_folder(&folder);
        }
        needs_redraw = true;
    } else if thread.flags.needs_redraw.swap(false, std::sync::atomic::Ordering::Relaxed) {
        needs_redraw = true;
    }
}
```

**Step 4: Add input routing in runtime.rs**

Find all the places where `WindowContent::FileManager` is matched for input handling and add `WindowContent::ImageViewer` cases alongside them:

1. **Mouse click** — convert to `AppInput::ImageViewerClick { x: lx, y: ly }`
2. **Scroll** — convert to `AppInput::ImageViewerScroll { dy }`
3. **Keyboard** — map arrow keys, Enter, Escape to `ImageViewerKey` variants and send as `AppInput::ImageViewerKey { key }`
4. **Window resize** — send `AppInput::ImageViewerResize { w, h }`
5. **Window close/shutdown** — handle `WindowContent::ImageViewer { thread, .. }` in shutdown and remove patterns

Search for every `FileManager` match arm in runtime.rs and add corresponding `ImageViewer` handling. The key mappings:
- `NamedKey::Escape` → `ImageViewerKey::Escape`
- `NamedKey::Enter` → `ImageViewerKey::Enter`
- `NamedKey::ArrowLeft` → `ImageViewerKey::ArrowLeft`
- `NamedKey::ArrowRight` → `ImageViewerKey::ArrowRight`
- `NamedKey::ArrowUp` → `ImageViewerKey::ArrowUp`
- `NamedKey::ArrowDown` → `ImageViewerKey::ArrowDown`

**Step 5: Verify build and run**

Run: `cd orus-universal && cargo build -p orus-macos`
Expected: Compiles without errors.

Run: `cd orus-universal && cargo run -p orus-macos`
Expected: Image Viewer appears in launcher. Clicking it opens a folder picker dialog. After selecting a folder with images, the gallery grid appears with thumbnails. Clicking a thumbnail shows the full-size preview.

**Step 6: Commit**

```bash
git add orus-universal/crates/orus-shell/src/launcher.rs orus-universal/crates/orus-shell/src/runtime.rs
git commit -m "feat(shell): wire ImageViewer into launcher and runtime with folder picker"
```

---

## Task 5: Add dock entry icon and session persistence

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/runtime.rs` (dock entry)

**Step 1: Add dock entry for Image Viewer**

Find where dock entries are created for other apps (search for `DockEntry` in runtime.rs). Add an ImageViewer dock entry following the same pattern.

**Step 2: Add session save/restore for Image Viewer** (optional, skip if session system doesn't support it yet)

Check if the session persistence system handles FileManager. If so, add equivalent ImageViewer state serialization (just the current folder path).

**Step 3: Verify**

Run: `cd orus-universal && cargo run -p orus-macos`
Expected: Image Viewer icon appears in dock when open. Can switch to it from other apps.

**Step 4: Commit**

```bash
git add orus-universal/crates/orus-shell/src/runtime.rs
git commit -m "feat(shell): add dock entry and session support for Image Viewer"
```

---

## Notes

- **rfd** is already a workspace dependency at version 0.15, no new deps needed.
- The `draw_image` Canvas API handles image loading, caching, and GPU upload automatically via engine-core's `ImageCache`. No manual texture management needed.
- SVGs use `draw_svg` / `draw_svg_styled` which is a separate rendering path (tessellation or rasterization).
- The folder picker (`rfd::FileDialog::pick_folder()`) **must** run on the main thread on macOS (AppKit requirement). This is why `needs_folder_picker` is a flag checked during the render loop on the main thread, not called from the app thread.
- The `ImageFitMode::Contain` mode preserves aspect ratio and centers the image within the bounds, which is the correct behavior for both thumbnails and preview.
