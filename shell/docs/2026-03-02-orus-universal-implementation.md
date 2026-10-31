# Orus OS Universal — Implementation Plan (Revised)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build Orus OS Universal — the full Orus OS experience as a sandboxed native desktop application, **built on the existing Detir stack**.

**Architecture:** Orus OS Universal is NOT a rewrite. It's a **composition layer** on top of existing Detir crates. The existing `detir-scene` already provides: window management (via winit), wgpu rendering, IR renderer, zone layout (toolbar, sidebar, viewport, dock, devtools), CEF hosting, Peco integration, input handling, hit-testing, and the full rendering pipeline. Orus OS Universal adds: multi-app compositing, shell UI, sandbox enforcement, agent protocol server, and app packaging.

**Tech Stack:** Existing Detir crates + new Orus OS-specific crates for sandbox, agent protocol, multi-app, and shell.

**Spec:** `docs/orus/orus-universal-technical-spec.md`

---

## Progress Checklist

> **Note:** Implementation diverged from plan — using engine-core + detir-surface directly (compositor-owned decorations) instead of wrapping detir-scene's ZoneManager. This gives full control over window chrome rendering.

### Phase 0: Project Structure

- [x] Task 0.1: Workspace structure with detir path deps (`e5d1517`)

### Phase 1: Single-App Orus OS Shell (~95% complete)

**Render loop & shell chrome:**

- [x] Launcher binary with wgpu + Canvas render loop (`e5d1517`)
- [x] Shell bar — 28px top strip, dark bg, bottom border separator (`e5d1517`)
- [x] Shell bar — app name (left), clock placeholder (center), agent status + "Orus OS" branding (right) (`e5d1517`)
- [x] Shell layout — splits window into shell_bar (28px) + app_viewport (`e5d1517`)

**App window decorations:**

- [x] AppWindow struct — title, bounds, focused, z_base (`1d9301e`)
- [x] Rounded rect body + 1px border (top/bottom/left/right) (`1d9301e`)
- [x] Title bar — background (focused/unfocused), bottom separator (`1d9301e`)
- [x] Traffic light buttons — close (red), minimize (yellow), maximize (green) with circle rendering (`1d9301e`)
- [x] Title text — centered in title bar, 13px medium weight (`1d9301e`)

**Window interaction:**

- [x] Title bar drag to move window (`1d9301e`)
- [x] 8-direction edge resize with 5px grab zone + min-size clamping (`1d9301e`)
- [x] Cursor icon caching — resize cursors on edge hover (`1d9301e`)
- [x] Close button — drops AppWindow + TerminalCore (Option wrapping) (`1d9301e`)
- [x] Maximize toggle — green button fills app_viewport, restores saved bounds (`712adf7`)
- [x] Title bar double-click — toggles maximize with 400ms threshold (`712adf7`)
- [x] Maximized state preserved on outer window resize (`712adf7`)

**Embedded terminal:**

- [x] TerminalRenderer renders into content_rect via (Canvas, Rect, z) (`1d9301e`)
- [x] Live terminal grid resize during edge drag (`1d9301e`)
- [x] Terminal refit on outer window resize + scale factor change (`1d9301e`)
- [x] Keyboard input — full key mapping (arrows, ctrl, backspace, tab, etc.) (`1d9301e`)
- [x] Mouse scroll — line + pixel delta scrollback (`1d9301e`)

**Not done:**

- [ ] Minimize button behavior (yellow traffic light renders but no handler)
- [x] Live date + time in shell bar right corner (chrono, macOS-style format)
- [x] Launcher overlay (Cmd+p app grid — prerequisite for Phase 2)
- [x] Detir-scene IR renderer embedded in an AppWindow (AppContent::IrScene + SceneRenderer)
- [ ] `orus-core/src/app.rs` manifest types (currently empty)

### Phase 2: App Packaging + Multi-App

- [ ] Task 2.1: Unified app registry + auto-discovery loader
  - **Current problem:** Two separate app models — built-in apps (Terminal, Browser, TextEdit) are hardcoded in launcher.rs with no manifest; IR apps (Workflow Editor, Sim Viewer) live in `~/.orus/apps/` with DETIR.MANIFEST.json. No auto-discovery.
  - **Needed:** Unified `AppDescriptor` that covers both built-in and user-installed apps. Built-ins registered programmatically, user apps discovered by scanning `~/.orus/apps/*/DETIR.MANIFEST.json`. Launcher populated from single registry. `orus-core/src/app.rs` manifest types still empty.
