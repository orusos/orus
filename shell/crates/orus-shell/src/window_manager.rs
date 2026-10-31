//! Window manager for Orus OS: manages multiple `ManagedWindow` instances
//! with focus tracking, z-ordering, and window lifecycle.
//!
//! Each window has a unique `WindowId` and contains either a Terminal or
//! an IR scene (browser). The WindowManager owns the z-layer gap between
//! windows and provides iteration in z-order for rendering.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use jag_draw::Rect;

use crate::app_thread::{
    AppInput, AppThreadHandle, FileManagerState, TerminalState, TextEditState, ViewerState,
};
use crate::app_window::AppWindow;
use crate::session::{Bounds, SessionState, WindowEntry};

/// Unique identifier for a managed window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u32);

/// A managed window within the Orus OS compositor.
pub struct ManagedWindow {
    pub id: WindowId,
    pub app_id: String,
    pub app_window: AppWindow,
    pub content: WindowContent,
}

/// Content hosted inside a managed window.
#[allow(clippy::large_enum_variant)]
pub enum WindowContent {
    Terminal {
        state: Arc<Mutex<TerminalState>>,
        thread: AppThreadHandle,
    },
    TextEdit {
        state: Arc<Mutex<TextEditState>>,
        thread: AppThreadHandle,
    },
    FileManager {
        state: Arc<Mutex<FileManagerState>>,
        thread: AppThreadHandle,
    },
    Viewer {
        state: Arc<Mutex<ViewerState>>,
        thread: AppThreadHandle,
    },
}

impl WindowContent {
    /// Returns a human-readable label for the shell bar.
    pub fn label(&self) -> &str {
        match self {
            Self::Terminal { .. } => "Terminal",
            Self::TextEdit { .. } => "Text Edit",
            Self::FileManager { .. } => "Files",
            Self::Viewer { .. } => "Viewer",
        }
    }

    /// Check whether the background thread flagged a redraw.
    /// Atomically clears the flag and returns the previous value.
    pub fn check_redraw(&self) -> bool {
        let flags = match self {
            Self::Terminal { thread, .. } => &thread.flags,
            Self::TextEdit { thread, .. } => &thread.flags,
            Self::FileManager { thread, .. } => &thread.flags,
            Self::Viewer { thread, .. } => &thread.flags,
        };
        flags.needs_redraw.swap(false, Ordering::Relaxed)
    }

    /// Check whether the app process has exited (e.g. PTY shell quit).
    pub fn has_exited(&self) -> bool {
        let flags = match self {
            Self::Terminal { thread, .. } => &thread.flags,
            Self::TextEdit { thread, .. } => &thread.flags,
            Self::FileManager { thread, .. } => &thread.flags,
            Self::Viewer { thread, .. } => &thread.flags,
        };
        flags.exited.load(Ordering::Relaxed)
    }

    /// Send an input event to the app's background thread.
    pub fn send_input(&self, input: AppInput) {
        let tx = match self {
            Self::Terminal { thread, .. } => &thread.input_tx,
            Self::TextEdit { thread, .. } => &thread.input_tx,
            Self::FileManager { thread, .. } => &thread.input_tx,
            Self::Viewer { thread, .. } => &thread.input_tx,
        };
        let _ = tx.send(input);
    }
}

/// Manages the set of open windows, focus, and z-ordering.
pub struct WindowManager {
    windows: Vec<ManagedWindow>,
    next_id: u32,
    focused: Option<WindowId>,
    /// Z-gap between the front and back window layers.
    z_gap: i32,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            next_id: 1,
            focused: None,
            z_gap: 20000,
        }
    }
}

