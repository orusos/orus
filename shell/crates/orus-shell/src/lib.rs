//! Orus OS shell chrome: a minimal OS-level shell bar replacing browser-style toolbar/sidebar.
//!
//! The shell provides a 28px bar at the top of the screen with app name, system clock,
//! agent status, and Orus OS branding. The rest of the window is the app viewport.

pub mod app_kind;
pub mod app_thread;
pub mod app_window;
pub mod appearance_watch;
pub mod dock;
pub mod dock_bar;
pub mod external_app;
pub mod icons;
pub mod launcher;
pub mod layout;
pub mod palette;
pub mod paths;
pub mod platform;
pub mod power_menu;
pub mod runtime;
pub mod sandbox;
pub mod session;
pub mod shell_bar;
pub mod start;
pub mod system_status;
pub mod web_app;
pub mod window_manager;
pub mod window_state;
