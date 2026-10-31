# Orus OS Universal — Technical Specification

**Edition:** Universal (U)
**Status:** Draft
**Purpose:** Orus OS as a native desktop application for macOS, Windows, and Linux. For trying Orus OS. Recommended production deployment is Orus OS Dev Box (D) on bare metal.

---

## 1. Overview

Orus OS Universal delivers the full Orus OS experience — IR-driven compositor, Detir renderer, CEF web compatibility, Peco agent interface — as a sandboxed native application on the host operating system. It uses the host GPU directly (Metal, Vulkan, DX12) via wgpu with no VM, no dual boot, and no GPU passthrough.

The OS is not hosting AI. It is designed to be controlled by AI. Universal makes that accessible to everyone.

---

## 2. Architecture

### 2.1 High-Level Stack

```
Host OS (macOS / Windows / Linux)
│
└── Orus OS Universal (native app)
    │
    ├── Native Shell (per-platform)
    │   ├── macOS: Swift/ObjC — NSApplication, NSWindow, NSViews
    │   ├── Windows: Win32/WinUI — HWND management
    │   └── Linux: GTK or Wayland client — window management
    │
    ├── Rust Core (cross-platform, shared)
    │   ├── Compositor (wgpu, single GPU frame graph, one final present)
    │   ├── Detir Renderer (html5ever, Taffy layout, wgpu IR, Boa JS)
    │   ├── Detir Terminal
    │   ├── Peco Agent Interface
    │   ├── Agent Protocol (WebSocket / Unix socket)
    │   ├── File Broker
    │   ├── Network Broker
    │   └── Audit Logger
    │
    ├── CEF (Chromium Embedded Framework)
    │   ├── Hosted as sibling native view (NSView / HWND)
    │   ├── Treated as overlay island in compositor
    │   └── Supervised child processes
    │
    └── Sandbox Boundary
        ├── Filesystem: deny-by-default, scoped to ~/.orus/
        ├── Network: brokered egress, private-network denial
        └── Process: supervisor tree, resource limits
```

### 2.2 Process Model

Orus OS Universal runs as a supervisor process with supervised children. CEF spawns its own renderer processes. Terminating the supervisor (Cmd+Q / Alt+F4 / kill) terminates all children.

```
Supervisor (native shell)
├── Compositor thread (wgpu rendering)
├── Detir App 1 thread (parse → layout → IR → surface)
├── Detir App 2 thread (parse → layout → IR → surface)
├── Detir App N thread (parse → layout → IR → surface)
├── Detir Terminal thread
├── Peco thread
├── Agent Protocol thread
├── File Broker thread
├── Network Broker thread
├── Audit Logger thread
└── CEF child processes (renderer, GPU, utility — managed by CEF)
```

Each Detir app maintains isolated state (DOM, layout tree, Boa JS context, IR output) on its own thread. Apps are logically isolated: app = isolated state + scheduler slot. Threads are an optimization detail.

### 2.3 Rendering

The compositor maintains a single GPU frame graph with damage tracking. Multiple render passes handle text atlas updates, offscreen surfaces, CEF texture copies, and effects. One final swapchain present per frame.

Detir app threads submit IR surfaces to the compositor. The compositor composites all surfaces — Detir and CEF — into one coherent output rendered via wgpu to the native shell's view.

CEF is a special-case overlay island: a sibling native view positioned and sized by the compositor. Limited transform and opacity expectations. No blur, rotation, or animation applied to the CEF view by the compositor.

### 2.4 App Window Model

Orus OS Universal runs inside a single host OS window but presents apps as individual windows with full desktop window behavior. Each app gets a GPU-drawn window frame (client-side decorations) rendered by the compositor:

- **Title bar:** App name and icon, draggable to reposition the app window within the Orus OS viewport.
- **Window controls:** Close, minimize, and maximize/restore buttons (rendered in Orus OS style, not native host OS style).
- **Resize handles:** Drag edges and corners to resize the app window.
- **Shadow/border:** Visual separation between overlapping app windows.
- **Focus ring:** Active window distinguished from background windows via border highlight and z-ordering.

The compositor manages z-ordering, focus cycling (Cmd/Alt+Tab), and window snapping/tiling. This mirrors the Dev Box (bare-metal) experience where apps are real Wayland toplevels with server-side decorations — Universal achieves the same UX by drawing decorations in the GPU pipeline via `engine-core`.

