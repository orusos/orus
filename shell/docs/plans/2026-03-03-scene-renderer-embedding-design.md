# Design: Embeddable SceneRenderer for Orus OS AppWindow

> Extract detir-scene's rendering + input handling into a reusable `SceneRenderer` that draws into a shared Canvas — same pattern as TerminalRenderer.

## Problem

detir-scene's `run()` is monolithic: it owns the window, surface, event loop, and entire rendering pipeline (~7000 lines in runner.rs). Orus OS needs to embed IR apps inside AppWindow's `content_rect`, exactly like TerminalRenderer embeds terminal output today:

```rust
// Terminal (works today):
terminal_renderer.render(tc, &mut canvas, content_rect, z_base + 10);

// IR app (goal):
scene_renderer.render(&mut canvas, content_rect, z_base + 10);
```

## Approach

Add a new public type `SceneRenderer` to detir-scene that wraps `IrRenderer` + `ZoneManager` + documents. It renders the full detir-scene experience (toolbar, sidebar, viewport with IR content, dock, devtools, chat) into a caller-provided `Canvas` at a given `Rect` and `z_base`.

**Existing `run()` stays untouched.** The new code lives in a separate file (`embeddable.rs`). Later, `run()` can optionally be refactored to use `SceneRenderer` internally, but that's not required for this design.

## Architecture

```
┌─ Orus OS Shell (owns window, surface, event loop) ──────────┐
│ [Shell Bar]  clock  agent status  Orus OS                    │
│                                                              │
│  ┌─ AppWindow ──────────────────────────────────────────┐    │
│  │ ● ● ●  My IR App                                    │    │
│  │ ┌──────────────────────────────────────────────────┐ │    │
│  │ │ SceneRenderer draws here (content_rect, z_base)  │ │    │
│  │ │ ┌────────────────────────────────────────────┐   │ │    │
│  │ │ │ [Toolbar: URL bar, nav buttons]            │   │ │    │
│  │ │ │ [Sidebar] [  IR Viewport  ] [Chat]         │   │ │    │
│  │ │ │           [  (web page)   ]                │   │ │    │
│  │ │ │ [Dock icons at bottom]                     │   │ │    │
│  │ │ └────────────────────────────────────────────┘   │ │    │
│  │ └──────────────────────────────────────────────────┘ │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─ AppWindow ──────────────────────────────────────────┐    │
│  │ ● ● ●  Terminal                                      │    │
│  │ ┌──────────────────────────────────────────────────┐ │    │
│  │ │ TerminalRenderer draws here (same pattern)       │ │    │
│  │ └──────────────────────────────────────────────────┘ │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

## SceneRenderer API

### Struct

```rust
pub struct SceneRenderer {
    // Core rendering
    ir_renderer: IrRenderer,
    zone_manager: ZoneManager,

    // Content
    view_doc: ViewDocument,
    data_doc: DataDocument,
    package_source: PackageSource,

    // Logic execution
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
    cursor_position: Option<(f32, f32)>,  // content-local coordinates
    modifiers: Modifiers,
    hit_index: Option<HitIndex>,
    click_count: u32,
    last_click_time: Option<Instant>,
    ime_committed_text: bool,

    // Flags
    needs_redraw: bool,
    scale_factor: f32,
}
```

### Creation

```rust
impl SceneRenderer {
    /// Create a new embeddable scene renderer.
    ///
    /// - `default_url`: IR package URL (e.g. "detir://sample/home_tab", "http://...")
    /// - `provider`: Shared text provider (fonts)
    /// - `scale_factor`: DPI scale (1.0 on non-retina, 2.0 on retina)
    /// - `width`, `height`: Initial logical size of the rendering area
    pub fn new(
        default_url: &str,
        provider: Arc<dyn TextProvider>,
        scale_factor: f32,
        width: f32,
        height: f32,
    ) -> Result<Self>;
}
```

### Rendering

```rust
impl SceneRenderer {
    /// Render the full scene into a sub-region of the canvas.
    ///
    /// Applies push_transform + push_clip_rect to constrain rendering
    /// to `rect`, then renders zones + IR content at z_base offset.
    pub fn render(
        &mut self,
        canvas: &mut Canvas,
        rect: Rect,
        z_base: u32,
    ) -> Result<()>;
}
```

**Internal rendering flow:**
```
canvas.push_transform(translate(rect.x, rect.y))
canvas.push_clip_rect(Rect { x: 0, y: 0, w: rect.w, h: rect.h })

  // 1. Clear content area background
  canvas.fill_rect(0, 0, rect.w, rect.h, bg_color, z_base)

  // 2. Render zone chrome (toolbar, sidebar, dock, devtools, chat)
  render_zones(canvas, zone_manager, provider)  // z offsets relative to z_base

  // 3. Render IR content in viewport zone
  let vp = zone_manager.layout.viewport
  canvas.push_transform(translate(vp.x, vp.y))
  canvas.push_clip_rect(vp)
  canvas.push_transform(translate(-scroll_x, -scroll_y))
  ir_renderer.render_canvas_at_offset(canvas, ...)  // z = z_base + zone offsets
  canvas.pop_transform()
  canvas.pop_clip()
  canvas.pop_transform()

  // 4. Scrollbars overlay
  // 5. Build HitIndex for this frame

