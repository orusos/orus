# Multi-Instance Per-App Threading Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Support multiple terminal and browser instances, each running on its own background thread with channel-based input and shared-state rendering.

**Architecture:** Each app instance gets a background thread for polling (PTY reads, HTTP loads, animations). Input events are sent via `mpsc` channels. App state is wrapped in `Arc<Mutex<>>` so the main thread can lock briefly to render. The main thread retains all GPU work, window chrome, and hit-testing.

**Tech Stack:** Rust `std::sync` (Arc, Mutex, atomic), `std::sync::mpsc` channels, `std::thread`

**Design doc:** `orus-universal/docs/plans/2026-03-05-multi-instance-threading-design.md`

---

### Task 1: Define thread communication types

**Files:**
- Create: `orus-universal/crates/orus-shell/src/app_thread.rs`
- Modify: `orus-universal/crates/orus-shell/src/lib.rs`

**Step 1: Create `app_thread.rs` with AppInput, state wrappers, and thread-safe WindowContent**

```rust
// orus-shell/src/app_thread.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use detir_terminal::TerminalKey;

/// Input events sent from main thread to app thread.
pub enum AppInput {
    /// Terminal key press.
    TerminalKey(TerminalKey),
    /// IR scene keyboard input.
    SceneKey {
        code: detir_platform::KeyCode,
        state: detir_platform::ElementState,
        mods: detir_platform::Modifiers,
        text: Option<String>,
    },
    /// Mouse click in IR scene content area.
    SceneMouseInput {
        button: detir_platform::MouseButton,
        state: detir_platform::ElementState,
        x: f32,
        y: f32,
        content_rect: engine_core::Rect,
    },
    /// Mouse move in IR scene content area.
    SceneMouseMove {
        x: f32,
        y: f32,
        content_rect: engine_core::Rect,
    },
    /// Scroll event.
    Scroll {
        dx: f32,
        dy: f32,
        mx: f32,
        my: f32,
        content_rect: engine_core::Rect,
    },
    /// Terminal scroll (lines).
    TerminalScrollUp(usize),
    TerminalScrollDown(usize),
    /// Window resized — terminal needs cols/rows update.
    TerminalResize { cols: usize, rows: usize },
    /// Navigate browser to URL.
    Navigate(String),
    /// Navigation: go back.
    NavBack,
    /// Navigation: go forward.
    NavForward,
    /// Navigation: reload.
    NavReload,
    /// Show the scene dock (home button).
    ShowDock,
    /// Graceful shutdown.
    Shutdown,
}

/// Shared terminal state behind Arc<Mutex<>>.
pub struct TerminalState {
    pub core: detir_terminal::TerminalCore,
    pub renderer: detir_terminal::TerminalRenderer,
}

/// Shared browser state behind Arc<Mutex<>>.
pub struct BrowserState {
    pub scene: detir_scene::SceneRenderer,
}

/// Signals from app thread to main thread (non-channel, atomic).
pub struct AppFlags {
    /// Set by app thread when content changed; cleared by main after rendering.
    pub needs_redraw: AtomicBool,
    /// Set by app thread when the app has exited (e.g., shell died).
    pub exited: AtomicBool,
}

impl AppFlags {
    pub fn new() -> Self {
        Self {
            needs_redraw: AtomicBool::new(true),
            exited: AtomicBool::new(false),
        }
    }
}

/// Handle to a running app thread.
pub struct AppThreadHandle {
    pub input_tx: std::sync::mpsc::Sender<AppInput>,
    pub flags: Arc<AppFlags>,
    pub join_handle: Option<JoinHandle<()>>,
}
```

**Step 2: Add module to `lib.rs`**

Add `pub mod app_thread;` to `orus-universal/crates/orus-shell/src/lib.rs`.

**Step 3: Build to verify**

Run: `cargo build -p orus-shell`
Expected: compiles successfully

**Step 4: Commit**

```bash
git add orus-universal/crates/orus-shell/src/app_thread.rs orus-universal/crates/orus-shell/src/lib.rs
git commit -m "feat(orus): add thread communication types for per-app threading"
```

---

### Task 2: Refactor WindowContent to use shared state

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/window_manager.rs`

**Step 1: Change WindowContent variants**

Replace the current `WindowContent` enum with thread-safe variants:

```rust
use crate::app_thread::{AppFlags, AppInput, AppThreadHandle, BrowserState, TerminalState};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

