# SceneRenderer Embedding — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Embed detir-scene's full rendering (toolbar, sidebar, viewport, dock, IR content) inside Orus OS AppWindow using the same `render(canvas, rect, z)` pattern as TerminalRenderer.

**Architecture:** New `SceneRenderer` struct in detir-scene wraps `IrRenderer` + `ZoneManager` + documents. Orus OS creates instances, forwards input, calls render per frame. Existing `run()` in runner.rs stays untouched.

**Tech Stack:** detir-scene (existing), engine-core Canvas API, detir-surface, detir-ir

**Design Doc:** `orus-universal/docs/plans/2026-03-03-scene-renderer-embedding-design.md`

---

## Progress Checklist

- [ ] Task 1: Promote private functions in runner.rs to `pub(crate)`
- [ ] Task 2: Create SceneRenderer struct with new()
- [ ] Task 3: Implement SceneRenderer::render()
- [ ] Task 4: Implement input forwarding methods
- [ ] Task 5: Implement poll() and needs_redraw()
- [ ] Task 6: Export SceneRenderer from detir-scene lib.rs
- [ ] Task 7: Add detir-scene dependency to orus-shell
- [ ] Task 8: Create AppContent enum and wire into AppWindow
- [ ] Task 9: Integrate SceneRenderer into orus-macos render loop
- [ ] Task 10: Integrate input forwarding into orus-macos event handlers
- [ ] Task 11: Build and verify

---

## Task 1: Promote private functions in runner.rs to `pub(crate)`

**Files:**
- Modify: `detir/crates/detir-scene/src/ir_renderer/runner.rs:43-108` (types)
- Modify: `detir/crates/detir-scene/src/ir_renderer/runner.rs:5834` (load_package_from_source)
- Modify: `detir/crates/detir-scene/src/ir_renderer/runner.rs:6811` (spawn_web_load)
- Modify: `detir/crates/detir-scene/src/ir_renderer/runner.rs:6830` (spawn_web_font_fetch)
- Modify: `detir/crates/detir-scene/src/ir_renderer/runner.rs:68` (create_arth_host)

**Step 1: Promote types to pub(crate)**

Change these from private to `pub(crate)`:

```rust
// runner.rs:43
pub(crate) struct WebLoadResult {
    pub(crate) url: String,
    pub(crate) result: Option<(DetirPackage, DataDocument, ViewDocument, PackageSource)>,
    pub(crate) pending_fonts: Vec<crate::web_fonts::PendingFont>,
}

// runner.rs:50
pub(crate) struct WebFontLoadResult {
    pub(crate) url: String,
    pub(crate) fonts: Vec<crate::web_fonts::ResolvedFont>,
}

// runner.rs:99
#[derive(Clone, Debug)]
pub(crate) enum PackageSource {
    Path(String),
    BuiltIn,
    Plugin(String),
    Url(String),
}
```

**Step 2: Promote functions to pub(crate)**

```rust
// runner.rs:68
pub(crate) fn create_arth_host(http_client: reqwest::blocking::Client) -> Result<ArthHost> { ... }

// runner.rs:5834
pub(crate) fn load_package_from_source(
    http: &reqwest::blocking::Client,
    source: &str,
) -> Result<(DetirPackage, DataDocument, ViewDocument, PackageSource)> { ... }

// runner.rs:6811
pub(crate) fn spawn_web_load(url: &str, tx: &mpsc::Sender<WebLoadResult>, http: &reqwest::blocking::Client) { ... }

// runner.rs:6830
pub(crate) fn spawn_web_font_fetch(
    url: &str,
    pending_fonts: Vec<crate::web_fonts::PendingFont>,
    tx: &mpsc::Sender<WebFontLoadResult>,
    ...
) { ... }
```

Also promote the helper function used by `load_package_from_source`:

```rust
// runner.rs — find load_default_package, load_package_from_path, is_url_candidate, try_load_ir_from_url
// Promote all to pub(crate)
```

**Step 3: Build to verify no breakage**

Run: `cd /Users/yppartha/PROJECTS/DETIR_WAID/detir && cargo build -p detir-scene --no-default-features 2>&1 | head -30`
Expected: Compiles successfully (pub(crate) is strictly additive)

**Step 4: Commit**

```bash
git add detir/crates/detir-scene/src/ir_renderer/runner.rs
git commit -m "refactor(detir-scene): promote runner internals to pub(crate) for embedding"
```

