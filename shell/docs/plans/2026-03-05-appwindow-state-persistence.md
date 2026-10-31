# AppWindow State Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Persist internal AppWindow layout and app-specific state across Orus OS sessions.

**Architecture:** Two-layer persistence — session layer (`~/.orus/session.json`) stores window layout owned by WindowManager; app layer (`~/.orus/apps/<app_id>/state.json`) stores content state owned by each app. Save on exit, load on startup, fallback to hardcoded defaults.

**Tech Stack:** Rust, serde + serde_json (already workspace deps), engine-core::Rect

---

### Task 1: Add serde deps to orus-shell

**Files:**
- Modify: `orus-universal/crates/orus-shell/Cargo.toml`

**Step 1: Add dependencies**

Add `serde` and `serde_json` to `[dependencies]`:

```toml
serde = { workspace = true }
serde_json = { workspace = true }
```

**Step 2: Verify it compiles**

Run: `cd <orus-universal checkout> && cargo check -p orus-shell`
Expected: compiles successfully

**Step 3: Commit**

```bash
git add crates/orus-shell/Cargo.toml
git commit -m "feat(orus-shell): add serde deps for session persistence"
```

---

### Task 2: Create session persistence module in orus-shell

**Files:**
- Create: `orus-universal/crates/orus-shell/src/session.rs`
- Modify: `orus-universal/crates/orus-shell/src/lib.rs`

**Step 1: Create `session.rs` with types and save/load**

```rust
//! Session persistence for Orus OS internal app windows.
//!
//! Saves/restores the internal window layout (bounds, maximized state,
//! focus) between sessions using `~/.orus/session.json`.
//! App-specific content state is NOT stored here — each app manages
//! its own state in `~/.orus/apps/<app_id>/state.json`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Serializable bounds (mirrors engine_core::Rect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Persisted state for a single internal app window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowEntry {
    /// App identifier (e.g., "terminal", "detir-browser").
    pub app_id: String,
    /// Window bounds (outer frame).
    pub bounds: Bounds,
    /// Whether the window was maximized.
    pub maximized: bool,
}

/// Top-level session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// The app_id of the focused window (if any).
    pub focused_app_id: Option<String>,
    /// All open windows, in z-order (back to front).
    pub windows: Vec<WindowEntry>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            version: 1,
            focused_app_id: None,
            windows: Vec::new(),
        }
    }
}

fn orus_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".orus")
}

fn session_path() -> PathBuf {
    orus_dir().join("session.json")
}

/// Directory for a specific app's state.
pub fn app_state_dir(app_id: &str) -> PathBuf {
    orus_dir().join("apps").join(app_id)
}

/// Load saved session state, or `None` if no file exists or it's invalid.
pub fn load() -> Option<SessionState> {
    let path = session_path();
    let data = std::fs::read_to_string(&path).ok()?;
    let state: SessionState = serde_json::from_str(&data).ok()?;
    if state.version != 1 {
        log::warn!("Unknown session version {}, ignoring", state.version);
        return None;
    }
    Some(state)
}

/// Save session state to disk. Creates `~/.orus/` if needed.
pub fn save(state: &SessionState) {
    let dir = orus_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("Failed to create {}: {e}", dir.display());
        return;
    }
    let path = session_path();
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                log::warn!("Failed to write {}: {e}", path.display());
            }
        }
        Err(e) => log::warn!("Failed to serialize session state: {e}"),
    }
}

/// Load app-specific state from `~/.orus/apps/<app_id>/state.json`.
pub fn load_app_state<T: serde::de::DeserializeOwned>(app_id: &str) -> Option<T> {
    let path = app_state_dir(app_id).join("state.json");
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save app-specific state to `~/.orus/apps/<app_id>/state.json`.
pub fn save_app_state<T: Serialize>(app_id: &str, state: &T) {
    let dir = app_state_dir(app_id);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("Failed to create {}: {e}", dir.display());
        return;
    }
    let path = dir.join("state.json");
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                log::warn!("Failed to write {}: {e}", path.display());
            }
        }
        Err(e) => log::warn!("Failed to serialize app state for {app_id}: {e}"),
    }
}
```

**Step 2: Export the module in `lib.rs`**

Add to `orus-universal/crates/orus-shell/src/lib.rs`:

```rust
pub mod session;
```

**Step 3: Verify it compiles**

Run: `cd <orus-universal checkout> && cargo check -p orus-shell`
Expected: compiles successfully

**Step 4: Commit**

```bash
git add crates/orus-shell/src/session.rs crates/orus-shell/src/lib.rs
git commit -m "feat(orus-shell): add session persistence module"
```

---