pub enum WindowContent {
    Terminal {
        state: Arc<Mutex<TerminalState>>,
        thread: AppThreadHandle,
    },
    IrScene {
        state: Arc<Mutex<BrowserState>>,
        address_bar_focused: bool,
        thread: AppThreadHandle,
    },
}

impl WindowContent {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Terminal { .. } => "Terminal",
            Self::IrScene { .. } => "Detir Browser",
        }
    }

    /// Check if the app thread signaled a need to redraw.
    pub fn check_redraw(&self) -> bool {
        let flags = match self {
            Self::Terminal { thread, .. } => &thread.flags,
            Self::IrScene { thread, .. } => &thread.flags,
        };
        flags.needs_redraw.swap(false, Ordering::Relaxed)
    }

    /// Check if the app has exited.
    pub fn has_exited(&self) -> bool {
        let flags = match self {
            Self::Terminal { thread, .. } => &thread.flags,
            Self::IrScene { thread, .. } => &thread.flags,
        };
        flags.exited.load(Ordering::Relaxed)
    }

    /// Send input to the app thread.
    pub fn send_input(&self, input: AppInput) {
        let tx = match self {
            Self::Terminal { thread, .. } => &thread.input_tx,
            Self::IrScene { thread, .. } => &thread.input_tx,
        };
        let _ = tx.send(input);
    }
}
```

**Step 2: Update `snapshot_session` if needed**

The `snapshot_session` method on `WindowManager` only reads `app_id`, `app_window` (bounds, maximized), so no changes needed there.

**Step 3: Build to verify**

Run: `cargo build -p orus-shell`
Expected: compiles (main.rs will NOT compile yet — that's expected, we'll fix it in later tasks)

**Step 4: Commit**

```bash
git add orus-universal/crates/orus-shell/src/window_manager.rs
git commit -m "refactor(orus): change WindowContent to use Arc<Mutex> shared state"
```

---

### Task 3: Implement terminal thread loop

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/app_thread.rs`

**Step 1: Add terminal thread function**

Append to `app_thread.rs`:

```rust
use std::sync::mpsc;
use std::time::Duration;

/// Spawn a terminal app thread. Returns the thread handle.
pub fn spawn_terminal_thread(
    state: Arc<Mutex<TerminalState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("orus-terminal".into())
        .spawn(move || terminal_thread_loop(state, flags, input_rx))
        .expect("failed to spawn terminal thread")
}

fn terminal_thread_loop(
    state: Arc<Mutex<TerminalState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) {
    loop {
        // Drain all pending input
        loop {
            match input_rx.try_recv() {
                Ok(AppInput::Shutdown) => return,
                Ok(AppInput::TerminalKey(k)) => {
                    let mut s = state.lock().unwrap();
                    s.core.send_key(k);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::TerminalResize { cols, rows }) => {
                    let mut s = state.lock().unwrap();
                    s.core.resize(cols, rows);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::TerminalScrollUp(n)) => {
                    let mut s = state.lock().unwrap();
                    s.core.scroll_view_up(n);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::TerminalScrollDown(n)) => {
                    let mut s = state.lock().unwrap();
                    s.core.scroll_view_down(n);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(_) => {} // Ignore non-terminal inputs
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // Poll PTY for new output
        {
            let mut s = state.lock().unwrap();
            if s.core.poll() {
                flags.needs_redraw.store(true, Ordering::Relaxed);
            }
            if !s.core.is_alive() {
                flags.exited.store(true, Ordering::Relaxed);
                flags.needs_redraw.store(true, Ordering::Relaxed);
                return;
            }
        }

        // Sleep to avoid busy-waiting (~120Hz poll rate)
        std::thread::sleep(Duration::from_millis(8));
    }
}
```

**Step 2: Build to verify**

Run: `cargo build -p orus-shell`
Expected: compiles

**Step 3: Commit**

```bash
git add orus-universal/crates/orus-shell/src/app_thread.rs
git commit -m "feat(orus): implement terminal background thread loop"
```

---

### Task 4: Implement browser thread loop

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/app_thread.rs`

**Step 1: Add browser thread function**

Append to `app_thread.rs`:

```rust
/// Spawn a browser app thread. Returns the thread handle.
pub fn spawn_browser_thread(
    state: Arc<Mutex<BrowserState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("orus-browser".into())
        .spawn(move || browser_thread_loop(state, flags, input_rx))
        .expect("failed to spawn browser thread")
}