---

## Task 2: Create SceneRenderer struct with new()

**Files:**
- Create: `detir/crates/detir-scene/src/embeddable.rs`

**Step 1: Create the SceneRenderer struct**

```rust
//! Embeddable scene renderer for hosting detir-scene inside external compositors.
//!
//! `SceneRenderer` wraps `IrRenderer` + `ZoneManager` and renders the full
//! detir-scene experience (toolbar, sidebar, viewport, dock) into a caller-
//! provided `Canvas` at a given `Rect` — same pattern as TerminalRenderer.

use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use anyhow::Result;
use engine_core::{HitIndex, Rect, TextProvider, Transform2D};

use crate::ir_renderer::core::IrRenderer;
use crate::ir_renderer::runner::{
    PackageSource, WebFontLoadResult, WebLoadResult,
    create_arth_host, load_package_from_source, spawn_web_load,
};
use crate::ir_renderer::ArthHost;
use crate::theme::{ResolvedTheme, ThemePalette};
use crate::zones::ZoneManager;

use detir_ir::data::DataDocument;
use detir_ir::view::ViewDocument;

/// Input modifiers state.
pub use detir_platform::Modifiers;

/// Embeddable scene renderer.
///
/// Renders the full detir-scene experience (zones + IR content) into a
/// sub-region of a shared `Canvas`. Created with a URL, renders per frame.
pub struct SceneRenderer {
    // Rendering
    ir_renderer: IrRenderer,
    zone_manager: ZoneManager,
    theme: ResolvedTheme,

    // Content
    view_doc: ViewDocument,
    data_doc: DataDocument,
    package_source: PackageSource,

    // Logic
    arth_host: ArthHost,

    // Text
    provider: Arc<dyn TextProvider>,

    // Background loading
    web_load_tx: mpsc::Sender<WebLoadResult>,
    web_load_rx: mpsc::Receiver<WebLoadResult>,
    web_font_tx: mpsc::Sender<WebFontLoadResult>,
    web_font_rx: mpsc::Receiver<WebFontLoadResult>,
    http_client: reqwest::blocking::Client,

    // Input state
    cursor_position: Option<(f32, f32)>,
    hit_index: Option<HitIndex>,
    last_click_time: Option<Instant>,
    click_count: u32,
    needs_redraw: bool,

    // Sizing
    width: f32,
    height: f32,
}
```

**Step 2: Implement SceneRenderer::new()**

```rust
impl SceneRenderer {
    /// Create a new embeddable scene renderer from a source URL or path.
    ///
    /// `source` can be:
    /// - `"detir://sample/home_tab"` — built-in sample
    /// - `"http://..."` — web URL (loaded synchronously on creation)
    /// - `"/path/to/ir-package/"` — filesystem path
    ///
    /// `provider` is a shared text provider (fonts). Orus OS typically creates
    /// one from `DetirTextProvider::from_system_fonts()` and shares it.
    pub fn new(
        source: &str,
        provider: Arc<dyn TextProvider>,
        width: f32,
        height: f32,
    ) -> Result<Self> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        // Load package from source URL/path
        let (package, data_doc, view_doc, package_source) =
            load_package_from_source(&http_client, source)?;

        // Theme
        let resolved_theme = ResolvedTheme::Dark;
        let theme_palette = ThemePalette::for_theme(resolved_theme);

        // Core components
        let mut ir_renderer = IrRenderer::new_with_theme(resolved_theme);
        let zone_manager = ZoneManager::new(
            width as u32,
            height as u32,
            theme_palette,
        );

        // Arth host for logic modules
        let mut arth_host = create_arth_host(http_client.clone())?;
        arth_host.load_package(&package)?;

        // Background loading channels
        let (web_load_tx, web_load_rx) = mpsc::channel();
        let (web_font_tx, web_font_rx) = mpsc::channel();

        Ok(Self {
            ir_renderer,
            zone_manager,
            theme: resolved_theme,
            view_doc,
            data_doc,
            package_source,
            arth_host,
            provider,
            web_load_tx,
            web_load_rx,
            web_font_tx,
            web_font_rx,
            http_client,
            cursor_position: None,
            hit_index: None,
            last_click_time: None,
            click_count: 0,
            needs_redraw: true,
            width,
            height,
        })
    }
}
```

**Step 3: Build to verify**