- [x] Task 2.2: Multi-app state — WindowManager with Vec\<ManagedWindow\>, focus/z-ordering, Ctrl+Tab cycling, session persistence
- [x] Task 2.3: Sample Orus OS apps — Workflow Editor + Sim Viewer (IR apps via DETIR.MANIFEST.json, hardcoded in launcher)
- [ ] Task 2.4: System Settings app — permission management UI for installed apps
- [ ] Task 2.5: Image Viewer app — IR app that renders an image from path (future: upgrade to image editor)
- [ ] Task 2.6: File Manager app — sandboxed browser for `~/.orus/` contents
  - Browse: `~/.orus/apps/` (installed apps), `~/.orus/data/<app>/` (app data), `~/.orus/shared/` (host exchange), `~/.orus/data/audit/` (logs)
  - Features: tree view, text/JSON/image preview, per-app storage usage vs quota, delete/clear cache
  - Drag-and-drop into `~/.orus/shared/` for host ↔ Orus OS file exchange
  - Does NOT access host filesystem outside `~/.orus/`

### Phase 3: Sandbox Layer

- [x] Task 3.1: File broker — deny-by-default, per-app isolation, path traversal prevention (`32464d6`)
- [x] Task 3.2: Network broker — private network denial, port allowlist, TOML whitelist (`32464d6`)
- [x] Task 3.3: Audit logger — JSON-Lines session logs, replay support (`32464d6`)

### Phase 4: Agent Protocol

- [ ] Task 4.0: Peco assistant UI — floating panel (not a windowed app)
  - **Design:** System-level overlay like Siri/Spotlight, not an AppWindow. Invoked via hotkey (e.g. Cmd+Shift+P), slides in as a floating panel over current workspace. No window chrome, rounded corners, semi-transparent backdrop.
  - **Why floating:** Peco is the system agent — it observes/acts across all apps via the agent protocol. A windowed app would limit it to its own sandbox. A floating panel gives it system-level presence.
  - **Current state:** Peco crate is loaded but not connected to an LLM backend. UI needs to be built.
  - **Components:** Chat input + response area, context indicator (which app Peco is observing), capability badges, streaming response display
- [ ] Task 4.1: Agent protocol server (compose push-wire + detir-server-rpc)
- [ ] Task 4.2: Capability gating middleware
- [ ] Task 4.3: Redaction filter
- [ ] Task 4.4: Peco LLM connection — wire peco-llm to a provider (Anthropic/OpenAI), API key management in System Settings

### Phase 5: System Tray Agent

> Goal: Long-lived background process that outlives the Orus OS window. Hosts Peco workflow scheduler, system tray menu, and IPC bridge to the main app.

- [ ] Task 5.1: `orus-tray` binary crate — system tray icon + menu using `tray-icon` + `muda`
  - Menu: Open Orus OS, Workflows (list + enable/disable), Peco quick prompt, Quit
  - Tray icon states: idle, running workflow, error
  - Lightweight — no GPU, no render loop
- [ ] Task 5.2: IPC bridge between tray agent and Orus OS shell
  - Unix domain socket (macOS/Linux) or named pipe (Windows) at `~/.orus/run/orus.sock`
  - Protocol: JSON messages — status queries, workflow dispatch, app launch requests
  - When Orus OS is open: tray routes workflows through agent protocol (full app context)
  - When Orus OS is closed: tray runs headless workflows (file ops, network, notifications)
- [ ] Task 5.3: Peco workflow scheduler
  - `Schedule { workflow_id, cron_expr, last_run, enabled }` persisted to `~/.orus/config/schedules.toml`
  - Tick loop (1s interval) checks cron expressions, dispatches to `peco-workflow` engine
  - Missed-run detection on startup (catch up if tray was not running)
  - Workflow results logged to audit system (Phase 3)
- [ ] Task 5.4: Auto-start on login
  - macOS: LaunchAgent plist (`~/Library/LaunchAgents/com.orus.tray.plist`)
  - Windows: Registry run key or startup folder shortcut
  - Linux: XDG autostart `.desktop` file
