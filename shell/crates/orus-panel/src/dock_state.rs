//! When the dock is out. It stays out unless a window covers the screen; then it
//! hides, and comes back while the pointer is at the bottom edge or over it.

use std::time::{Duration, Instant};

/// How long the dock stays after the pointer leaves it, so a pointer resting in its
/// bottom margin does not make it flicker.
pub const HIDE_DELAY: Duration = Duration::from_millis(300);

#[derive(Debug)]
pub struct DockVisibility {
    open: bool,
    pointer_in: bool,
    covered: bool,
    hide_at: Option<Instant>,
}

impl Default for DockVisibility {
    fn default() -> Self {
        Self {
            open: true,
            pointer_in: false,
            covered: false,
            hide_at: None,
        }
    }
}

impl DockVisibility {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// A window now covers the screen, or no longer does.
    pub fn set_covered(&mut self, covered: bool, now: Instant) {
        self.covered = covered;
        if !covered {
            self.open = true;
            self.hide_at = None;
        } else if self.open && !self.pointer_in {
            self.hide_at = Some(now + HIDE_DELAY);
        }
    }

    pub fn pointer_entered(&mut self) {
        self.pointer_in = true;
        self.hide_at = None;
        self.open = true;
    }

    pub fn pointer_left(&mut self, now: Instant) {
        self.pointer_in = false;
        if self.covered && self.open {
            self.hide_at = Some(now + HIDE_DELAY);
        }
    }

    /// Hide the dock if its delay has run out. Returns true if it just hid.
    pub fn tick(&mut self, now: Instant) -> bool {
        match self.hide_at {
            Some(at) if now >= at && self.covered && !self.pointer_in => {
                self.hide_at = None;
                self.open = false;
                true
            }
            Some(at) if now >= at => {
                self.hide_at = None;
                false
            }
            _ => false,
        }
    }
}