Run: `cd /Users/yppartha/PROJECTS/DETIR_WAID/detir && cargo check -p detir-scene 2>&1 | head -30`

Note: This will fail until Task 6 wires the module into lib.rs. For now, verify the file compiles conceptually. We'll do a full build in Task 6.

**Step 4: Commit**

```bash
git add detir/crates/detir-scene/src/embeddable.rs
git commit -m "feat(detir-scene): add SceneRenderer struct with new()"
```

---

## Task 3: Implement SceneRenderer::render()

**Files:**
- Modify: `detir/crates/detir-scene/src/embeddable.rs`

**Reference:** `runner.rs:7132-7300` (`render_frame_with_zones()`)

**Step 1: Implement render method**

The key insight: we DON'T call `render_frame_with_zones()` because that function owns the surface frame lifecycle. Instead, we replicate just the **canvas drawing** portion — the same zone rendering + IR content rendering, but into an existing canvas at a given rect.

```rust
impl SceneRenderer {
    /// Render the full scene into a sub-region of the canvas.
    ///
    /// Pushes a transform+clip to constrain all rendering to `rect`,
    /// then renders zones (toolbar, sidebar, dock) and IR content.
    pub fn render(
        &mut self,
        canvas: &mut detir_surface::Canvas,
        rect: Rect,
        z_base: u32,
    ) -> Result<()> {
        // Push into content_rect coordinate space
        canvas.push_transform(Transform2D::translate(rect.x, rect.y));
        canvas.push_clip_rect(Rect {
            x: 0.0,
            y: 0.0,
            w: rect.w,
            h: rect.h,
        });

        // Resize zone manager if dimensions changed
        if (self.width - rect.w).abs() > 0.5 || (self.height - rect.h).abs() > 0.5 {
            self.width = rect.w;
            self.height = rect.h;
            self.zone_manager.resize(rect.w as u32, rect.h as u32);
        }

        // Clear content background
        let bg = engine_core::ColorLinPremul::from_srgba_u8([32, 32, 36, 255]);
        canvas.fill_rect(0.0, 0.0, rect.w, rect.h, engine_core::Brush::Solid(bg), z_base);

        // Render zone chrome (toolbar, sidebar, dock, devtools, chat)
        crate::zones::render::render_zones(
            canvas,
            &mut self.zone_manager,
            self.provider.as_ref(),
        );

        // Render IR content in viewport zone
        let viewport_rect = self.zone_manager.layout.viewport;

        let mut ir_viewport_width = viewport_rect.w;
        if self.zone_manager.viewport.has_vertical_scrollbar {
            ir_viewport_width =
                (ir_viewport_width - crate::zones::VIEWPORT_SCROLLBAR_WIDTH).max(0.0);
        }

        canvas.push_transform(Transform2D::translate(viewport_rect.x, viewport_rect.y));
        canvas.push_clip_rect(Rect {
            x: 0.0,
            y: 0.0,
            w: viewport_rect.w,
            h: viewport_rect.h,
        });
        canvas.push_transform(Transform2D::translate(
            -self.zone_manager.viewport.scroll_offset_x,
            -self.zone_manager.viewport.scroll_offset_y,
        ));

        let layout_height = viewport_rect.h;
        let (content_height, content_width) = self.ir_renderer.render_canvas_at_offset(
            canvas,
            &self.data_doc,
            &self.view_doc,
            0.0,
            0.0,
            ir_viewport_width,
            layout_height,
            viewport_rect.h,
            self.zone_manager.viewport.scroll_offset_x,
            self.zone_manager.viewport.scroll_offset_y,
            self.provider.as_ref(),
            z_base,
        )?;

        canvas.pop_transform(); // scroll
        canvas.pop_clip();
        canvas.pop_transform(); // viewport translate

        // Update content size for scrollbar calculations
        self.zone_manager.viewport.set_content_size(
            content_width,
            content_height,
            ir_viewport_width,
            viewport_rect.h,
        );

        // Render scrollbars (if overflow)
        if let Some((track_rect, thumb_rect)) = self
            .zone_manager
            .viewport
            .vertical_scrollbar_geometry(viewport_rect.w, viewport_rect.h)
        {
            use engine_core::Brush;
            let track_color = engine_core::ColorLinPremul::from_srgba_u8([15, 23, 42, 180]);
            let thumb_color = engine_core::ColorLinPremul::from_srgba_u8([148, 163, 184, 230]);

            canvas.push_transform(Transform2D::translate(viewport_rect.x, viewport_rect.y));

            canvas.fill_rect(
                track_rect.x, track_rect.y, track_rect.w, track_rect.h,
                Brush::Solid(track_color), z_base + 7000,
            );
            canvas.fill_rect(
                thumb_rect.x, thumb_rect.y, thumb_rect.w, thumb_rect.h,
                Brush::Solid(thumb_color), z_base + 7001,
            );

            canvas.pop_transform();
        }

        // Build hit index for this frame's geometry
        self.hit_index = Some(HitIndex::build(canvas.display_list()));

        canvas.pop_clip();
        canvas.pop_transform(); // content_rect translate

        self.needs_redraw = false;
        Ok(())
    }
}
```