Apps can be in one of three states: **floating** (freely positioned and resized), **maximized** (fills the viewport below the shell bar), or **minimized** (hidden, accessible from shell bar or app switcher).

### 2.5 Per-Platform Native Shell

| Component | macOS | Windows | Linux |
|-----------|-------|---------|-------|
| App framework | NSApplication (Swift/ObjC) | Win32 / WinUI | GTK or Wayland client |
| Window | NSWindow (borderless) | HWND (borderless) | Borderless window |
| Compositor view | NSView + Metal (via wgpu) | HWND + DX12/Vulkan (via wgpu) | Window + Vulkan (via wgpu) |
| CEF view | NSView (sibling, same NSWindow) | HWND (sibling, same parent) | Window (sibling, same parent) |
| GPU API | Metal | DX12 / Vulkan | Vulkan |
| Sandbox | App Sandbox / seatbelt | AppContainer / restricted token | bubblewrap / namespaces / seccomp |

The Rust core (compositor, Detir, agent protocol, brokers) is shared across all platforms via FFI. The native shell is thin glue: window management, CEF hosting, input routing, and platform sandbox integration.

---

## 3. Sandbox

### 3.1 Design Principle

Orus OS Universal is a sandboxed runtime with brokered file, network, and UI capabilities. The agent operates inside Orus OS and cannot access the host filesystem, network services, or processes outside the sandbox.

### 3.2 Filesystem Sandbox

**Deny-by-default.** Orus OS Universal can only access `~/.orus/` and explicitly user-granted paths.

```
~/.orus/
├── apps/         ← Detir app bundles
├── data/         ← per-app data (databases, state)
├── downloads/    ← CEF downloads
├── config/       ← agent config, network whitelist, preferences
└── shared/       ← user-facing exchange directory (host ↔ Orus OS)
```

A File Broker mediates all filesystem access:

- All file operations routed through the broker, not direct syscalls.
- Per-app subdirectories under `~/.orus/data/` with quotas.
- `~/.orus/shared/` is the only bridge between host and Orus OS. User drops files in to share with the agent; agent outputs land here for the user.
- User may grant additional paths at runtime via system file picker (brokered, explicit).

Platform enforcement:

- **macOS:** App Sandbox entitlements or userspace FS broker.
- **Windows:** Restricted token + job object + explicit file picker broker.
- **Linux:** bubblewrap / mount namespaces / seccomp.

### 3.3 Network Sandbox

**Brokered egress with private-network denial by default.** A Network Broker mediates all socket operations.

```
ALLOW   outbound → internet (ports 80, 443)
ALLOW   outbound → user-configured API whitelist
ALLOW   local    → agent protocol (unix socket / localhost agent port)
DENY    outbound → host localhost (127.0.0.1 / ::1)
DENY    outbound → LAN (RFC1918: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
DENY    outbound → link-local, multicast
DENY    inbound  → all (no port binding)
```

DNS is brokered — direct IP connections to private ranges are blocked regardless of DNS resolution. All network requests are logged to the audit trail.

The whitelist is user-configurable in `~/.orus/config/network.toml`.

### 3.4 Process Sandbox

- Supervisor process manages all children.
- Resource limits (CPU, memory) enforced per app.
- Crash containment: a Detir app crash does not bring down the compositor or other apps.
- CEF child processes are supervised and terminated with the parent.

---

## 4. Agent Protocol

### 4.1 Dual Interface

The agent protocol provides two control surfaces:

| Surface | Interface | Performance | Determinism |
|---------|-----------|-------------|-------------|
| Detir apps | Typed IR — canonical layout + computed state | Sub-50ms actions | Deterministic |
| CEF browser | Accessibility tree (via agent-browser pattern) | ~2s per action cycle | Non-deterministic |

Detir IR is the premium path. CEF accessibility tree is the compatibility fallback. The agent seamlessly switches between both.

### 4.2 Capability Gating

Agent access is capability-gated per app and per action:

- **Read-only:** agent can observe IR state / accessibility tree.
- **Actuation:** agent can perform actions (click, fill, navigate). Requires explicit grant.
- **Per-app scoping:** agent permissions are scoped to individual apps.
- **User confirmation:** high-risk actions (purchases, form submissions) can require user confirmation via Peco.

### 4.3 Redaction Layer

Sensitive content is filtered before reaching the agent:

- Password fields are redacted from IR and accessibility tree.
- Clipboard contents are isolated within Orus OS (not shared with host).
- File paths outside `~/.orus/` are not exposed.
- Secrets in `~/.orus/config/` are not readable by the agent.

