# AppWindow State Persistence — Design

**Date:** 2026-03-05
**Status:** Approved

## Problem

Internal AppWindows (Terminal, Browser) are created with hardcoded positions and sizes on every launch. Users lose their window arrangement when restarting Orus OS.

## Architecture

Two-layer persistence with separation of concerns:

### 1. Session Layer (WindowManager-owned)

**File:** `~/.orus/session.json`

Stores window layout only — which apps are open, their bounds, z-order, focus state, and maximized state. References apps by `app_id` string. No content state.

```json
{
  "version": 1,
  "focused_app_id": "terminal",
  "windows": [
    {
      "app_id": "terminal",
      "bounds": { "x": 50.0, "y": 60.0, "w": 800.0, "h": 500.0 },
      "maximized": false
    },
    {
      "app_id": "detir-browser",
      "bounds": { "x": 200.0, "y": 80.0, "w": 800.0, "h": 600.0 },
      "maximized": false
    }
  ]
}
```

### 2. App Layer (app-owned)

Each app manages its own state in `~/.orus/apps/<app_id>/state.json`:

- **terminal:** `{ "cwd": "/Users/..." }`
- **detir-browser:** `{ "url": "https://..." }`

Apps are responsible for saving and loading their own state. The session layer never reads or writes app state files.

### 3. Outer Window (unchanged)

**File:** `~/.orus/window_state.json`

Existing platform-specific persistence for the macOS host window. Not touched by this change.

## File Layout

```
~/.orus/
├── window_state.json          # outer macOS window (existing, unchanged)
├── session.json               # internal window layout (NEW)
└── apps/
    ├── terminal/
    │   └── state.json         # terminal's own state
    └── detir-browser/
        └── state.json         # browser's own state
```

## Save Trigger

On exit only (`CloseRequested` handler), matching existing outer window persistence pattern.

## Restore Flow

1. Load `session.json` -> get window list with bounds
2. For each window entry, create `AppWindow` with saved bounds
3. Each app loads its own `state.json` to initialize content
4. Focus the window matching `focused_app_id`
5. If no session file exists (first run), fall back to current hardcoded defaults

## Key Types

### In `orus-shell/src/session.rs` (cross-platform)

- `SessionState` — top-level: version, focused_app_id, windows vec
- `WindowEntry` — per-window: app_id, bounds, maximized
- `save(state: &SessionState)` / `load() -> Option<SessionState>`

### In app code

Each app type has its own simple state struct + save/load. No shared trait needed yet (only two app types).

## Scope

- `orus-shell/src/session.rs` — new module for session persistence
- `orus-shell/src/lib.rs` — export session module
- `orus-shell/Cargo.toml` — add serde, serde_json deps
- `orus-macos/src/main.rs` — wire save on exit, load on startup
- App-specific state save/load in respective locations