**Step 2: Commit**

```bash
git add detir/crates/detir-scene/src/embeddable.rs
git commit -m "feat(detir-scene): implement SceneRenderer::render()"
```

---

## Task 4: Implement input forwarding methods

**Files:**
- Modify: `detir/crates/detir-scene/src/embeddable.rs`

**Reference:** `runner.rs:1557-1656` (CursorMoved), `runner.rs:1800-1983` (scroll), `runner.rs:1988-3905` (click)

**Step 1: Implement coordinate helpers and input methods**

```rust
impl SceneRenderer {
    /// Check if a point (in window-logical coordinates) is inside the given rect.
    fn contains(rect: &Rect, x: f32, y: f32) -> bool {
        x >= rect.x && x < rect.x + rect.w && y >= rect.y && y < rect.y + rect.h
    }

    /// Mouse moved. Coordinates are window-logical. Returns true if needs redraw.
    pub fn handle_mouse_move(&mut self, logical_x: f32, logical_y: f32, rect: Rect) -> bool {
        let local_x = logical_x - rect.x;
        let local_y = logical_y - rect.y;
        self.cursor_position = Some((local_x, local_y));
        // Zone hover effects can trigger redraws
        false
    }

    /// Scroll event. Returns true if needs redraw.
    pub fn handle_scroll(
        &mut self,
        delta_x: f32,
        delta_y: f32,
        logical_x: f32,
        logical_y: f32,
        rect: Rect,
    ) -> bool {
        let local_x = logical_x - rect.x;
        let local_y = logical_y - rect.y;

        // Route scroll to viewport
        let viewport = self.zone_manager.layout.viewport;
        if Self::contains(&viewport, local_x, local_y) {
            use crate::zones::{ZoneId, ZoneInputEvent, ZoneInputKind};
            let event = ZoneInputEvent {
                zone: ZoneId::Viewport,
                local_x: local_x - viewport.x,
                local_y: local_y - viewport.y,
                kind: ZoneInputKind::Scroll {
                    delta_x,
                    delta_y,
                },
                region_id: None,
            };
            self.zone_manager.handle_input(
                event,
                &mut self.ir_renderer,
                &mut self.arth_host,
            );
            self.needs_redraw = true;
            return true;
        }
        false
    }

    /// Mouse click/release. Returns true if needs redraw.
    pub fn handle_mouse_input(
        &mut self,
        pressed: bool,
        logical_x: f32,
        logical_y: f32,
        rect: Rect,
    ) -> bool {
        let local_x = logical_x - rect.x;
        let local_y = logical_y - rect.y;

        // Determine which zone was clicked
        let layout = &self.zone_manager.layout;
        let zone = if Self::contains(&layout.toolbar, local_x, local_y) {
            crate::zones::ZoneId::Toolbar
        } else if Self::contains(&layout.sidebar, local_x, local_y) {
            crate::zones::ZoneId::Sidebar
        } else {
            crate::zones::ZoneId::Viewport
        };

        let zone_rect = self.zone_manager.layout.get_zone_rect(zone);
        let event = crate::zones::ZoneInputEvent {
            zone,
            local_x: local_x - zone_rect.x,
            local_y: local_y - zone_rect.y,
            kind: crate::zones::ZoneInputKind::MouseClick {
                button: detir_platform::MouseButton::Left,
                state: if pressed {
                    detir_platform::ElementState::Pressed
                } else {
                    detir_platform::ElementState::Released
                },
                click_count: 1,
            },
            region_id: self.hit_index.as_ref().and_then(|hi| {
                hi.topmost_at([local_x, local_y]).map(|h| h.region_id)
            }),
        };

        self.zone_manager.handle_input(
            event,
            &mut self.ir_renderer,
            &mut self.arth_host,
        );
        self.needs_redraw = true;
        true
    }

    /// Keyboard event. Returns true if handled.
    pub fn handle_key(
        &mut self,
        key: detir_platform::KeyCode,
        pressed: bool,
        modifiers: detir_platform::Modifiers,
        text: Option<&str>,
    ) -> bool {
        let state = if pressed {
            detir_platform::ElementState::Pressed
        } else {
            detir_platform::ElementState::Released
        };

        self.zone_manager.handle_keyboard_for_zones(
            key,
            state,
            modifiers,
            text,
            None,
        )
    }

    /// Resize the scene renderer viewport.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.zone_manager.resize(width as u32, height as u32);
        self.needs_redraw = true;
    }
}
```

