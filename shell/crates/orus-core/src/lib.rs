//! Orus OS Core — app manifest, package loading, and multi-app management.
//!
//! This crate provides Orus OS-specific types that sit on top of the Detir stack.
//! Input events, rendering and windowing live in the shell crate; apps run in `waid-shell`.

pub mod app;
pub mod appearance;
pub mod design;
pub mod desktop_apps;
pub mod display;
pub mod input;

pub mod installed_apps;
pub mod palette;
pub mod system_status;
