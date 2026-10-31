//! Keyboard events for the launcher.

use orus_shell::launcher::LauncherKey;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym};

/// The launcher key for `event`, if it is one the launcher reads.
pub fn launcher_key(event: &KeyEvent) -> Option<LauncherKey> {
    match event.keysym {
        Keysym::Escape => Some(LauncherKey::Escape),
        Keysym::Return | Keysym::KP_Enter => Some(LauncherKey::Enter),
        Keysym::Left => Some(LauncherKey::ArrowLeft),
        Keysym::Right => Some(LauncherKey::ArrowRight),
        Keysym::Up => Some(LauncherKey::ArrowUp),
        Keysym::Down => Some(LauncherKey::ArrowDown),
        Keysym::BackSpace => Some(LauncherKey::Backspace),
        _ => event
            .utf8
            .as_deref()
            .and_then(|text| text.chars().next())
            .filter(|c| !c.is_control())
            .map(LauncherKey::Char),
    }
}