Note: The exact method signatures on `ZoneManager::handle_input`, `ZoneInputEvent`, `ZoneInputKind`, `ZoneId` etc. need to be checked against the actual code during implementation. The above is the structural pattern — adapt field names and enum variants to match the real types.

**Step 2: Commit**

```bash
git add detir/crates/detir-scene/src/embeddable.rs
git commit -m "feat(detir-scene): implement SceneRenderer input forwarding"
```

---

## Task 5: Implement poll() and needs_redraw()

**Files:**
- Modify: `detir/crates/detir-scene/src/embeddable.rs`

**Reference:** `runner.rs:4901` (web_load_rx poll), `runner.rs:5013` (web_font_rx poll)

**Step 1: Implement poll and needs_redraw**

```rust
impl SceneRenderer {
    /// Poll background tasks (HTTP page loads, web font loads).
    /// Call once per frame. Returns true if state changed and redraw needed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;

        // Poll web content loads
        if let Ok(result) = self.web_load_rx.try_recv() {
            if let Some((package, data_doc, view_doc, source)) = result.result {
                self.data_doc = data_doc;
                self.view_doc = view_doc;
                self.package_source = source;
                let _ = self.arth_host.load_package(&package);
                changed = true;
            }
            // Handle pending fonts
            if !result.pending_fonts.is_empty() {
                crate::ir_renderer::runner::spawn_web_font_fetch(
                    &result.url,
                    result.pending_fonts,
                    &self.web_font_tx,
                );
            }
        }

        // Poll web font loads
        if let Ok(result) = self.web_font_rx.try_recv() {
            for font in result.fonts {
                // Register loaded fonts with text provider
                // (check actual TextProvider API during implementation)
            }
            changed = true;
        }

        if changed {
            self.needs_redraw = true;
        }
        changed
    }

    /// Whether the scene needs a redraw.
    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    /// Navigate to a new URL. Triggers async background load.
    pub fn navigate(&mut self, url: &str) {
        spawn_web_load(url, &self.web_load_tx, &self.http_client);
        self.needs_redraw = true;
    }
}
```

**Step 2: Commit**

```bash
git add detir/crates/detir-scene/src/embeddable.rs
git commit -m "feat(detir-scene): implement SceneRenderer poll + navigate"
```

---

## Task 6: Export SceneRenderer from detir-scene lib.rs

**Files:**
- Modify: `detir/crates/detir-scene/src/lib.rs:11-39` (add module)

**Step 1: Add embeddable module to lib.rs**

Add after line 39 (after the existing `pub mod zones;` line):

```rust
pub mod embeddable;
pub use embeddable::SceneRenderer;
```

**Step 2: Build detir-scene**

Run: `cd /Users/yppartha/PROJECTS/DETIR_WAID/detir && cargo build -p detir-scene 2>&1 | head -50`

Expected: May have compile errors from Tasks 2-5. Fix type mismatches, missing imports, wrong method signatures. The plan provides structural patterns — actual field names and method signatures must match the real code.

**Step 3: Iterate until it compiles**

Common issues to expect:
- `ZoneInputEvent` field names may differ — check `detir/crates/detir-scene/src/zones/mod.rs`
- `ZoneManager::handle_input` signature may differ — check actual params
- `render_canvas_at_offset` param count may differ — check `core.rs:646`
- `ArthHost::load_package` may need different params
- `HitIndex::build` may take different args

Fix each until `cargo build -p detir-scene` passes.

**Step 4: Commit**

```bash
git add detir/crates/detir-scene/src/lib.rs detir/crates/detir-scene/src/embeddable.rs
git commit -m "feat(detir-scene): export SceneRenderer from lib.rs"
```

