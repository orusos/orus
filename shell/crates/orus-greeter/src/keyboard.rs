//! The keyboard layout the login screen shows.

use std::fs;

/// The label for the first layout in `layouts` (an `XKBLAYOUT` value such as
/// `"us"` or `"de,us"`). Layouts with no name here show their code.
pub fn layout_label(layouts: &str) -> Option<String> {
    let code = layouts.split(',').next()?.trim();
    if code.is_empty() {
        return None;
    }
    Some(match code {
        "us" => "English (US)".to_string(),
        "gb" => "English (UK)".to_string(),
        other => other.to_uppercase(),
    })
}

/// The `XKBLAYOUT` value in `/etc/default/keyboard` text.
pub fn xkb_layout(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("XKBLAYOUT=")?;
        Some(value.trim_matches('"').trim_matches('\'').to_string())
    })
}

/// What the compositor is using: the environment if it names a layout, else the
/// system setting.
pub fn current_label() -> Option<String> {
    let layouts = std::env::var("XKB_DEFAULT_LAYOUT")
        .ok()
        .filter(|l| !l.is_empty())
        .or_else(|| xkb_layout(&fs::read_to_string("/etc/default/keyboard").ok()?))?;
    layout_label(&layouts)
}