canvas.pop_clip()
canvas.pop_transform()
```

### Input Forwarding

All input methods take the `rect` parameter so they can translate window-global coordinates to content-local coordinates:

```rust
impl SceneRenderer {
    /// Mouse moved. Coordinates are window-logical (not physical).
    /// Returns true if state changed (needs redraw).
    pub fn handle_mouse_move(
        &mut self,
        logical_x: f32,
        logical_y: f32,
        rect: Rect,
    ) -> bool;

    /// Mouse button press/release.
    pub fn handle_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        logical_x: f32,
        logical_y: f32,
        rect: Rect,
    ) -> bool;

    /// Scroll event.
    pub fn handle_scroll(
        &mut self,
        delta_x: f32,
        delta_y: f32,
        logical_x: f32,
        logical_y: f32,
        rect: Rect,
    ) -> bool;

    /// Keyboard event. No rect needed (not spatial).
    pub fn handle_key(
        &mut self,
        key: KeyCode,
        state: ElementState,
        modifiers: Modifiers,
        text: Option<&str>,
    ) -> bool;

    /// IME text commit.
    pub fn handle_text_input(&mut self, text: &str) -> bool;

    /// Resize the scene viewport.
    pub fn resize(&mut self, width: f32, height: f32);
}
```

**Coordinate translation** (inside each method):
```rust
let local_x = logical_x - rect.x;
let local_y = logical_y - rect.y;
// Forward (local_x, local_y) to zone_manager / ir_renderer
```

### Background Task Polling

```rust
impl SceneRenderer {
    /// Poll background HTTP loads and web font loads.
    /// Call this each frame (or on AboutToWait).
    /// Returns true if state changed (needs redraw).
    pub fn poll(&mut self) -> bool;

    /// Whether the scene needs a redraw.
    pub fn needs_redraw(&self) -> bool;
}
```

## Integration in Orus OS

### In orus-macos/src/main.rs

```rust
// At startup:
let scene = SceneRenderer::new(
    "detir://sample/home_tab",
    provider.clone(),
    scale_factor,
    content_rect.w,
    content_rect.h,
)?;

// In render loop:
let cr = app_window.content_rect();
scene.render(&mut canvas, cr, app_window.z_base + 10)?;

// In event handling:
WindowEvent::CursorMoved { position, .. } => {
    let lx = position.x as f32 / scale_factor;
    let ly = position.y as f32 / scale_factor;
    let cr = app_window.content_rect();
    // Only forward if inside content_rect
    if lx >= cr.x && lx < cr.x + cr.w && ly >= cr.y && ly < cr.y + cr.h {
        scene.handle_mouse_move(lx, ly, cr);
    }
}

// In AboutToWait:
if scene.poll() || scene.needs_redraw() {
    window.request_redraw();
}
```

### AppWindow Types

Each AppWindow can hold either a terminal or an IR scene:

```rust
pub enum AppContent {
    Terminal {
        renderer: TerminalRenderer,
        core: TerminalCore,
    },
    IrScene {
        scene: SceneRenderer,
    },
}
```

## Files Changed

### New files (in detir-scene):
- `detir/crates/detir-scene/src/embeddable.rs` — SceneRenderer struct + impl

### Modified files (in detir-scene):
- `detir/crates/detir-scene/src/lib.rs` — add `pub mod embeddable; pub use embeddable::SceneRenderer;`
- `detir/crates/detir-scene/src/ir_renderer/runner.rs` — extract `render_zones()` to be callable from embeddable.rs (may need to make it `pub(crate)` instead of `fn`)

### Modified files (in orus-universal):
- `orus-universal/crates/orus-shell/Cargo.toml` — add detir-scene dependency
- `orus-universal/crates/orus-shell/src/app_window.rs` — add `AppContent` enum
- `orus-universal/crates/orus-macos/src/main.rs` — create SceneRenderer, wire into render loop + events

## What Stays Untouched

- `detir-scene/src/ir_renderer/runner.rs` `run()` function — no changes to the monolithic runner
- All existing Orus OS functionality — shell bar, terminal embedding, window decorations, drag/resize
- Terminal embedding pattern — continues to work alongside IR scenes

## Extraction Strategy

The embeddable.rs file will:

1. **Reuse IrRenderer creation**: Call `IrRenderer::new_with_theme()` (already public)
2. **Reuse ZoneManager creation**: Call `ZoneManager::new()` (already public)
3. **Reuse package loading**: Extract `load_ir_package()` into a callable function (currently in runner.rs, needs `pub(crate)` or extraction)
4. **Reuse render_zones()**: Make the existing zone rendering function accessible (currently file-private in runner.rs)
5. **Reuse input handling**: `ZoneManager::handle_input()`, `handle_keyboard_for_zones()`, `handle_text_input_for_zones()` are all public
6. **Duplicate minimally**: The "glue" between these components (coordinate transforms, hit-test queries, state updates) will be re-implemented in embeddable.rs since it's tightly coupled to the event loop in runner.rs

## Risks

1. **runner.rs private functions**: `render_zones()`, `load_ir_package()`, `spawn_web_load()` are private. Need to promote to `pub(crate)`.
2. **State coupling**: runner.rs has ~30 local variables managing state. SceneRenderer must replicate the essential ones.
3. **ArthHost lifecycle**: Arth VM needs proper lifecycle management (load package, handle mutations, poll HTTP callbacks).
4. **Background task integration**: HTTP loads and web font loads need channel polling, which currently happens in the event loop's `AboutToWait` handler.

## Non-Goals (for this design)

- Refactoring `run()` to use `SceneRenderer` internally (can be done later)
- Multi-app state management (Phase 2 from implementation plan)
- CEF integration (Phase 5)
- Sandboxing (Phase 3)