- [ ] Task 5.5: Tray → Orus OS launch integration
  - "Open Orus OS" menu item spawns `orus-macos` (or platform equivalent)
  - Orus OS on startup connects to existing tray agent via IPC
  - Orus OS window close does NOT kill tray agent — only "Quit" from tray menu exits both

### Phase 6: CEF Integration

- [ ] Task 6.1: CEF browser hosting via detir-cef-native

### Phase 7: Cross-Platform + Native Sandboxing

- [ ] Task 7.1: Windows shell + AppContainer sandbox
  - `orus-windows` binary crate
  - AppContainer — run each Orus OS app as a restricted process (deny filesystem/registry/network outside broker)
  - MSIX packaging for Windows Store distribution (auto-grants AppContainer)
- [ ] Task 7.2: Linux shell + sandbox
  - `orus-linux` binary crate
  - Primary sandbox: bubblewrap (`bwrap`) — embedded, no external dependency, namespace-based isolation
  - Distribution: `.deb` / `.rpm` / AppImage (native packages, no runtime dependency)
  - Optional: Flatpak build for users who prefer it (portals for file picker, notifications)
  - Bubblewrap is what Flatpak uses internally — we get the same isolation without requiring the Flatpak runtime
  - Graceful degradation: detect unprivileged user namespaces on startup, warn if unavailable ("OS-level sandboxing unavailable — running with application-level sandbox only"), fall back to Rust brokers (Phase 3)
- [ ] Task 7.3: macOS App Sandbox + notarized .app bundle
  - Entitlements: `com.apple.security.app-sandbox`, network client, user-selected file access
  - Hardened runtime for notarization
  - .app bundle with embedded tray agent as login item
- [ ] Task 7.4: Native sandbox ↔ Rust broker integration
  - OS sandbox provides process-level enforcement (defense-in-depth)
  - Rust brokers (Phase 3) provide per-app isolation within the Orus OS process
  - Two layers: even if a Rust broker bug exists, OS container catches it

---

## What Already Exists (DO NOT REBUILD)

| Need                       | Existing Crate                                                                   | What It Provides                                                                               |
| -------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Window + GPU + render loop | `detir-platform` (`WinitBackend`)                                                | winit window, wgpu surface/device/queue, `PlatformBackend` trait                               |
| Input events               | `detir-platform` (`event.rs`)                                                    | `KeyEvent`, `MouseEvent`, `ScrollEvent`, `TouchEvent`, `Modifiers` — already platform-agnostic |
| IR renderer                | `detir-scene` (`IrRenderer`)                                                     | HTML → Taffy layout → display list → GPU. Full pipeline.                                       |
| Zone layout                | `detir-scene` (`ZoneManager`)                                                    | Toolbar, sidebar, viewport, dock, devtools, chat panel                                         |
| CEF native view (macOS)    | `detir-cef-native`                                                               | NSView sibling hosting, positioning, input forwarding                                          |
| CEF headless               | `detir-cef`                                                                      | Headless Chromium rendering, CDP protocol                                                      |
| Peco agent                 | `peco-agent`, `peco-automation`, `peco-workflow`, `peco-llm`, `peco-page-reader` | LLM integration, DOM automation, workflow engine                                               |
| JS engine + CSS            | `detir-js`                                                                       | Boa runtime, DOM shim, CSS cascade, HTML→IR translation                                        |
| Arth VM                    | `detir-arth`                                                                     | Bytecode execution, scene graph mutations                                                      |
| App runtime contract       | `detir-app-runtime`                                                              | `AppRuntime` trait, `UiEvent`, `IrMutation`                                                    |
| Text rendering             | `engine-core` + `detir-text`                                                     | CosmicText shaping, FreeType LCD, subpixel AA                                                  |
| Hit testing                | `engine-core`                                                                    | `HitIndex`, `HitRegion`, z-ordered spatial queries                                             |
| Animations                 | `detir-scene/animation/`                                                         | Keyframe, transition, easing, state tracking                                                   |
| Config                     | `detir-config`                                                                   | `detir.toml` loading, env var overrides                                                        |
| Accessibility              | `detir-scene/ir_renderer/accessibility.rs`                                       | A11y tree generation from IR                                                                   |
| WebSocket server           | `push-wire-server`                                                               | Multiplexed WS, priority queues, replay, binary codec, delta compression                       |
| IR RPC protocol            | `detir-server-rpc`                                                               | `RpcRequest`/`RpcResponse` for GetIr, Event, Subscribe                                         |
| IR diff engine             | `detir-server-ir-diff`                                                           | Efficient delta updates between IR snapshots                                                   |
| Push channels              | `detir-server-rps`                                                               | Detir channel taxonomy (Ui, Devtools, Presence, RTC)                                           |
| App manifest (server)      | `detir-server-core`                                                              | `AppManifest`, `DetirPackage`, event types                                                     |

