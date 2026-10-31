//! The windows the compositor reports (wlr-foreign-toplevel): enough to name the
//! focused app in the bar and to know whether a window covers the dock.

use std::collections::BTreeMap;

/// The wallpaper window's title (see `session/rc.xml`). It is the desktop, not an app.
pub const DESKTOP_TITLE: &str = "Orus OS Universal";

/// `zwlr_foreign_toplevel_handle_v1` state values.
const STATE_MAXIMIZED: u32 = 0;
const STATE_MINIMIZED: u32 = 1;
const STATE_ACTIVATED: u32 = 2;
const STATE_FULLSCREEN: u32 = 3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toplevel {
    pub title: String,
    pub app_id: String,
    pub maximized: bool,
    pub minimized: bool,
    pub activated: bool,
    pub fullscreen: bool,
}

impl Toplevel {
    /// Set the flags from the protocol's state array (native-endian u32s).
    pub fn set_states(&mut self, bytes: &[u8]) {
        self.maximized = false;
        self.minimized = false;
        self.activated = false;
        self.fullscreen = false;
        // `as_chunks` would do, but needs a newer Rust than this workspace requires.
        #[allow(clippy::chunks_exact_to_as_chunks)]
        for chunk in bytes.chunks_exact(4) {
            match u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) {
                STATE_MAXIMIZED => self.maximized = true,
                STATE_MINIMIZED => self.minimized = true,
                STATE_ACTIVATED => self.activated = true,
                STATE_FULLSCREEN => self.fullscreen = true,
                _ => {}
            }
        }
    }

    pub fn is_desktop(&self) -> bool {
        self.title == DESKTOP_TITLE
    }
}

/// Every open window, by an id the caller chooses (the protocol object's id).
#[derive(Debug, Default)]
pub struct Toplevels {
    items: BTreeMap<u32, Toplevel>,
}

impl Toplevels {
    pub fn entry(&mut self, id: u32) -> &mut Toplevel {
        self.items.entry(id).or_default()
    }

    /// Every window with its id, oldest first (the compositor numbers windows as it
    /// creates them).
    pub fn iter(&self) -> impl Iterator<Item = (u32, &Toplevel)> {
        self.items.iter().map(|(id, t)| (*id, t))
    }

    /// The window that has keyboard focus, if it is an app window.
    pub fn active_id(&self) -> Option<u32> {
        self.iter()
            .find(|(_, t)| t.activated && !t.is_desktop() && !t.minimized)
            .map(|(id, _)| id)
    }

    pub fn remove(&mut self, id: u32) {
        self.items.remove(&id);
    }

    /// Whether a window fills the screen, so the dock should get out of its way.
    pub fn covers_screen(&self) -> bool {
        self.items
            .values()
            .any(|t| !t.is_desktop() && !t.minimized && (t.maximized || t.fullscreen))
    }

    /// The title of the focused app window, if any. Chromium shows its app name, not the page title.
    pub fn active_title(&self) -> Option<&str> {
        self.items
            .values()
            .find(|t| t.activated && !t.is_desktop() && !t.minimized)
            .map(|t| {
                if t.app_id.starts_with("chromium") {
                    "Chromium"
                } else {
                    t.title.as_str()
                }
            })
    }
}
