//! The Orus OS panel: the top bar, the dock and the launcher as wlr-layer-shell
//! surfaces, so windows can never cover them. `orus-linux --panel` runs it next to
//! the desktop window (`orus-shell` in `Desktop` mode).

pub mod dock_model;
pub mod dock_state;
pub mod toplevels;

#[cfg(target_os = "linux")]
mod gpu;
#[cfg(target_os = "linux")]
mod handlers;
#[cfg(target_os = "linux")]
mod ipc;
#[cfg(target_os = "linux")]
mod keys;
#[cfg(target_os = "linux")]
mod overlay;
#[cfg(target_os = "linux")]
mod render;
#[cfg(target_os = "linux")]
mod state;
#[cfg(target_os = "linux")]
mod surface;

#[cfg(target_os = "linux")]
pub use state::run;

/// Open or close the running panel's launcher (what the launcher keybinding runs).
#[cfg(target_os = "linux")]
pub fn toggle_launcher() -> anyhow::Result<()> {
    ipc::send(ipc::TOGGLE_LAUNCHER)
}

/// The panel needs Wayland layer-shell, which only Linux compositors offer.
#[cfg(not(target_os = "linux"))]
pub fn run() -> anyhow::Result<()> {
    anyhow::bail!("the panel runs only on Linux, under a layer-shell compositor")
}

#[cfg(not(target_os = "linux"))]
pub fn toggle_launcher() -> anyhow::Result<()> {
    anyhow::bail!("the panel runs only on Linux")
}