### 4.4 Deterministic Event Model

For Detir apps, the agent protocol provides:

- Input events with deterministic ordering.
- Focus model with explicit focus targets (no race conditions).
- Navigation model with confirmed state transitions.
- Timing guarantees: action → state confirmation round-trip.
- Deterministic snapshot and replay of agent sessions.

### 4.5 Audit Trail

Every agent action is logged:

```
~/.orus/data/audit/
├── session-2026-03-02T10-30-00.log
└── session-2026-03-02T14-15-00.log
```

Each log entry records: timestamp, agent ID, target app, action type, action payload, result, and state hash before/after. Sessions are replayable from audit logs.

---

## 5. App Model

### 5.1 Detir Apps

Apps are directories under `~/.orus/apps/`:

```
~/.orus/apps/
├── pharmacy/
│   ├── manifest.json
│   └── index.html
├── file-manager/
│   ├── manifest.json
│   └── index.html
└── notes/
    ├── manifest.json
    └── index.html
```

`manifest.json` defines the app:

```json
{
  "name": "Pharmacy",
  "version": "1.0.0",
  "entry": "index.html",
  "icon": "icon.png",
  "permissions": {
    "network": ["api.pharmacy.com"],
    "filesystem": ["downloads"],
    "agent": {
      "read": true,
      "actuate": true,
      "confirm_actions": ["purchase", "submit"]
    }
  }
}
```

Each app renders through Detir (HTML/CSS/JS → parse → Taffy layout → IR → compositor surface). Apps cannot call host APIs. All capabilities are brokered.

### 5.2 CEF Browser

CEF provides web compatibility for sites and apps that require full Chromium rendering (Gmail, banking, e-commerce, etc.). CEF is a sibling native view managed by the compositor.

The agent controls CEF via the accessibility tree pattern (snapshot → ref → action → re-snapshot). The user sees the live browser. Both agent and user interact with the same CEF instance — no sync issues.

### 5.3 Shell and Launcher

The Orus OS shell is rendered by the compositor:

- **Shell bar** — displays running apps, system status, agent status.
- **App launcher** — reads `~/.orus/apps/*/manifest.json`, displays icons, launches apps.
- **Peco** — agent chat/status overlay, always accessible.
- **Detir terminal** — command-line access within the Orus OS sandbox.

All shell UI is Detir-rendered. No host UI frameworks are used for shell content.

---

## 6. GPU and Rendering

### 6.1 GPU Access

Orus OS Universal uses the host GPU directly through wgpu:

| Platform | GPU API | Backend |
|----------|---------|---------|
| macOS | Metal | wgpu metal backend |
| Windows | DirectX 12 / Vulkan | wgpu dx12 or vulkan backend |
| Linux | Vulkan | wgpu vulkan backend |

No GPU passthrough, no VM, no virtio-gpu. Native performance.

### 6.2 Compositor Pipeline

```
Detir App threads → IR surfaces ─┐
                                  ├→ Compositor (frame graph) → wgpu → present
CEF native view ─────────────────┘   (damage-tracked, single swapchain present)
```

The compositor handles z-ordering, focus, window management, damage tracking, and final presentation. CEF is composited as an overlay island at the native view level.

---

## 7. Cross-Platform Build

### 7.1 Shared Rust Core

The following are compiled as a shared library (.dylib / .dll / .so):

- Compositor (wgpu)
- Detir renderer (html5ever, Taffy, Boa, wgpu IR)
- Detir terminal
- Peco
- Agent protocol
- File broker
- Network broker
- Audit logger

### 7.2 Platform Shells

Thin native code per platform:

- **macOS:** Swift/ObjC — NSApplication, NSWindow, NSView management, CEF NSView hosting, App Sandbox integration. ~500-1000 lines.
- **Windows:** C++/C# — Win32 window management, CEF HWND hosting, AppContainer integration. ~500-1000 lines.
- **Linux:** Rust/C — GTK or Wayland client window, CEF window hosting, bubblewrap integration. ~500-1000 lines.

### 7.3 Distribution

| Platform | Format | Size (estimated) |
|----------|--------|-------------------|
| macOS | .app bundle (notarized) | ~250MB (Detir ~49MB + CEF ~200MB) |
| Windows | .exe installer or MSIX | ~250MB |
| Linux | AppImage or Flatpak | ~250MB |