## What's Actually New

1. **Multi-app compositor** — Run N Detir apps simultaneously as separate surfaces
2. **Orus OS shell** — Shell bar, app launcher, app switcher (replaces current zone layout)
3. **App packaging** — `~/.orus/apps/` directory scanning, manifest.json (extend `detir-server-core::AppManifest` with Orus OS permissions)
4. **File broker** — Deny-by-default filesystem sandbox scoped to `~/.orus/`
5. **Network broker** — Brokered egress, private network denial
6. **Agent protocol server** — Thin composition of `push-wire-server` + `detir-server-rpc` with Orus OS-specific capability gating, redaction, and audit middleware
7. **Audit logger** — Action logging with replay support
8. **Redaction layer** — Filter sensitive content from agent view

---

## Phase 0: Project Structure

### Task 0.1: Restructure orus-universal workspace

The workspace already exists from initial scaffolding. Restructure it to properly depend on detir crates as the foundation rather than reimplementing them.

**Files:**

- Modify: `orus-universal/Cargo.toml`
- Modify: `orus-universal/crates/orus-core/Cargo.toml`
- Modify: `orus-universal/crates/orus-compositor/Cargo.toml`
- Modify: `orus-universal/crates/orus-macos/Cargo.toml`

**Step 1: Update workspace deps to include detir-scene, detir-platform, detir-app-runtime**

Add to `orus-universal/Cargo.toml` workspace dependencies:

```toml
detir-scene = { path = "../detir/crates/detir-scene", default-features = false }
detir-platform = { path = "../detir/crates/detir-platform" }
detir-app-runtime = { path = "../detir/crates/detir-app-runtime" }
detir-config = { path = "../detir/crates/detir-config" }
detir-js = { path = "../detir/crates/detir-js" }
detir-cef-native = { path = "../detir/crates/detir-cef-native" }
peco-agent = { path = "../detir/crates/peco-agent" }
```

**Step 2: Make orus-macos depend on detir-scene + detir-platform**

The macOS binary should use `WinitBackend` from detir-platform and `SceneApp`/`IrRenderer` from detir-scene — NOT raw objc2.

**Step 3: Verify builds**

Run: `cd orus-universal && cargo build`

**Step 4: Commit**

```bash
git add orus-universal/
git commit -m "refactor: restructure orus-universal to build on detir stack"
```

---

## Phase 1: Single-App Orus OS Shell

> Goal: Replace detir-scene's default zone layout with Orus OS shell chrome (shell bar, dark background, "Orus OS" branding). Still runs one app.

### Task 1.1: Orus OS launcher binary using detir-scene

**Files:**

- Modify: `orus-universal/crates/orus-macos/src/main.rs`
- Modify: `orus-universal/crates/orus-macos/Cargo.toml`

**Step 1: Write main.rs that calls detir_scene::run()**

The simplest starting point: just call `detir_scene::run()` which already creates a winit window, initializes wgpu, loads IR packages, renders with zones, and handles all input.

```rust
fn main() -> anyhow::Result<()> {
    env_logger::init();
    // For now, just use the existing detir-scene runtime
    detir_scene::run()
}
```

**Step 2: Run and verify**

Run: `cargo run -p orus-macos`
Expected: Full detir-scene window with toolbar, sidebar, viewport, dock.

**Step 3: Commit**

```bash
git commit -m "feat: orus-macos launcher using detir-scene runtime"
```

### Task 1.2: Create Orus OS shell zone layout

**Files:**

- Create: `orus-universal/crates/orus-shell/Cargo.toml`
- Create: `orus-universal/crates/orus-shell/src/lib.rs`
- Create: `orus-universal/crates/orus-shell/src/shell_bar.rs`
- Create: `orus-universal/crates/orus-shell/src/launcher.rs`

**Step 1: Define shell bar as a zone**

Use the existing `ZoneManager` pattern from `detir-scene/src/zones/`. The Orus OS shell bar is analogous to the existing toolbar — a fixed-height region at the top of the screen.

