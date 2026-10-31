//! Session persistence for Orus OS internal app windows.
//!
//! Saves/restores the internal window layout (bounds, maximized state,
//! focus) between sessions using brokered storage under `~/.orus/data/`.
//! App-specific content state is NOT stored here — each app manages
//! its own state in `~/.orus/data/<app_id>/state.json`.

use crate::sandbox::SandboxContext;
use serde::{Deserialize, Serialize};

/// Serializable bounds (mirrors jag_draw::Rect).
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
    pub app_id: String,
    pub bounds: Bounds,
    pub maximized: bool,
}

/// Top-level session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub version: u32,
    pub focused_app_id: Option<String>,
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

/// Load saved session state, or `None` if no file exists or it's invalid.
pub fn load(sandbox: &SandboxContext) -> Option<SessionState> {
    let state: SessionState = sandbox.load_system_json("session.json")?;
    if state.version != 1 {
        log::warn!("Unknown session version {}, ignoring", state.version);
        return None;
    }
    Some(state)
}

/// Save session state to brokered system storage.
pub fn save(sandbox: &SandboxContext, state: &SessionState) {
    sandbox.save_system_json("session.json", state);
}

/// Load app-specific state from `~/.orus/data/<app_id>/state.json`.
pub fn load_app_state<T: serde::de::DeserializeOwned>(
    sandbox: &SandboxContext,
    app_id: &str,
) -> Option<T> {
    sandbox.load_app_json(app_id, "state.json")
}

/// Save app-specific state to `~/.orus/data/<app_id>/state.json`.
pub fn save_app_state<T: Serialize>(sandbox: &SandboxContext, app_id: &str, state: &T) {
    sandbox.save_app_json(app_id, "state.json", state);
}