### Task 3: Add app_id to ManagedWindow

The window manager needs to know which app type each window is so it can serialize the session. Add an `app_id` field to `ManagedWindow`.

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/window_manager.rs`

**Step 1: Add `app_id` field to `ManagedWindow`**

At `window_manager.rs:17-21`, change:

```rust
pub struct ManagedWindow {
    pub id: WindowId,
    pub app_id: String,
    pub app_window: AppWindow,
    pub content: WindowContent,
}
```

**Step 2: Update `WindowManager::add` to accept `app_id`**

At `window_manager.rs:72-81`, change signature:

```rust
pub fn add(
    &mut self,
    app_id: impl Into<String>,
    app_window: AppWindow,
    content: WindowContent,
) -> WindowId {
    let id = WindowId(self.next_id);
    self.next_id += 1;
    self.windows.push(ManagedWindow {
        id,
        app_id: app_id.into(),
        app_window,
        content,
    });
    id
}
```

**Step 3: Verify it compiles (expect errors in main.rs)**

Run: `cd <orus-universal checkout> && cargo check -p orus-shell`
Expected: orus-shell compiles; orus-macos will fail (fixed in Task 5)

**Step 4: Commit**

```bash
git add crates/orus-shell/src/window_manager.rs
git commit -m "feat(orus-shell): add app_id to ManagedWindow"
```

---

### Task 4: Add session snapshot/restore helpers to WindowManager

**Files:**
- Modify: `orus-universal/crates/orus-shell/src/window_manager.rs`

**Step 1: Add snapshot method**

Add these imports at the top of `window_manager.rs`:

```rust
use crate::session::{Bounds, SessionState, WindowEntry};
```

Add to `impl WindowManager`:

```rust
/// Snapshot current window layout into a `SessionState` for persistence.
pub fn snapshot_session(&self) -> SessionState {
    let focused_app_id = self
        .focused
        .and_then(|id| self.get(id))
        .map(|mw| mw.app_id.clone());

    let windows: Vec<WindowEntry> = self
        .iter_back_to_front()
        .map(|mw| WindowEntry {
            app_id: mw.app_id.clone(),
            bounds: Bounds {
                x: mw.app_window.bounds.x,
                y: mw.app_window.bounds.y,
                w: mw.app_window.bounds.w,
                h: mw.app_window.bounds.h,
            },
            maximized: mw.app_window.is_maximized(),
        })
        .collect();

    SessionState {
        version: 1,
        focused_app_id,
        windows,
    }
}
```

**Step 2: Verify it compiles**

Run: `cd <orus-universal checkout> && cargo check -p orus-shell`
Expected: compiles

**Step 3: Commit**

```bash
git add crates/orus-shell/src/window_manager.rs
git commit -m "feat(orus-shell): add session snapshot to WindowManager"
```

---

### Task 5: Wire persistence into main.rs — save on exit

Update `orus-macos/src/main.rs` to (a) pass `app_id` to `wm.add()`, (b) save session + app state on exit.

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Fix all `wm.add()` call sites to include `app_id`**

There are multiple `wm.add()` calls. Each needs an `app_id` string as the first argument:

- Terminal windows: `wm.add("terminal", term_aw, WindowContent::Terminal { ... })`
- Browser/IR windows: `wm.add("detir-browser", ir_aw, WindowContent::IrScene { ... })`
- IR app windows (from launcher): `wm.add("detir-browser", aw, WindowContent::IrScene { ... })`

Search for all `wm.add(` calls and prepend the appropriate `app_id`.

**Step 2: Add app state structs**

Add near the top of `main.rs` (after the imports):

```rust
use orus_shell::session;

/// Terminal app state (saved to ~/.orus/apps/terminal/state.json).
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct TerminalAppState {
    cwd: Option<String>,
}

/// Browser app state (saved to ~/.orus/apps/detir-browser/state.json).
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct BrowserAppState {
    url: Option<String>,
}
```

**Step 3: Save session + app state on CloseRequested**

Replace the `CloseRequested` handler (around line 262-265):

```rust
WindowEvent::CloseRequested => {
    // Save outer window state (existing)
    persistence::save(&win_state);

    // Save internal window session
    let session_state = wm.snapshot_session();
    session::save(&session_state);

    // Save per-app state
    for mw in wm.windows() {
        match &mw.content {
            WindowContent::Terminal { .. } => {
                // Terminal CWD: best-effort from /proc or env
                let cwd = std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
                session::save_app_state(
                    &mw.app_id,
                    &TerminalAppState { cwd },
                );
            }
            WindowContent::IrScene { .. } => {
                let url = mw
                    .app_window
                    .address_bar
                    .as_ref()
                    .map(|ab| ab.text().to_string())
                    .filter(|u| !u.is_empty());
                session::save_app_state(
                    &mw.app_id,
                    &BrowserAppState { url },
                );
            }
        }
    }

    elwt.exit();
}
```

**Step 4: Verify it compiles**

Run: `cd <orus-universal checkout> && cargo check -p orus-macos`
Expected: compiles

**Step 5: Commit**

```bash
git add crates/orus-macos/src/main.rs
git commit -m "feat(orus-macos): save session and app state on exit"
```

---

### Task 6: Wire persistence into main.rs — restore on startup

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Load session and create windows from saved state**

Replace the hardcoded window creation block (lines ~168-242) with session-aware logic. The key change: after `let mut wm = WindowManager::new();`, try to load the session and create windows from it, falling back to the current hardcoded defaults.

```rust
// ── WindowManager setup ──────────────────────────────────────────
let mut wm = WindowManager::new();
let mut dock = OrusDock::new();

let logical_w = size.width as f32 / scale_factor;
let logical_h = size.height as f32 / scale_factor;
let layout = ShellLayout::calculate(logical_w as u32, logical_h as u32);
let vp = layout.app_viewport;

// Try to restore session; fall back to defaults
let saved_session = session::load();
let mut browser_id: Option<WindowId> = None;
let mut term_id: Option<WindowId> = None;

if let Some(ref session_state) = saved_session {
    for entry in &session_state.windows {
        let bounds = Rect {
            x: entry.bounds.x,
            y: entry.bounds.y,
            w: entry.bounds.w,
            h: entry.bounds.h,
        };
        match entry.app_id.as_str() {
            "terminal" => {
                let app_state: TerminalAppState =
                    session::load_app_state("terminal").unwrap_or_default();
                let term_cfg = TerminalConfig::default();
                let term_renderer = TerminalRenderer::new(provider.as_ref(), term_cfg);
                let aw = AppWindow::new("Terminal", bounds);
                let cr = aw.content_rect();
                let cols = term_renderer
                    .metrics
                    .cols_for_width(cr.w, term_renderer.cfg.padding);
                let rows = term_renderer
                    .metrics
                    .rows_for_height(cr.h, term_renderer.cfg.padding);
                let cwd_path = app_state
                    .cwd
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| {
                        std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                    });
                match TerminalCore::new(cols, rows, &cwd_path) {
                    Ok(core) => {
                        let id = wm.add(
                            "terminal",
                            aw,
                            WindowContent::Terminal {
                                core,
                                renderer: term_renderer,
                            },
                        );
                        term_id = Some(id);
                    }
                    Err(e) => log::warn!("Failed to restore terminal: {e}"),
                }
            }
            "detir-browser" => {
                let app_state: BrowserAppState =
                    session::load_app_state("detir-browser").unwrap_or_default();
                let mut aw = AppWindow::new_with_address_bar("Detir Browser", bounds);

                // Determine whether to load a URL or go home
                let scene_result = if let Some(ref url) = app_state.url {
                    if !url.is_empty()
                        && !url.starts_with("detir://home")
                    {
                        aw.address_bar.as_mut().map(|ab| ab.set_text(url));
                        detir_scene::SceneRenderer::new(
                            url,
                            provider.clone(),
                            bounds.w,
                            bounds.h,
                        )
                    } else {
                        detir_scene::SceneRenderer::new_home(
                            provider.clone(),
                            bounds.w,
                            bounds.h,
                        )
                    }
                } else {
                    detir_scene::SceneRenderer::new_home(
                        provider.clone(),
                        bounds.w,
                        bounds.h,
                    )
                };

                match scene_result {
                    Ok(mut scene) => {
                        scene.set_chrome_hidden(true);
                        scene.set_dock_disabled(true);
                        let id = wm.add(
                            "detir-browser",
                            aw,
                            WindowContent::IrScene {
                                scene,
                                address_bar_focused: false,
                            },
                        );
                        browser_id = Some(id);
                    }
                    Err(e) => log::warn!("Failed to restore browser: {e}"),
                }
            }
            other => {
                log::warn!("Unknown app_id in session: {other}, skipping");
            }
        }
    }

    // Restore focus
    if let Some(ref focused_id) = session_state.focused_app_id {
        for mw in wm.windows() {
            if mw.app_id == *focused_id {
                let id = mw.id;
                wm.focus(id);
                break;
            }
        }
    }
}