Shell bar contents:

- App icons for running apps (like dock items)
- System clock
- Agent status indicator (Peco connected/idle/active)
- "Orus OS" branding

Render using `engine_core::Painter` + `detir-surface` Canvas API — same pattern as existing zones.

**Step 2: Define app launcher overlay**

When user clicks launcher button or presses Cmd+Space:

- Read `~/.orus/apps/*/manifest.json`
- Display grid of app icons + names
- Click launches app in viewport

Render as a full-screen overlay zone (like the existing devtools panel).

**Step 3: Commit**

```bash
git commit -m "feat(shell): Orus OS shell bar + app launcher zones"
```

### Task 1.3: Custom Orus OS runner (fork of ir_renderer::run)

**Files:**

- Create: `orus-universal/crates/orus-shell/src/runner.rs`
- Modify: `orus-universal/crates/orus-macos/src/main.rs`

**Step 1: Create a Orus OS-specific runner**

Fork the structure of `detir-scene/src/ir_renderer/runner.rs` but replace the default zone layout with the Orus OS shell layout. This runner:

1. Uses `WinitBackend` from `detir-platform` (window + wgpu — reused as-is)
2. Creates the same wgpu pipeline (PassManager, RenderAllocator — reused as-is)
3. Replaces `ZoneManager` with `OrusShell` (shell bar + app viewport)
4. Uses `IrRenderer` to render the active app (reused as-is)
5. Handles input via `detir-platform` events (reused as-is)

Key difference: the shell manages which app is visible, not the toolbar URL bar.

**Step 2: Wire orus-macos to use the Orus OS runner**

```rust
fn main() -> anyhow::Result<()> {
    env_logger::init();
    orus_shell::run()
}
```

**Step 3: Commit**

```bash
git commit -m "feat: custom Orus OS runner with shell bar replacing default zones"
```

---

## Phase 2: App Packaging + Multi-App

> Goal: Load multiple apps from `~/.orus/apps/`, switch between them.

### Task 2.1: App manifest + package loader

**Files:**

- Create: `orus-universal/crates/orus-core/src/manifest.rs`
- Create: `orus-universal/crates/orus-core/src/package_loader.rs`
- Test: `orus-universal/crates/orus-core/tests/manifest_test.rs`

**Step 1: Write test for manifest parsing**

```rust
#[test]
fn test_parse_manifest() {
    let json = r#"{"name":"Test","version":"1.0.0","entry":"index.html","permissions":{"network":[],"filesystem":[],"agent":{"read":true,"actuate":false}}}"#;
    let manifest: AppManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.name, "Test");
    assert!(manifest.permissions.agent.read);
}
```

**Step 2: Implement AppManifest and loader**

Scan `~/.orus/apps/*/manifest.json`. Each app has:

- `manifest.json` (name, version, entry, icon, permissions)
- `index.html` (entry point for Detir rendering)
- Optional: additional HTML/CSS/JS/assets

Use `detir_ir::package::DetirPackage` to load the IR package from each app directory — this already exists.

**Step 3: Commit**

```bash
git commit -m "feat(core): app manifest parsing + ~/.orus/apps/ package loader"
```

### Task 2.2: Multi-app state management

**Files:**

- Create: `orus-universal/crates/orus-core/src/app_manager.rs`
- Modify: `orus-universal/crates/orus-shell/src/runner.rs`

**Step 1: AppManager struct**

```rust
pub struct AppManager {
    apps: Vec<RunningApp>,
    active_app: Option<usize>,
}

pub struct RunningApp {
    pub manifest: AppManifest,
    pub ir_renderer: IrRenderer,
    pub view_doc: ViewDocument,
    pub data_doc: DataDocument,
    pub arth_host: ArthHost,
}
```

Each app has its own `IrRenderer` instance (owns its own Taffy layout tree, element state, scroll positions, etc.). The compositor renders only the active app's display list.

**Step 2: App switching**

Shell bar click → `AppManager::switch_to(index)` → compositor renders different app's surface.

Each app preserves its state when backgrounded (IR state, scroll position, form data, etc.).

**Step 3: Commit**

```bash
git commit -m "feat(core): multi-app state management with per-app IrRenderer"
```

### Task 2.3: Create sample Orus OS apps

**Files:**