fn browser_thread_loop(
    state: Arc<Mutex<BrowserState>>,
    flags: Arc<AppFlags>,
    input_rx: mpsc::Receiver<AppInput>,
) {
    loop {
        // Drain all pending input
        loop {
            match input_rx.try_recv() {
                Ok(AppInput::Shutdown) => return,
                Ok(AppInput::SceneKey { code, state: ks, mods, text }) => {
                    let mut s = state.lock().unwrap();
                    if s.scene.handle_key(code, ks, mods, text.as_deref()) {
                        flags.needs_redraw.store(true, Ordering::Relaxed);
                    }
                }
                Ok(AppInput::SceneMouseInput { button, state: ms, x, y, content_rect }) => {
                    let mut s = state.lock().unwrap();
                    if s.scene.handle_mouse_input(button, ms, x, y, content_rect) {
                        flags.needs_redraw.store(true, Ordering::Relaxed);
                    }
                }
                Ok(AppInput::SceneMouseMove { x, y, content_rect }) => {
                    let mut s = state.lock().unwrap();
                    if s.scene.handle_mouse_move(x, y, content_rect) {
                        flags.needs_redraw.store(true, Ordering::Relaxed);
                    }
                }
                Ok(AppInput::Scroll { dx, dy, mx, my, content_rect }) => {
                    let mut s = state.lock().unwrap();
                    s.scene.handle_scroll(dx, dy, mx, my, content_rect);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::Navigate(url)) => {
                    let mut s = state.lock().unwrap();
                    s.scene.navigate(&url);
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::NavBack) => {
                    let mut s = state.lock().unwrap();
                    if let Some(url) = s.scene.nav_ctx_mut().ir_go_back() {
                        s.scene.load_content(&url);
                    }
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::NavForward) => {
                    let mut s = state.lock().unwrap();
                    if let Some(url) = s.scene.nav_ctx_mut().ir_go_forward() {
                        s.scene.load_content(&url);
                    }
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::NavReload) => {
                    let mut s = state.lock().unwrap();
                    s.scene.nav_ctx_mut().reload();
                    s.scene.mark_dirty();
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(AppInput::ShowDock) => {
                    let mut s = state.lock().unwrap();
                    s.scene.show_dock();
                    flags.needs_redraw.store(true, Ordering::Relaxed);
                }
                Ok(_) => {} // Ignore non-browser inputs
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // Poll for loaded resources and animations
        {
            let mut s = state.lock().unwrap();
            let polled = s.scene.poll();
            let animating = s.scene.has_active_animations();
            if animating {
                s.scene.update_animations(8.0);
            }
            if polled || animating || s.scene.needs_redraw() {
                flags.needs_redraw.store(true, Ordering::Relaxed);
            }
        }

        std::thread::sleep(Duration::from_millis(8));
    }
}
```

**Step 2: Build to verify**

Run: `cargo build -p orus-shell`
Expected: compiles. If `TerminalState`, `BrowserState`, or inner types are not `Send`, wrap with:
```rust
// In app_thread.rs, if needed:
unsafe impl Send for TerminalState {}
unsafe impl Send for BrowserState {}
```

**Step 3: Commit**

```bash
git add orus-universal/crates/orus-shell/src/app_thread.rs
git commit -m "feat(orus): implement browser background thread loop"
```

---

### Task 5: Add helper functions to create threaded app instances

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/app_thread.rs`

**Step 1: Add factory functions**

These encapsulate the pattern: create state + channels + spawn thread + return WindowContent.

```rust
use crate::window_manager::WindowContent;

/// Create a terminal instance with its background thread.
pub fn create_terminal(
    core: detir_terminal::TerminalCore,
    renderer: detir_terminal::TerminalRenderer,
) -> (Arc<Mutex<TerminalState>>, WindowContent) {
    let state = Arc::new(Mutex::new(TerminalState { core, renderer }));
    let flags = Arc::new(AppFlags::new());
    let (input_tx, input_rx) = mpsc::channel();

    let join_handle = spawn_terminal_thread(Arc::clone(&state), Arc::clone(&flags), input_rx);

    let content = WindowContent::Terminal {
        state: Arc::clone(&state),
        thread: AppThreadHandle {
            input_tx,
            flags,
            join_handle: Some(join_handle),
        },
    };

    (state, content)
}

/// Create a browser instance with its background thread.
pub fn create_browser(
    scene: detir_scene::SceneRenderer,
) -> (Arc<Mutex<BrowserState>>, WindowContent) {
    let state = Arc::new(Mutex::new(BrowserState { scene }));
    let flags = Arc::new(AppFlags::new());
    let (input_tx, input_rx) = mpsc::channel();

    let join_handle = spawn_browser_thread(Arc::clone(&state), Arc::clone(&flags), input_rx);

    let content = WindowContent::IrScene {
        state: Arc::clone(&state),
        address_bar_focused: false,
        thread: AppThreadHandle {
            input_tx,
            flags,
            join_handle: Some(join_handle),
        },
    };

    (state, content)
}

/// Shut down an app thread gracefully.
pub fn shutdown_thread(handle: &mut AppThreadHandle) {
    let _ = handle.input_tx.send(AppInput::Shutdown);
    if let Some(jh) = handle.join_handle.take() {
        let _ = jh.join();
    }
}
```

**Step 2: Build to verify**

Run: `cargo build -p orus-shell`
Expected: compiles

**Step 3: Commit**

```bash
git add orus-universal/crates/orus-shell/src/app_thread.rs
git commit -m "feat(orus): add factory functions for threaded app instances"
```

---

### Task 6: Update main.rs — window creation and launcher

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

This is the largest task. It updates all window creation code and the launcher.

**Step 1: Update imports**

Replace/add at top of main.rs:

```rust
use orus_shell::app_thread::{self, AppInput, BrowserState, TerminalState};
```

**Step 2: Update default window creation (lines ~302-368)**

Replace the terminal creation block:

```rust
// Before (old):
// wm.add("terminal", term_aw, WindowContent::Terminal { core: term_core, renderer: term_renderer });

// After (new):
let (_, term_content) = app_thread::create_terminal(term_core, term_renderer);
wm.add("terminal", term_aw, term_content);
```

Replace the browser creation block:

```rust
// Before:
// wm.add("detir-browser", ir_aw, WindowContent::IrScene { scene, address_bar_focused: false });

// After:
let (_, browser_content) = app_thread::create_browser(scene);
wm.add("detir-browser", ir_aw, browser_content);
```

**Step 3: Update session restore (lines ~208-284)**

Same pattern — replace direct `WindowContent::Terminal { core, renderer }` with `app_thread::create_terminal(core, renderer)` and `WindowContent::IrScene { scene, .. }` with `app_thread::create_browser(scene)`.

**Step 4: Update launcher — remove "focus existing" logic (lines ~1514-1597)**

For terminal launch, remove the `existing` check and always create new:

```rust
Some(LauncherAction::Launch(AppId::Terminal)) => {
    launcher.visible = false;

    let lw = (size.width as f32 / scale_factor) as u32;
    let lh = (size.height as f32 / scale_factor) as u32;
    let viewport = ShellLayout::calculate(lw, lh).app_viewport;

    let bounds = window_bounds_from_viewport(viewport);
    let mut aw = AppWindow::new("Terminal", bounds);
    let cr = aw.content_rect();
    let term_cfg = TerminalConfig::default();
    let renderer = TerminalRenderer::new(provider.as_ref(), term_cfg);
    let cols = renderer.metrics.cols_for_width(cr.w, renderer.cfg.padding);
    let rows = renderer.metrics.rows_for_height(cr.h, renderer.cfg.padding);

    if let Ok(core) = TerminalCore::new(cols, rows, cwd) {
        aw.toggle_maximize(viewport);
        let (_, content) = app_thread::create_terminal(core, renderer);
        let id = wm.add("terminal", aw, content);
        Some(id)
    } else {
        None
    }
}
```

For browser launch, same — always create new:

```rust
Some(LauncherAction::Launch(AppId::DetirBrowser)) => {
    launcher.visible = false;

    let lw = (size.width as f32 / scale_factor) as u32;
    let lh = (size.height as f32 / scale_factor) as u32;
    let viewport = ShellLayout::calculate(lw, lh).app_viewport;

    let bounds = window_bounds_from_viewport(viewport);
    let mut aw = AppWindow::new_with_address_bar("Detir Browser", bounds);
    match detir_scene::SceneRenderer::new_home(provider.clone(), bounds.w, bounds.h) {
        Ok(mut scene) => {
            scene.set_chrome_hidden(true);
            scene.set_dock_disabled(true);
            aw.toggle_maximize(viewport);
            let (_, content) = app_thread::create_browser(scene);
            let id = wm.add("detir-browser", aw, content);
            Some(id)
        }
        Err(e) => {
            log::error!("Failed to create IR SceneRenderer: {e}");
            None
        }
    }
}
```

**Step 5: Build to check progress**

Run: `cargo build -p orus-macos 2>&1 | head -50`
Expected: errors in rendering and input routing (fixed in next tasks), but creation code should be correct.

**Step 6: Commit**

```bash
git add orus-universal/crates/orus-macos/src/main.rs
git commit -m "feat(orus): update window creation and launcher for multi-instance threading"
```

---

### Task 7: Update main.rs — rendering loop

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Update RedrawRequested handler (lines ~1211-1262)**

Replace the per-window rendering match block:

```rust
for mw in wm.iter_back_to_front_mut() {
    mw.app_window.render(&mut canvas, provider.as_ref());

    match &mut mw.content {
        WindowContent::Terminal { state, thread } => {
            let s = state.lock().unwrap();
            let cr = mw.app_window.content_rect();
            s.renderer.render(&s.core, &mut canvas, cr, mw.app_window.z_base + 10);
            if s.core.is_alive() || thread.flags.needs_redraw.load(Ordering::Relaxed) {
                needs_redraw = true;
            }
        }
        WindowContent::IrScene {
            state,
            address_bar_focused: ab_focused,
            thread,
        } => {
            // Tick address bar caret blink (main thread UI state)
            if *ab_focused {
                if let Some(ref mut ab) = mw.app_window.address_bar {
                    if ab.input.is_focused() {
                        ab.input.update_blink(1.0 / 60.0);
                        needs_redraw = true;
                    }
                }
            }

            let mut s = state.lock().unwrap();

            // Sync navigation state to address bar
            if let Some(ref mut ab) = mw.app_window.address_bar {
                let nav = &s.scene.nav_ctx().state;
                ab.can_go_back = nav.can_go_back;
                ab.can_go_forward = nav.can_go_forward;
                if !ab.input.is_focused() {
                    let current = nav.current_url.as_deref().unwrap_or("");
                    if ab.text() != current {
                        ab.set_text(current);
                    }
                }
            }

            let cr = mw.app_window.content_rect();
            if let Err(e) = s.scene.render(&mut canvas, cr, mw.app_window.z_base + 10) {
                log::error!("IR scene render error: {e}");
            }

            if thread.flags.needs_redraw.swap(false, Ordering::Relaxed) {
                needs_redraw = true;
            }
        }
    }
}
```

**Step 2: Update Canvas3D post-processing (lines ~1279-1284)**

```rust
for mw in wm.iter_back_to_front_mut() {
    if let WindowContent::IrScene { ref state, .. } = mw.content {
        let mut s = state.lock().unwrap();
        s.scene.process_canvas3d(&mut surf);
    }
}
```

**Step 3: Update needs_redraw check at end**

Add check for any app thread redraw signals:

```rust
// Also check if any app threads signaled redraw
let any_app_needs_redraw = wm.windows().any(|mw| mw.content.check_redraw());
if needs_redraw || any_app_needs_redraw || launcher.needs_redraw() || dock.visible {
    window.request_redraw();
}
```

Note: `check_redraw()` does an atomic swap so this won't double-trigger. But since we already clear flags in the render loop above, we can simplify to just keeping the existing `needs_redraw` bool from the loop.

**Step 4: Commit**

```bash
git add orus-universal/crates/orus-macos/src/main.rs
git commit -m "feat(orus): update rendering loop to use shared state with mutex locks"
```

---

### Task 8: Update main.rs — input routing

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Update keyboard routing to terminal (lines ~1102-1134)**

Replace direct `core.send_key(k)` with channel send:

```rust
WindowContent::Terminal { .. } => {
    let ctrl = current_modifiers.state().control_key();
    let key = match event.logical_key.as_ref() {
        Key::Named(NamedKey::Enter) => Some(TerminalKey::Enter),
        Key::Named(NamedKey::Backspace) => Some(TerminalKey::Backspace),
        Key::Named(NamedKey::Tab) => Some(TerminalKey::Tab),
        Key::Named(NamedKey::Escape) => Some(TerminalKey::Escape),
        Key::Named(NamedKey::ArrowUp) => Some(TerminalKey::ArrowUp),
        Key::Named(NamedKey::ArrowDown) => Some(TerminalKey::ArrowDown),
        Key::Named(NamedKey::ArrowLeft) => Some(TerminalKey::ArrowLeft),
        Key::Named(NamedKey::ArrowRight) => Some(TerminalKey::ArrowRight),
        Key::Named(NamedKey::Home) => Some(TerminalKey::Home),
        Key::Named(NamedKey::End) => Some(TerminalKey::End),
        Key::Named(NamedKey::PageUp) => Some(TerminalKey::PageUp),
        Key::Named(NamedKey::PageDown) => Some(TerminalKey::PageDown),
        Key::Named(NamedKey::Delete) => Some(TerminalKey::Delete),
        Key::Named(NamedKey::Space) => Some(TerminalKey::Char(' ')),
        Key::Character(ch) => {
            if ctrl {
                ch.chars().next().map(TerminalKey::Ctrl)
            } else {
                ch.chars().next().map(TerminalKey::Char)
            }
        }
        _ => None,
    };
    if let Some(k) = key {
        mw.content.send_input(AppInput::TerminalKey(k));
        window.request_redraw();
    }
}
```

**Step 2: Update keyboard routing to IR scene (lines ~1136-1178)**

For scene key input (non-address-bar), replace direct `scene.handle_key(...)` with channel:

```rust
} else {
    let key_code = winit_to_platform_key(&event.physical_key);
    let mods = winit_to_platform_modifiers(&current_modifiers);
    let text_str = match event.logical_key.as_ref() {
        Key::Character(ch) => Some(ch.to_string()),
        Key::Named(NamedKey::Space) => Some(" ".to_string()),
        _ => None,
    };
    mw.content.send_input(AppInput::SceneKey {
        code: key_code,
        state: detir_platform::ElementState::Pressed,
        mods,
        text: text_str,
    });
    window.request_redraw();
}
```

For `address_bar_focused` access — it's still directly on `WindowContent::IrScene { address_bar_focused, .. }`, which is main-thread state. The `handle_address_bar_key` function needs scene access for navigation. Update it to use channels:

In `handle_address_bar_key`, where it calls `scene.navigate(url)`, `scene.load_content(url)`, etc., replace with `send_input(AppInput::Navigate(url))`, `send_input(AppInput::NavBack)`, etc.

**Step 3: Update mouse input routing**

For IR scene left-click (lines ~807-846), replace direct `scene.handle_mouse_input(...)`:

```rust
if let WindowContent::IrScene { .. } = &mw.content {
    // Check address_bar_focused
    let is_ab_focused = matches!(
        &mw.content,
        WindowContent::IrScene { address_bar_focused: true, .. }
    );
    if !is_ab_focused {
        let cr = mw.app_window.content_rect();
        let in_content = mx >= cr.x && mx < cr.x + cr.w && my >= cr.y && my < cr.y + cr.h;
        // For releasing_drag, we need to check scene.is_dragging() — lock mutex briefly
        let is_dragging = {
            if let WindowContent::IrScene { state, .. } = &mw.content {
                let s = state.lock().unwrap();
                s.scene.is_dragging()
            } else {
                false
            }
        };
        let releasing_drag = state == ElementState::Released && is_dragging;
        if in_content || releasing_drag {
            let dp_state = if state == ElementState::Pressed {
                detir_platform::ElementState::Pressed
            } else {
                detir_platform::ElementState::Released
            };
            mw.content.send_input(AppInput::SceneMouseInput {
                button: detir_platform::MouseButton::Left,
                state: dp_state,
                x: mx,
                y: my,
                content_rect: cr,
            });
            window.request_redraw();
            ir_consumed = true;
        }
    }
}
```

**Step 4: Update mouse move routing (lines ~565-577)**

```rust
if !launcher.visible
    && !dock.visible
    && let Some(mw) = wm.focused_mut()
    && matches!(&mw.content, WindowContent::IrScene { .. })
{
    let cr = mw.app_window.content_rect();
    let in_content = lx >= cr.x && lx < cr.x + cr.w && ly >= cr.y && ly < cr.y + cr.h;
    let is_dragging = if let WindowContent::IrScene { state, .. } = &mw.content {
        let s = state.lock().unwrap();
        s.scene.is_dragging()
    } else {
        false
    };
    if in_content || is_dragging {
        mw.content.send_input(AppInput::SceneMouseMove {
            x: lx,
            y: ly,
            content_rect: cr,
        });
        window.request_redraw();
    }
}
```

**Step 5: Update scroll routing (lines ~988-1036)**

For IR scene scroll, send through channel:

```rust
WindowContent::IrScene { .. } => {
    let cr = mw.app_window.content_rect();
    let (dx, dy) = match delta { /* same as before */ };
    mw.content.send_input(AppInput::Scroll {
        dx, dy, mx, my, content_rect: cr,
    });
    window.request_redraw();
}
```

For terminal scroll, send through channel:

```rust
WindowContent::Terminal { state, .. } => {
    let s = state.lock().unwrap();
    let ch = s.renderer.metrics.cell_height;
    drop(s); // Release lock before sending
    match delta {
        winit::event::MouseScrollDelta::LineDelta(_, y) => {
            let n = y.abs().max(1.0) as usize;
            if y > 0.0 {
                mw.content.send_input(AppInput::TerminalScrollUp(n));
            } else if y < 0.0 {
                mw.content.send_input(AppInput::TerminalScrollDown(n));
            }
        }
        winit::event::MouseScrollDelta::PixelDelta(pos) => {
            scroll_accum += pos.y as f32;
            let lines = (scroll_accum / ch).trunc() as i32;
            if lines != 0 {
                scroll_accum -= lines as f32 * ch;
                if lines > 0 {
                    mw.content.send_input(AppInput::TerminalScrollUp(lines as usize));
                } else {
                    mw.content.send_input(AppInput::TerminalScrollDown((-lines) as usize));
                }
            }
        }
    }
    window.request_redraw();
}
```

**Step 6: Update resize handling (lines ~453-463, ~504-513, ~521-530)**

For terminal resize during window resize/drag, send through channel:

```rust
if let WindowContent::Terminal { state, .. } = &mw.content {
    let s = state.lock().unwrap();
    let cr = mw.app_window.content_rect();
    let cols = s.renderer.metrics.cols_for_width(cr.w, s.renderer.cfg.padding);
    let rows = s.renderer.metrics.rows_for_height(cr.h, s.renderer.cfg.padding);
    drop(s);
    mw.content.send_input(AppInput::TerminalResize { cols, rows });
}
```

**Step 7: Update right/middle mouse (Canvas3D pan, lines ~950-981)**

Same pattern as left click — send `AppInput::SceneMouseInput` with the appropriate button.

**Step 8: Update address bar button handlers (lines ~722-766)**

Replace direct `scene.show_dock()`, `scene.nav_ctx_mut().ir_go_back()`, etc. with:

```rust
ToolbarButton::Home => {
    mw.content.send_input(AppInput::ShowDock);
}
ToolbarButton::Back => {
    mw.content.send_input(AppInput::NavBack);
}
ToolbarButton::Forward => {
    mw.content.send_input(AppInput::NavForward);
}
ToolbarButton::Refresh => {
    mw.content.send_input(AppInput::NavReload);
}
```

**Step 9: Build and fix remaining compile errors**

Run: `cargo build -p orus-macos`
Fix any remaining match arm issues. The main pattern is: where old code matched `WindowContent::Terminal { core, renderer }`, new code matches `WindowContent::Terminal { state, .. }` and locks the mutex if needed.

**Step 10: Commit**

```bash
git add orus-universal/crates/orus-macos/src/main.rs
git commit -m "feat(orus): route all input through channels to app threads"
```

---

### Task 9: Update close/cleanup and session persistence

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Update close button handler**

When a window is closed (lines ~873-882), shut down the thread before removing:

```rust
WindowButton::Close => {
    // Shutdown app thread before removing
    if let Some(mw) = wm.get_mut(target_id) {
        match &mut mw.content {
            WindowContent::Terminal { thread, .. }
            | WindowContent::IrScene { thread, .. } => {
                app_thread::shutdown_thread(thread);
            }
        }
    }
    wm.remove(target_id);
    // ... rest unchanged
}
```

**Step 2: Update CloseRequested / LoopExiting**

Shut down all app threads on quit:

```rust
WindowEvent::CloseRequested => {
    persistence::save(&win_state);
    let session_state = wm.snapshot_session();
    session::save(&session_state);

    // Save per-app state
    for mw in wm.windows() {
        match &mw.content {
            WindowContent::Terminal { .. } => {
                let cwd = std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
                session::save_app_state(&mw.app_id, &TerminalAppState { cwd });
            }
            WindowContent::IrScene { .. } => {
                let url = mw.app_window.address_bar.as_ref()
                    .map(|ab| ab.text().to_string())
                    .filter(|u| !u.is_empty());
                session::save_app_state(&mw.app_id, &BrowserAppState { url });
            }
        }
    }

    // Shutdown all app threads
    for mw in wm.iter_back_to_front_mut() {
        match &mut mw.content {
            WindowContent::Terminal { thread, .. }
            | WindowContent::IrScene { thread, .. } => {
                app_thread::shutdown_thread(thread);
            }
        }
    }

    elwt.exit();
}
```

**Step 3: Handle dead app detection in render loop**

After the render loop, clean up windows whose apps have exited:

```rust
// After rendering loop, check for exited apps
let exited_ids: Vec<WindowId> = wm.windows()
    .filter(|mw| mw.content.has_exited())
    .map(|mw| mw.id)
    .collect();
for id in exited_ids {
    if let Some(mw) = wm.get_mut(id) {
        match &mut mw.content {
            WindowContent::Terminal { thread, .. }
            | WindowContent::IrScene { thread, .. } => {
                app_thread::shutdown_thread(thread);
            }
        }
    }
    wm.remove(id);
    if let Some(mw) = wm.focused_ref() {
        shell_bar.app_name = mw.content.label().to_string();
    }
}
```

**Step 4: Build and test**

Run: `cargo build -p orus-macos`
Expected: compiles

Run: `cargo run -p orus-macos` (from orus-universal/)
Test: Open launcher (Cmd+P), launch multiple terminals and browsers. Verify:
- Each terminal has its own shell instance
- Each browser has its own navigation state
- Closing one doesn't affect others
- Focus switching works (Ctrl+Tab, click)

**Step 5: Commit**

```bash
git add orus-universal/crates/orus-macos/src/main.rs
git commit -m "feat(orus): add thread cleanup on close and dead app detection"
```

---

### Task 10: Fix Send bounds and address bar key handler

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/app_thread.rs`
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Add unsafe Send impls if needed**

If `cargo build` fails with "`TerminalCore` cannot be sent between threads safely" or similar:

```rust
// app_thread.rs — these types are protected by Mutex, ensuring exclusive access
unsafe impl Send for TerminalState {}
unsafe impl Send for BrowserState {}
```

This is safe because:
- Access is always serialized through `Mutex::lock()`
- The types contain no thread-local state
- GPU resources are NOT in these types (they're in Canvas/DetirSurface on main thread)

**Step 2: Update `handle_address_bar_key`**

This function currently takes `&mut SceneRenderer` directly. Update it to accept `&WindowContent` or just use `send_input` for navigation:

Where the function calls `scene.navigate(url)` on Enter, replace with sending through the content's channel. The function signature may need to change to accept `&WindowContent` instead of `scene: &mut SceneRenderer`.

Alternatively, keep the function taking scene but lock the mutex when calling it. Since address bar is main-thread UI, it's simplest to pass the sender:

```rust
fn handle_address_bar_key(
    event: &winit::event::KeyEvent,
    modifiers: &Modifiers,
    address_bar: &mut Option<orus_shell::address_bar::AddressBar>,
    content: &WindowContent,  // Changed from scene
    address_bar_focused: &mut bool,
) {
    // ... existing key handling for InputBox ...
    // On Enter (navigate):
    if let Some(ref ab) = address_bar {
        let url = ab.text().to_string();
        content.send_input(AppInput::Navigate(url));
        *address_bar_focused = false;
    }
}
```

**Step 3: Build and run final verification**

Run: `cargo build -p orus-macos`
Run: `cargo run -p orus-macos`

Manual test checklist:
- [ ] Launch 2+ terminals via Cmd+P
- [ ] Each terminal runs its own shell
- [ ] Type in one terminal, other is unaffected
- [ ] Launch 2+ browsers via Cmd+P
- [ ] Navigate to different URLs in each
- [ ] Close individual windows (red button)
- [ ] Ctrl+Tab cycles through all windows
- [ ] Dock (Cmd+D) shows all windows
- [ ] Quit (Cmd+Q / close) cleanly shuts down all threads
- [ ] Resize works for terminals (grid reflows)
- [ ] Scroll works in both terminals and browsers

**Step 4: Commit**

```bash
git add -A orus-universal/
git commit -m "feat(orus): multi-instance per-app threading complete"
```

---

## Notes

### Threading Model Summary
- **Main thread:** Window chrome, GPU rendering, hit-testing, focus management
- **Per-app thread:** PTY polling (terminal), HTTP/resource loading (browser), input processing, animation updates
- **Communication:** `mpsc::Sender<AppInput>` (main→app), `Arc<AtomicBool>` flags (app→main), `Arc<Mutex<State>>` (shared for rendering)

### Known Limitations
- Main thread locks mutex during render (~1ms per window). With many windows, this could add up. Acceptable for <10 windows.
- Browser navigation (HTML parsing) holds the mutex. During a heavy page load, rendering of that window may briefly stutter. Can be optimized later by splitting parse/layout from render state.
- `scroll_accum` is per-compositor, not per-window. If multiple terminals need pixel-scroll tracking, this should be moved to per-window state.