// Fallback: create default windows if session was empty or missing
if wm.is_empty() {
    // Create terminal window (existing default logic)
    let term_cfg = TerminalConfig::default();
    let term_renderer = TerminalRenderer::new(provider.as_ref(), term_cfg);
    let term_bounds = window_bounds_from_viewport(vp);
    let term_aw = AppWindow::new("Terminal", term_bounds);
    let term_cr = term_aw.content_rect();
    let init_cols = term_renderer
        .metrics
        .cols_for_width(term_cr.w, term_renderer.cfg.padding);
    let init_rows = term_renderer
        .metrics
        .rows_for_height(term_cr.h, term_renderer.cfg.padding);
    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let term_core = TerminalCore::new(init_cols, init_rows, &cwd)
        .expect("Failed to create terminal");
    term_id = Some(wm.add(
        "terminal",
        term_aw,
        WindowContent::Terminal {
            core: term_core,
            renderer: term_renderer,
        },
    ));

    // Create browser window
    let ir_w = 800.0_f32.min(vp.w - 40.0).max(300.0);
    let ir_h = 600.0_f32.min(vp.h - 40.0).max(200.0);
    let ir_bounds = Rect {
        x: vp.x + (vp.w - ir_w) / 2.0,
        y: vp.y + (vp.h - ir_h) / 2.0,
        w: ir_w,
        h: ir_h,
    };
    let ir_aw = AppWindow::new_with_address_bar("Detir Browser", ir_bounds);
    if let Ok(mut scene) =
        detir_scene::SceneRenderer::new_home(provider.clone(), ir_bounds.w, ir_bounds.h)
    {
        scene.set_chrome_hidden(true);
        scene.set_dock_disabled(true);
        browser_id = Some(wm.add(
            "detir-browser",
            ir_aw,
            WindowContent::IrScene {
                scene,
                address_bar_focused: false,
            },
        ));
    }

    // Focus browser by default
    if let Some(bid) = browser_id {
        wm.focus(bid);
    }
}
```

**Step 2: Remove the old hardcoded variables**

The old `let term_id = ...;` and `let browser_id = ...;` are now `Option<WindowId>`. Update any downstream usage of `term_id` and `browser_id` to handle `Option`. The existing code only uses `let _ = term_id;` which can be removed.

**Step 3: Update shell_bar init**

The shell bar currently hardcodes `"Detir Browser"`. If there's a focused window from the session, use its title:

```rust
let initial_title = wm
    .focused_ref()
    .map(|mw| mw.app_window.title.as_str())
    .unwrap_or("Orus OS");