- Create: `~/.orus/apps/hello/manifest.json`
- Create: `~/.orus/apps/hello/index.html`
- Create: `~/.orus/apps/notes/manifest.json`
- Create: `~/.orus/apps/notes/index.html`

**Step 1: Create hello app (minimal)**

Simple styled page: "Hello from Orus OS" with the Orus OS logo.

**Step 2: Create notes app**

Basic note-taking app with text input, save to `~/.orus/data/notes/`.

- We already have a markdown editor do we need notes?

**Step 3: Test multi-app switching**

Run `cargo run -p orus-macos`, verify both apps appear in launcher, switching works.

**Step 4: Commit**

```bash
git commit -m "feat: sample Orus OS apps (hello, notes)"
```

---

## Phase 3: Sandbox Layer

> Goal: Deny-by-default filesystem and network access with brokered capabilities.

### Task 3.1: File broker

**Files:**

- Create: `orus-universal/crates/orus-sandbox/Cargo.toml`
- Create: `orus-universal/crates/orus-sandbox/src/lib.rs`
- Create: `orus-universal/crates/orus-sandbox/src/file_broker.rs`
- Test: `orus-universal/crates/orus-sandbox/tests/file_broker_test.rs`

**Step 1: Write tests**

```rust
#[test]
fn deny_outside_orus_dir() {
    let broker = FileBroker::new("/Users/test/.orus");
    assert!(broker.check_read("/Users/test/.orus/apps/test/index.html").is_ok());
    assert!(broker.check_read("/etc/passwd").is_err());
    assert!(broker.check_read("/Users/test/.orus/../../etc/passwd").is_err());
}

#[test]
fn per_app_data_isolation() {
    let broker = FileBroker::new("/Users/test/.orus");
    assert!(broker.check_write("app-a", "/Users/test/.orus/data/app-a/notes.db").is_ok());
    assert!(broker.check_write("app-a", "/Users/test/.orus/data/app-b/notes.db").is_err());
}
```

**Step 2: Implement FileBroker**

Per spec Section 3.2:

- Deny-by-default, canonicalize all paths
- Scoped to `~/.orus/` with per-app data subdirectories
- Quotas per app
- `~/.orus/shared/` as host ↔ Orus OS exchange directory

**Step 3: Commit**

```bash
git commit -m "feat(sandbox): file broker with deny-by-default + per-app isolation"
```

### Task 3.2: Network broker

**Files:**

- Create: `orus-universal/crates/orus-sandbox/src/network_broker.rs`
- Test: `orus-universal/crates/orus-sandbox/tests/network_broker_test.rs`

**Step 1: Write tests**

```rust
#[test]
fn deny_private_networks() {
    let broker = NetworkBroker::default();
    assert!(broker.check_egress("93.184.216.34", 443).is_ok());   // public
    assert!(broker.check_egress("127.0.0.1", 8080).is_err());     // localhost
    assert!(broker.check_egress("192.168.1.1", 80).is_err());     // LAN
    assert!(broker.check_egress("10.0.0.1", 443).is_err());       // private
}
```

**Step 2: Implement NetworkBroker**

Per spec Section 3.3:

- Brokered egress, allow 80/443 to public internet
- Deny localhost, LAN (RFC1918), link-local, multicast
- DNS brokered (resolve before checking)
- Whitelist in `~/.orus/config/network.toml`
- All requests logged

**Step 3: Commit**

```bash
git commit -m "feat(sandbox): network broker with private network denial"
```

### Task 3.3: Audit logger

**Files:**

- Create: `orus-universal/crates/orus-sandbox/src/audit.rs`
- Test: `orus-universal/crates/orus-sandbox/tests/audit_test.rs`

**Step 1: Implement audit logger**

Per spec Section 4.5:

- Log to `~/.orus/data/audit/session-<timestamp>.log`
- Each entry: timestamp, agent ID, target app, action type, payload, result, state hash
- Sessions replayable from logs

**Step 2: Commit**

```bash
git commit -m "feat(sandbox): audit logger with session replay support"
```

---

## Phase 4: Agent Protocol

> Goal: External agents can observe IR state and perform actions via WebSocket.

### Existing Infrastructure (DO NOT REBUILD)