Optional: **Orus OS Universal Lite** without CEF (~49MB) for users who only need Detir-native apps.

---

## 8. Comparison with Orus OS Editions

| | Universal (U) | Dev Box (D) | Edge (E) | Secure (S) |
|---|---|---|---|---|
| **Purpose** | Try Orus OS | Recommended experience | Headless deployment | Safety-critical |
| **Deployment** | Native app on host OS | Bare metal | Bare metal / cloud | Bare metal (seL4) |
| **GPU** | Host GPU (Metal/Vulkan/DX12) | Direct hardware | N/A (headless) | Minimal |
| **CEF** | Yes (sibling native view) | Yes (or Firefox fallback) | No | No |
| **Sandbox** | Brokered (file, network, process) | Full OS isolation | Full OS isolation | seL4 microkernel |
| **IR Protocol** | Same | Same | Same | Same |
| **Agent Protocol** | Same | Same | Same | Same |
| **Base** | Host OS + native shell | Buildroot + Wayland + musl | Buildroot headless | seL4 + Arth binaries |
| **Target user** | Everyone | Developers, homelabs | DevOps, edge | Robotics, industrial |

All editions share the same IR, the same agent protocol, and the same Peco interface. An agent skill developed on Universal runs identically on Dev Box, Edge, or Secure.

---

## 9. Security Posture

### 9.1 What Orus OS Universal Credibly Claims

Orus OS Universal is a sandboxed runtime with brokered file, network, and UI capabilities:

- **File broker:** deny-by-default filesystem access, scoped to `~/.orus/` with per-app subdirectories and quotas.
- **Network broker:** brokered egress with private-network denial by default, DNS brokered, all requests logged.
- **Process supervision:** supervisor tree with resource limits, crash containment, clean termination.
- **Capability-gated agent protocol:** read-only vs actuation, per-app scopes, per-action user grants.
- **Audit trail:** every agent action logged with timestamps, payloads, and state hashes. Sessions replayable.
- **Redaction:** passwords, secrets, and host paths filtered from agent view.

### 9.2 What Orus OS Universal Does Not Claim

- Full kernel-level isolation (that requires Dev Box or Secure editions on bare metal).
- Protection against sandbox escape exploits at the OS level (defense-in-depth via platform sandbox, but not equivalent to a VM or microkernel).
- CEF determinism — CEF web content is operated via accessibility tree, not typed IR.

### 9.3 Comparison with OpenClaw

| | OpenClaw | Orus OS Universal |
|---|---|---|
| Agent filesystem access | Root access to host machine | Scoped to `~/.orus/` via file broker |
| Agent network access | Full access to host + LAN | Brokered egress, private networks denied |
| Browser control | Playwright + screenshots | Detir IR (deterministic) + CEF accessibility (compat) |
| Process model | Node.js daemon + orphaned children | Supervisor tree, clean termination |
| Auditability | Logs at best | Full audit trail, replayable sessions |
| Determinism | LLM guesses, retries | Typed IR, confirmed state transitions |

---

## 10. Roadmap

### Phase 1: Foundation
- [ ] macOS native shell (Swift + NSWindow + NSViews)
- [ ] Rust core as shared library via FFI
- [ ] Compositor rendering to NSView via wgpu (Metal)
- [ ] Multiple Detir apps on separate threads
- [ ] CEF as sibling NSView
- [ ] Detir terminal
- [ ] Peco agent interface
- [ ] Basic agent protocol (WebSocket)

### Phase 2: Sandbox
- [ ] File broker (deny-by-default, `~/.orus/` scoped)
- [ ] Network broker (egress filtering, private network denial)
- [ ] macOS App Sandbox integration
- [ ] Audit logger
- [ ] Agent capability gating

### Phase 3: Cross-Platform
- [ ] Windows native shell (Win32 + HWND)
- [ ] Linux native shell (GTK / Wayland)
- [ ] Windows sandbox (AppContainer)
- [ ] Linux sandbox (bubblewrap / namespaces)
- [ ] Distribution packaging (AppImage, MSIX, .app)

### Phase 4: Agent Ecosystem
- [ ] App marketplace / registry
- [ ] Agent skill sharing
- [ ] Deterministic replay / debugging tools
- [ ] Redaction layer
- [ ] User confirmation flows for high-risk actions

---

*Orus OS Universal: the on-ramp to Orus OS. Try it on your machine. Deploy on bare metal when you're ready.*