let mut shell_bar = ShellBar::new(initial_title);
```

**Step 4: Verify it compiles and runs**

Run: `cd <orus-universal checkout> && cargo build -p orus-macos`
Expected: compiles successfully

Run: `cargo run -p orus-macos`
Expected: launches normally (no session file yet = fallback to defaults)

**Step 5: Commit**

```bash
git add crates/orus-macos/src/main.rs
git commit -m "feat(orus-macos): restore session and app state on startup"
```

---

### Task 7: Fix remaining wm.add() calls in launcher/dock handlers

**Files:**
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Find and fix all remaining `wm.add()` calls**

Search for all `wm.add(` in `main.rs`. The launcher action handlers (around lines 1360-1450) create windows dynamically. Each needs the `app_id` parameter:

- Terminal creation in launcher: `wm.add("terminal", aw, ...)`
- Browser creation in launcher: `wm.add("detir-browser", aw, ...)`
- IR app creation from URL: `wm.add("detir-browser", aw, ...)`

**Step 2: Verify it compiles**

Run: `cd <orus-universal checkout> && cargo check -p orus-macos`
Expected: no errors

**Step 3: Commit**

```bash
git add crates/orus-macos/src/main.rs
git commit -m "fix(orus-macos): add app_id to all dynamic wm.add calls"
```

---

### Task 8: End-to-end verification

**Step 1: Build and run**

```bash
cd <orus-universal checkout>
cargo build -p orus-macos && cargo run -p orus-macos
```

**Step 2: Test persistence round-trip**

1. Move/resize the terminal and browser windows
2. Navigate the browser to a URL
3. Close the app (Cmd+Q or window close)
4. Verify `~/.orus/session.json` exists with correct bounds
5. Verify `~/.orus/apps/terminal/state.json` and `~/.orus/apps/detir-browser/state.json` exist
6. Re-launch — windows should appear at saved positions with saved content

**Step 3: Test first-run fallback**

1. Delete `~/.orus/session.json`
2. Re-launch — should create default windows (centered terminal + browser)

**Step 4: Lint**

```bash
cargo fmt -p orus-shell -p orus-macos
cargo clippy -p orus-shell -p orus-macos --no-deps -- -D warnings
```

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix(orus): lint and end-to-end session persistence fixes"
```