impl WindowManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a window and return its assigned ID. Does NOT auto-focus.
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

    /// Remove a window by ID. If it was focused, focus shifts to the next window.
    pub fn remove(&mut self, id: WindowId) {
        self.windows.retain(|mw| mw.id != id);
        if self.focused == Some(id) {
            self.focused = self.windows.last().map(|mw| mw.id);
            self.update_z_order();
        }
    }

    /// Focus a window by ID. Updates z-ordering.
    pub fn focus(&mut self, id: WindowId) {
        if !self.windows.iter().any(|mw| mw.id == id) {
            return;
        }
        self.focused = Some(id);
        self.update_z_order();
    }

    /// Get the currently focused window ID.
    pub fn focused_id(&self) -> Option<WindowId> {
        self.focused
    }

    /// Get a reference to a window by ID.
    pub fn get(&self, id: WindowId) -> Option<&ManagedWindow> {
        self.windows.iter().find(|mw| mw.id == id)
    }

    /// Get a mutable reference to a window by ID.
    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut ManagedWindow> {
        self.windows.iter_mut().find(|mw| mw.id == id)
    }

    /// Get a mutable reference to the focused window.
    pub fn focused_mut(&mut self) -> Option<&mut ManagedWindow> {
        let id = self.focused?;
        self.windows.iter_mut().find(|mw| mw.id == id)
    }

    /// Get a reference to the focused window.
    pub fn focused_ref(&self) -> Option<&ManagedWindow> {
        let id = self.focused?;
        self.windows.iter().find(|mw| mw.id == id)
    }

    /// Iterate windows back-to-front (unfocused first, focused last).
    /// This is the render order.
    pub fn iter_back_to_front(&self) -> impl Iterator<Item = &ManagedWindow> {
        let focused = self.focused;
        // Unfocused windows first, then focused window
        self.windows
            .iter()
            .filter(move |mw| Some(mw.id) != focused)
            .chain(self.windows.iter().filter(move |mw| Some(mw.id) == focused))
    }

    /// Iterate windows back-to-front mutably (for rendering).
    pub fn iter_back_to_front_mut(&mut self) -> Vec<&mut ManagedWindow> {
        let focused = self.focused;
        // Sort: unfocused first, focused last
        self.windows
            .sort_by_key(|mw| if Some(mw.id) == focused { 1 } else { 0 });
        self.windows.iter_mut().collect()
    }

    /// Iterate windows front-to-back (focused first).
    /// This is the hit-test order.
    pub fn iter_front_to_back(&self) -> impl Iterator<Item = &ManagedWindow> {
        let focused = self.focused;
        self.windows
            .iter()
            .filter(move |mw| Some(mw.id) == focused)
            .chain(self.windows.iter().filter(move |mw| Some(mw.id) != focused))
    }

    /// Find the topmost window containing the point (x, y).
    /// Checks focused window first, then others in order.
    pub fn window_at(&self, x: f32, y: f32) -> Option<WindowId> {
        for mw in self.iter_front_to_back() {
            let b = mw.app_window.bounds;
            if x >= b.x && x < b.x + b.w && y >= b.y && y < b.y + b.h {
                return Some(mw.id);
            }
        }
        None
    }

    /// Cycle focus to the next window (Ctrl+Tab behavior).
    pub fn cycle_focus(&mut self) {
        if self.windows.len() < 2 {
            return;
        }
        let current_idx = self
            .focused
            .and_then(|id| self.windows.iter().position(|mw| mw.id == id))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % self.windows.len();
        let next_id = self.windows[next_idx].id;
        self.focus(next_id);
    }

    /// Number of managed windows.
    pub fn len(&self) -> usize {
        self.windows.len()
    }

    /// Whether there are no managed windows.
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Iterate over all windows (for building dock entries, etc.).
    pub fn windows(&self) -> impl Iterator<Item = &ManagedWindow> {
        self.windows.iter()
    }

    /// Check if a window with the given ID exists.
    pub fn contains(&self, id: WindowId) -> bool {
        self.windows.iter().any(|mw| mw.id == id)
    }

    /// Update z-ordering: each window gets a unique z layer.
    /// Unfocused windows are stacked with `z_gap` between them,
    /// and the focused window sits on top.
    fn update_z_order(&mut self) {
        let focused = self.focused;
        let mut z = 0i32;
        // Unfocused windows first (in their current order)
        for mw in &mut self.windows {
            if Some(mw.id) != focused {
                mw.app_window.z_base = z;
                mw.app_window.focused = false;
                z += self.z_gap;
            }
        }
        // Focused window on top
        for mw in &mut self.windows {
            if Some(mw.id) == focused {
                mw.app_window.z_base = z;
                mw.app_window.focused = true;
            }
        }
    }

    /// Resize all maximized windows to fit a new viewport.
    pub fn resize_maximized(&mut self, viewport: Rect) {
        for mw in &mut self.windows {
            if mw.app_window.is_maximized() {
                mw.app_window.bounds = viewport;
            }
        }
    }

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
}
