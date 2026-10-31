//! Window state persistence for Orus OS.
//!
//! Saves/restores the outer host window size and position between sessions
//! using brokered system storage under `~/.orus/data/system/window_state.json`.

use serde::{Deserialize, Serialize};

use crate::sandbox::SandboxContext;

/// Persisted window geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1400.0,
            height: 900.0,
            x: -1,
            y: -1,
            maximized: false,
        }
    }
}

impl WindowState {
    /// Returns `true` if the stored size looks valid (positive and non-tiny).
    pub fn has_valid_size(&self) -> bool {
        self.width >= 200.0 && self.height >= 150.0
    }

    /// Returns `true` if a position was explicitly saved (not the sentinel -1).
    pub fn has_valid_position(&self) -> bool {
        self.x != -1 || self.y != -1
    }
}

/// Load saved window state, or return `None` if no state file exists.
pub fn load(sandbox: &SandboxContext) -> Option<WindowState> {
    sandbox.load_system_json("window_state.json")
}

/// Save window state to brokered system storage.
pub fn save(sandbox: &SandboxContext, state: &WindowState) {
    sandbox.save_system_json("window_state.json", state);
}
