# Image Viewer — Design Document

**Goal:** Built-in Orus OS app that displays images from `~/.orus/`. Supports three launch modes: folder picker, programmatic folder open, and programmatic single-file open.

**Architecture:** Standalone crate (`orus-imageviewer`) following FileManager pattern. Canvas-based rendering, `WindowContent::ImageViewer` variant, `AppId::ImageViewer` in launcher.

## Launch Modes

1. **Launcher (no args)** — Opens `rfd::FileDialog` folder picker starting at `~/.orus/`. Shows gallery after selection.
2. **Programmatic folder** — `ImageViewerApp::new_with_folder(path)` — opens directly in Gallery Mode showing images from that folder.
3. **Programmatic file** — `ImageViewerApp::new_with_file(path)` — opens directly in Preview Mode showing that image, with prev/next navigating siblings in the same directory.

All paths validated to be under `~/.orus/`.

## UI Modes

**Gallery Mode:** Grid of image thumbnails (120x120 cells) with filenames below. Scrollable. Click thumbnail → Preview. Toolbar: breadcrumb path + "Change Folder" button.

**Preview Mode:** Full-size image with `Contain` fit, centered. Top bar: back arrow, filename, dimensions. Arrow keys or buttons for prev/next. Escape → Gallery.

## Supported Formats

`.png`, `.jpg`, `.jpeg`, `.webp`, `.gif`, `.svg`

## Crate Structure

```
orus-universal/crates/system-apps/orus-imageviewer/
├── Cargo.toml
└── src/
    ├── lib.rs      — ImageViewerApp state, input handling, launch modes
    └── render.rs   — Canvas rendering (gallery grid, preview, toolbar)
```
