# Standalone IR App Windows in Orus OS

**Date**: 2026-03-05
**Status**: Approved

## Goal

Add a "Workflow Editor" entry to the Orus OS launcher that opens an IR package as a standalone app window — no address bar, no dock, no navigation chrome. Just the Orus OS title bar (traffic lights + app name) with IR content filling the content area.

## Package Location

`~/.orus/apps/workflow-editor/` — user copies the IR package manually. Future app installer will automate this from a forge/repo.

## Changes

### 1. Launcher — add Workflow Editor entry (`launcher.rs`)
- New `AppEntry` with `AppId::IrApp { url }` pointing to `~/.orus/apps/workflow-editor`
- Path expanded at runtime via `dirs::home_dir()`
- Purple icon color, label "Workflow Editor"

### 2. IrApp launch handler — use plain window (`main.rs`)
- Change `AppWindow::new_with_address_bar("IR App", bounds)` to `AppWindow::new(<title>, bounds)`
- Extract title from directory name or manifest
- `SceneRenderer::new()` with `chrome_hidden=true`, `dock_disabled=true`
- Background browser thread via `create_browser()` (unchanged)

### 3. Shell bar label — dynamic IrScene label (`window_manager.rs`)
- Add `title: String` to `WindowContent::IrScene` so shell bar shows app name
- Browser windows pass "Detir Browser", standalone apps pass their app name

### What stays the same
- `SceneRenderer` — no changes
- `AppWindow::new()` — already exists without address bar
- `DetirBrowser` launch handler — completely untouched, keeps address bar
- Input routing, window management — unchanged

### Architecture

```
Launcher -> "Workflow Editor" clicked
  -> Expand ~/.orus/apps/workflow-editor/
  -> SceneRenderer::new(path, provider, w, h)
  -> scene.set_chrome_hidden(true) + set_dock_disabled(true)
  -> AppWindow::new("Workflow Editor", bounds)  // no address bar
  -> create_browser(scene) -> background thread
  -> wm.add("workflow-editor", aw, content)
```

### Separation from DetirBrowser

```
DetirBrowser -> AppWindow::new_with_address_bar("Browser", bounds)  // unchanged
IrApp        -> AppWindow::new("Workflow Editor", bounds)           // no address bar
```
