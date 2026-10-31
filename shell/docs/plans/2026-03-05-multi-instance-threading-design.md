# Multi-Instance Per-App Threading Design

**Date:** 2026-03-05
**Status:** Approved

## Problem

Orus OS currently supports one terminal and one browser instance. The launcher focuses the existing window instead of creating new ones. All app logic runs on the main thread. The spec (section 2.2) requires per-app threads with isolated state and compositor-based rendering.

## Architecture

```
Main Thread (Compositor)
  |- WindowManager (focus, z-order, hit-test, drag/resize)
  |- Receives display list snapshots from app threads
  |- Composites all snapshots + shell chrome -> GPU frame
  |- Routes input events to app threads via channels

App Thread 1 (Terminal)
  |- Owns: TerminalCore, TerminalRenderer
  |- Receives: AppInput via mpsc channel
  |- Sends: AppOutput via mpsc channel
  |- Polls PTY continuously, re-renders on change

App Thread 2 (Browser)
  |- Owns: SceneRenderer
  |- Receives: AppInput via mpsc channel
  |- Sends: AppOutput via mpsc channel
  |- Re-renders on input/navigation/animation

App Thread N...
```

## Channel Protocol

### Main -> App Thread

```rust
enum AppInput {
    Key { code, state, mods, text },
    MouseDown { x, y, button },      // coordinates relative to content rect
    MouseUp { x, y, button },
    MouseMove { x, y },
    Scroll { dx, dy },
    Resize { width, height },         // content rect changed
    AddressBarNavigate { url },       // browser only
    Shutdown,                         // graceful exit
}
```

### App Thread -> Main

```rust
enum AppOutput {
    Redraw { display_list: Vec<DrawCommand> },
    TitleChanged { title: String },
    Closed,                           // app exited (e.g. shell died)
}
```

## Display List Snapshot

Each app thread renders into a local `Vec<DrawCommand>` -- a flat list of primitives (rects, text runs, images, rounded rects, shadows). The compositor replays this snapshot into the real Canvas at the correct position/z-offset for that window.

`DrawCommand` mirrors the primitives Canvas already accepts, extracted from engine-core.

## WindowContent Refactor

Replace direct ownership of TerminalCore/SceneRenderer with channel handles:

```rust
enum WindowContent {
    App {
        input_tx: mpsc::Sender<AppInput>,
        output_rx: mpsc::Receiver<AppOutput>,
        last_snapshot: Vec<DrawCommand>,
        thread: JoinHandle<()>,
    },
}
```

## Launcher Changes

- Remove "if already open, just focus" logic
- Each launch spawns a new thread + channels + ManagedWindow
- No limit on number of instances

## Rendering Loop

```
RedrawRequested:
  for each window (back to front):
    1. Drain output_rx -- update last_snapshot if Redraw received
    2. Render AppWindow chrome (title bar, buttons, borders)
    3. Replay last_snapshot into Canvas at content_rect offset + z_base
```

## Thread Lifecycle

- **Spawn**: Launcher creates channels, spawns thread, registers window
- **Run**: Thread loops on input channel, polls internal state, sends snapshots
- **Close**: Main sends Shutdown via channel, joins thread, removes window
- **Crash**: Thread panics -> output_rx disconnects -> main detects, removes window, logs error

## Shared Resources

Already Arc-wrapped and thread-safe:
- `Arc<dyn TextProvider>` -- needed by app threads for text measurement/shaping
- `Arc<wgpu::Device>`, `Arc<wgpu::Queue>` -- not needed by app threads (display list, not GPU)

## What Stays on Main Thread

- Window chrome rendering (AppWindow)
- Shell bar, dock, launcher overlays
- Hit-testing, focus management, drag/resize
- GPU submission (Canvas -> wgpu -> present)