---

## Task 7: Add detir-scene dependency to orus-shell

**Files:**
- Modify: `orus-universal/crates/orus-shell/Cargo.toml`

**Step 1: Add detir-scene to dependencies**

```toml
[dependencies]
engine-core = { workspace = true }
detir-surface = { workspace = true }
detir-scene = { workspace = true }
detir-platform = { workspace = true }
chrono = { workspace = true }
log = { workspace = true }
anyhow = { workspace = true }
```

Note: `detir-scene` is already in the workspace Cargo.toml as:
```toml
detir-scene = { path = "../detir/crates/detir-scene", default-features = false }
```

Also add `detir-platform` since we need `KeyCode`, `Modifiers`, etc. for input forwarding. And `anyhow` for `Result`.

**Step 2: Build to verify**

Run: `cd /Users/yppartha/PROJECTS/DETIR_WAID/kilneos-universal && cargo check -p orus-shell 2>&1 | head -20`
Expected: Compiles (no code changes yet, just dependency added)

**Step 3: Commit**

```bash
git add orus-universal/crates/orus-shell/Cargo.toml
git commit -m "build(orus-shell): add detir-scene dependency"
```

---

## Task 8: Create AppContent enum and wire into AppWindow

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/app_window.rs`
- Modify: `orus-universal/crates/orus-shell/src/lib.rs`

**Step 1: Add AppContent enum**

Add to `app_window.rs` (after the existing `AppWindow` struct):

```rust
/// Content type inside an AppWindow.
///
/// Each AppWindow holds either a terminal or an IR scene (or nothing yet).
/// This enum allows the render loop to dispatch to the right renderer.
pub enum AppContent {
    /// Embedded terminal (existing functionality).
    Terminal,
    /// Embedded detir-scene IR renderer.
    IrScene {
        scene: detir_scene::SceneRenderer,
    },
}
```

Note: The `Terminal` variant doesn't hold the renderer because `TerminalRenderer` + `TerminalCore` are currently managed separately in main.rs. We keep that pattern and just add the IR scene variant. The enum tells the render loop which content to draw.

**Step 2: Re-export from lib.rs**

Make sure `AppContent` is accessible. In `orus-shell/src/lib.rs`, verify `pub mod app_window;` exists (it should already).

**Step 3: Build**

Run: `cd /Users/yppartha/PROJECTS/DETIR_WAID/kilneos-universal && cargo check -p orus-shell 2>&1 | head -20`

**Step 4: Commit**

```bash
git add orus-universal/crates/orus-shell/src/app_window.rs
git commit -m "feat(orus-shell): add AppContent enum for terminal/IR scene"
```

---

## Task 9: Integrate SceneRenderer into orus-macos render loop

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`
- Modify: `orus-universal/crates/orus-macos/Cargo.toml` (if needed)

**Step 1: Add SceneRenderer alongside existing terminal**

In main.rs, after the existing terminal setup, add an IR scene instance. For now, create one IR scene AppWindow alongside the terminal AppWindow:

```rust
// After terminal setup (~line 90):
use detir_scene::SceneRenderer;

// Create IR scene AppWindow
let mut ir_app_win = AppWindow::new(
    "Detir Browser".to_string(),
    Rect { x: 80.0, y: 100.0, w: 700.0, h: 500.0 },
    50, // z_base (different from terminal window)
);

let mut ir_scene = SceneRenderer::new(
    "detir://sample/home_tab",
    provider.clone(),
    ir_app_win.content_rect().w,
    ir_app_win.content_rect().h,
)?;
```

**Step 2: Render IR scene in RedrawRequested**

In the render loop (around line 450), add IR scene rendering alongside terminal:

```rust
// After terminal rendering:
// IR scene AppWindow
ir_app_win.render(&mut canvas);
{
    let cr = ir_app_win.content_rect();
    ir_scene.render(&mut canvas, cr, ir_app_win.z_base + 10)?;
}
```

**Step 3: Poll in AboutToWait**

Find the `AboutToWait` event handler and add:

```rust
Event::AboutToWait => {
    // ... existing terminal poll ...
    if ir_scene.poll() || ir_scene.needs_redraw() {
        window.request_redraw();
    }
}
```

If there is no `AboutToWait` handler yet, the polling can happen at the start of `RedrawRequested`.

**Step 4: Commit**