| Need                 | Existing Crate                                            | What It Provides                                                                          |
| -------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| WebSocket transport  | `push-wire-server` (`PushServer`)                         | Multiplexed WS with priority queues, cursor-based replay, binary codec, delta compression |
| IR state query       | `detir-server-rpc` (`RpcRequest::GetIr`)                  | Request/response for ViewDocument + DataDocument                                          |
| UI event dispatch    | `detir-server-rpc` (`RpcRequest::Event`)                  | `UiEvent` → `IrMutation` pipeline                                                         |
| Live IR updates      | `detir-server-rpc` (`RpcRequest::Subscribe`)              | Push IR diffs over WebSocket                                                              |
| IR diff streaming    | `detir-server-ir-diff`                                    | Efficient delta updates between IR snapshots                                              |
| Session management   | `detir-server-rpc::SessionManager`                        | Per-client sessions with state tracking                                                   |
| Channel multiplexing | `detir-server-rps::Channel` (Ui, Devtools, Presence, RTC) | Named channels with per-channel handlers                                                  |
| App manifest         | `detir-server-core::AppManifest`                          | Manifest parsing already exists                                                           |
| Accessibility tree   | `detir-scene/ir_renderer/accessibility.rs`                | A11y tree generation for CEF agent control                                                |

### What's Actually New

Only **three thin layers** on top of the existing stack:

### Task 4.1: Agent protocol server (compose existing crates)

**Files:**

- Create: `orus-universal/crates/orus-agent/Cargo.toml`
- Create: `orus-universal/crates/orus-agent/src/lib.rs`
- Create: `orus-universal/crates/orus-agent/src/server.rs`

**Step 1: Define Orus OS agent channel**

Extend `detir-server-rps` channel taxonomy with a `Channel::Agent` variant, or define a Orus OS-specific `ChannelKind` that wraps the existing Detir channels and adds agent-specific ones:

```rust
// Reuse push-wire for transport
use push_wire_server::PushServer;
// Reuse detir-server-rpc for IR query/event protocol
use detir_server_rpc::{RpcRequest, RpcResponse};
// Reuse detir-server-rps for channel definitions
use detir_server_rps::Channel;
```

**Step 2: Wire to AppManager**

The agent server receives `RpcRequest` messages and routes them through the `AppManager`:

- `GetIr { package_id }` → `AppManager::get_app_state(id)` → return ViewDocument + DataDocument
- `Event { event }` → `AppManager::dispatch_event(id, event)` → return IrMutations
- `Subscribe { package_id }` → push-wire subscription to IR diff stream

For CEF apps: use `detir-scene/ir_renderer/accessibility.rs` to generate the a11y tree snapshot.

**Step 3: Commit**

```bash
git commit -m "feat(agent): agent protocol server composing push-wire + detir-server-rpc"
```

### Task 4.2: Capability gating middleware

**Files:**

- Create: `orus-universal/crates/orus-agent/src/capabilities.rs`

**Step 1: Implement as push-wire handler wrapper**

Wrap the channel handler to check manifest permissions before forwarding:

```rust
fn capability_gated_handler(
    manifest: &AppManifest,
    inner: ChannelHandler,
) -> ChannelHandler {
    // Check manifest.permissions.agent.read / .actuate
    // Block actuation requests if not granted
    // Require user confirmation for confirm_actions list
}
```

Per spec Section 4.2: read-only vs actuation, per-app scoping, user confirmation for high-risk actions.

**Step 2: Commit**

```bash
git commit -m "feat(agent): capability gating middleware"
```

### Task 4.3: Redaction filter

**Files:**

- Create: `orus-universal/crates/orus-agent/src/redaction.rs`

**Step 1: Implement as response filter**

Filter `RpcResponse::Ir` payloads before sending to agent:

- Password fields → redacted from ViewDocument
- File paths outside `~/.orus/` → stripped
- Secrets in `~/.orus/config/` → not exposed
- Clipboard → isolated within Orus OS

Per spec Section 4.3.

**Step 2: Commit**

```bash
git commit -m "feat(agent): redaction layer for sensitive content"
```

---

## Phase 6: CEF Integration

> Goal: Host CEF browser as sibling native view, managed by the compositor.

### Task 6.1: CEF in Orus OS shell

**Files:**

- Modify: `orus-universal/crates/orus-shell/src/runner.rs`

**Step 1: Integrate existing detir-cef-native**

The `detir-cef-native` crate already handles:

- Creating CEF as sibling NSView (macOS)
- Positioning and sizing the CEF view
- Input forwarding

The Orus OS shell needs to:

- Treat CEF as a special app type (not IR-rendered)
- Position CEF view to match the active viewport zone
- Show/hide CEF view when switching apps

This is mostly wiring — `detir-cef-native::NativeCefView` is already the implementation.

**Step 2: Agent protocol bridge for CEF**

When agent queries a CEF app, use the accessibility tree pattern (snapshot → ref → action → re-snapshot). Existing `peco-page-reader` + `peco-automation` already implement this.

**Step 3: Commit**

```bash
git commit -m "feat: CEF integration via existing detir-cef-native"
```

---

## Phase 7: Cross-Platform

> Goal: Windows and Linux support via detir-platform backends.

### Task 7.1: Windows shell

**Files:**

- Create: `orus-universal/crates/orus-windows/`

Since `detir-platform` already has the `WinitBackend` which works on Windows (winit supports Win32), and wgpu supports DX12/Vulkan on Windows, this is mostly:

- A new binary crate that calls `orus_shell::run()`
- Windows sandbox (AppContainer + restricted token)
- CEF HWND hosting (already supported in `detir-cef-native` Cargo.toml)

### Task 7.2: Linux shell

Same pattern — new binary crate, bubblewrap sandbox, CEF via X11/Wayland.

Note: On Linux, Universal is a Wayland/X11 **client** (app inside the desktop), NOT a compositor.

### Task 7.3: Distribution packaging

- macOS: .app bundle (notarized)
- Windows: MSIX installer
- Linux: AppImage or Flatpak
- Optional: Lite build without CEF (~49MB vs ~250MB)

---

## Dependency Graph

```
Phase 0: Restructure workspace
    │
    ▼
Phase 1: Single-app Orus OS shell ◄── builds on detir-scene
    │
    ├──────────────────────┐
    ▼                      ▼
Phase 2: Multi-app     Phase 6: CEF ◄── builds on detir-cef-native
    │
    ▼
Phase 3: Sandbox
    │
    ▼
Phase 4: Agent Protocol (Peco UI + LLM)
    │
    ▼
Phase 5: System Tray Agent (tray-icon, IPC, scheduler)
    │
    ▼
Phase 7: Cross-Platform
```

Phases 1-2 are the critical path. Phase 6 (CEF) can run in parallel. Phase 5 (tray) depends on Phase 4 (agent protocol) for workflow dispatch.

---

## Final Crate Structure

```
orus-universal/
├── Cargo.toml
├── crates/
│   ├── orus-core/         # App manifest, package loader, app manager
│   ├── orus-shell/        # Shell bar, launcher, Orus OS runner
│   ├── orus-sandbox/      # File broker, network broker, audit logger
│   ├── orus-agent/        # Agent protocol server, capabilities, redaction
│   ├── orus-tray/         # System tray agent — background daemon, scheduler, IPC
│   ├── orus-macos/        # macOS binary (thin — calls orus_shell::run())
│   ├── orus-windows/      # Windows binary
│   └── orus-linux/        # Linux binary
└── docs/

Reuses (path dependencies, NOT copies):
├── detir-platform    → Window, GPU, input events
├── detir-scene       → IrRenderer, zones, animations, focus, Peco
├── detir-app-runtime → AppRuntime trait, UiEvent, IrMutation
├── detir-ir          → IR types (ViewDocument, DataDocument, etc.)
├── detir-js          → Boa JS engine + CSS
├── detir-cef-native  → CEF native view hosting
├── engine-core       → GPU rendering, hit-testing, text
├── engine-shaders    → WGSL shaders
├── peco-*            → Agent intelligence
└── detir-config      → Configuration loading
```

---

## Dev Box (ORUS/) Reuse

From the separate Dev Box repository at `/Users/yppartha/PROJECTS/ORUS/`:

**Port algorithms (not crate deps):**

- `compositor/src/shell/layout.rs` → Tiling layouts (master-stack, binary split, monocle) for multi-app window management
- `compositor/src/shell/mod.rs` → Focus cycling, workspace management
- `compositor/src/ipc/protocol.rs` → Binary protocol patterns (for agent protocol design reference)

**Do NOT port:**

- Smithay/Wayland protocol handling (irrelevant for Universal)
- DRM/KMS/evdev backends (replaced by detir-platform)
- Layer-shell client code (replaced by Orus OS shell zones)

---

_Orus OS Universal: built on Detir, not beside it._
