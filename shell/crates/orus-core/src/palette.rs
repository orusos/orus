//! Orus OS colors as sRGB hex for the current theme and accent, shared by the
//! shell's own chrome and the system apps.
//!
//! Surfaces, accents, dots and the dock are sampled from the dark and light desktop
//! concepts (`ORUS/assets/concepts/orus-desktop-concept.png` and
//! `orus-desktop-light-concept.png`). `titlebar_inactive`, `text` and `text_dim`
//! are chosen by eye to sit with them; the concepts have no flat patch of those.
//! Accent and selection colors come from `orus_core::appearance`, which the
//! Settings app reads too.

use std::sync::RwLock;

use crate::appearance::{Accent, Theme, accent_color, selection_color, surface_color};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub theme: Theme,
    pub bar: u32,
    pub bar_border: u32,
    pub titlebar: u32,
    pub titlebar_inactive: u32,
    /// Window content, sidebars, and the desktop behind a missing wallpaper.
    pub surface: u32,
    pub border: u32,
    pub text: u32,
    pub text_dim: u32,
    /// Focus and active states only.
    pub accent: u32,
    /// The selected row or tile.
    pub selection: u32,
    pub dock: u32,
    pub dock_border: u32,
    pub dot_running: u32,
    pub dot_idle: u32,
    /// Sidebars (Files).
    pub sidebar: u32,
    /// The selected tab, and tabs not selected (Files).
    pub tab_active: u32,
    pub tab_inactive: u32,
    /// Input fields and pills: the path bar, search (Files).
    pub field: u32,
}

/// The concepts' teal running dot, which differs a little from the teal swatch.
const DARK_TEAL_DOT: u32 = 0x75ccc3;
const LIGHT_TEAL_DOT: u32 = 0x198c8b;

impl Palette {
    pub fn new(theme: Theme, accent: Accent) -> Self {
        let is_teal = accent == Accent::Teal;
        let selection = selection_color(theme, accent);
        let surface = surface_color(theme);
        let accent = accent_color(theme, accent);
        match theme {
            Theme::Dark => Self {
                theme,
                bar: 0x1c272d,
                bar_border: 0x293740,
                titlebar: 0x1c2427,
                titlebar_inactive: 0x161c1f,
                surface,
                border: 0x2c3338,
                text: 0xe6ebee,
                text_dim: 0x9aa5ab,
                accent,
                selection,
                dock: 0x181e22,
                dock_border: 0x30373b,
                dot_running: if is_teal { DARK_TEAL_DOT } else { accent },
                dot_idle: 0x69757f,
                sidebar: 0x1b2226,
                tab_active: 0x2b3237,
                tab_inactive: 0x151b1e,
                field: 0x272d32,
            },
            Theme::Light => Self {
                theme,
                bar: 0xf9fcfd,
                bar_border: 0xd3e3ed,
                titlebar: 0xf1f3f6,
                titlebar_inactive: 0xf6f7f9,
                surface,
                border: 0xe1e3e7,
                text: 0x1c2227,
                text_dim: 0x5d6870,
                accent,
                selection,
                dock: 0xe9eef0,
                dock_border: 0xc3d0d4,
                dot_running: if is_teal { LIGHT_TEAL_DOT } else { accent },
                dot_idle: 0xc1c8d0,
                sidebar: 0xf9f9fb,
                tab_active: 0xfbfcfe,
                tab_inactive: 0xf4f5f8,
                field: 0xfafafc,
            },
        }
    }
}

static CURRENT: RwLock<Option<Palette>> = RwLock::new(None);

/// The palette in use; dark teal until `set` is called.
pub fn get() -> Palette {
    CURRENT
        .read()
        .ok()
        .and_then(|p| *p)
        .unwrap_or_else(|| Palette::new(Theme::Dark, Accent::Teal))
}

pub fn set(palette: Palette) {
    if let Ok(mut current) = CURRENT.write() {
        *current = Some(palette);
    }
}