```bash
git add orus-universal/crates/orus-macos/
git commit -m "feat(orus-macos): render SceneRenderer inside AppWindow"
```

---

## Task 10: Integrate input forwarding into orus-macos event handlers

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Forward mouse events to IR scene**

In `CursorMoved` handler, add IR scene forwarding:

```rust
WindowEvent::CursorMoved { position, .. } => {
    let lx = position.x as f32 / scale_factor;
    let ly = position.y as f32 / scale_factor;
    // ... existing terminal/app_window logic ...

    // Forward to IR scene if inside its content_rect
    let cr = ir_app_win.content_rect();
    if lx >= cr.x && lx < cr.x + cr.w && ly >= cr.y && ly < cr.y + cr.h {
        if ir_scene.handle_mouse_move(lx, ly, cr) {
            window.request_redraw();
        }
    }
}
```

**Step 2: Forward scroll events**

In `MouseWheel` handler:

```rust
WindowEvent::MouseWheel { delta, .. } => {
    let (dx, dy) = match delta {
        winit::event::MouseScrollDelta::LineDelta(x, y) => (x * 20.0, y * 20.0),
        winit::event::MouseScrollDelta::PixelDelta(pos) => {
            (pos.x as f32 / scale_factor, pos.y as f32 / scale_factor)
        }
    };

    if let Some((lx, ly)) = cursor_pos {
        let cr = ir_app_win.content_rect();
        if lx >= cr.x && lx < cr.x + cr.w && ly >= cr.y && ly < cr.y + cr.h {
            if ir_scene.handle_scroll(dx, dy, lx, ly, cr) {
                window.request_redraw();
            }
        }
    }
}
```

**Step 3: Forward click events**

In `MouseInput` handler:

```rust
// Add click forwarding to IR scene (when click is in IR AppWindow content area)
let cr = ir_app_win.content_rect();
if let Some((lx, ly)) = cursor_pos {
    if lx >= cr.x && lx < cr.x + cr.w && ly >= cr.y && ly < cr.y + cr.h {
        let pressed = matches!(state, ElementState::Pressed);
        if ir_scene.handle_mouse_input(pressed, lx, ly, cr) {
            window.request_redraw();
        }
    }
}
```

Note: This task provides the structural pattern. The actual event handler code in main.rs may use different variable names or control flow. Adapt to fit the existing code structure — don't restructure what works.

**Step 4: Commit**

```bash
git add orus-universal/crates/orus-macos/src/main.rs
git commit -m "feat(orus-macos): forward input events to SceneRenderer"
```

---

## Task 11: Build and verify

**Step 1: Full workspace build**

Run: `cd /Users/yppartha/PROJECTS/DETIR_WAID/kilneos-universal && cargo build -p orus-macos 2>&1 | tail -20`

Expected: Compiles successfully.

**Step 2: Run and visual verify**

Run: `cd /Users/yppartha/PROJECTS/DETIR_WAID/kilneos-universal && cargo run -p orus-macos`

Expected:
- Orus OS shell bar at top (existing, unchanged)
- Terminal AppWindow (existing, unchanged)
- IR scene AppWindow showing detir-scene's home_tab sample with toolbar, sidebar, viewport, dock
- Scrolling works in the IR viewport
- Window decorations (traffic lights, title bar, drag, resize) work on both windows

**Step 3: Fix issues and iterate**

Common issues:
- Z-ordering conflicts between terminal window and IR window (adjust z_base values)
- Coordinate mismatch in input forwarding (check transform stack)
- Missing text in zones (ensure provider is passed correctly)
- Zone layout overflows content_rect (check ZoneManager resize)

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: IR scene rendering inside Orus OS AppWindow"
```

---

## File Summary

| Action | File | Purpose |
|--------|------|---------|
| Modify | `detir/crates/detir-scene/src/ir_renderer/runner.rs` | Promote types/functions to pub(crate) |
| Create | `detir/crates/detir-scene/src/embeddable.rs` | SceneRenderer struct + all methods |
| Modify | `detir/crates/detir-scene/src/lib.rs` | Export embeddable module |
| Modify | `orus-universal/crates/orus-shell/Cargo.toml` | Add detir-scene dependency |
| Modify | `orus-universal/crates/orus-shell/src/app_window.rs` | Add AppContent enum |
| Modify | `orus-universal/crates/orus-macos/src/main.rs` | Create + render + input for SceneRenderer |
