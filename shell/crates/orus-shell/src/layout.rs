//! Orus OS shell layout: computes rects for the shell bar and app viewport.
//!
//! ```text
//! +---------------------------------------+
//! | Shell Bar (36px)                       |
//! +---------------------------------------+
//! | App Viewport (rest of window)          |
//! |                                        |
//! +---------------------------------------+
//! ```

use jag_draw::Rect;

/// Height of the shell bar in logical pixels.
pub const SHELL_BAR_HEIGHT: f32 = 36.0;

/// Layout rects for the Orus OS shell.
#[derive(Debug, Clone, Copy)]
pub struct ShellLayout {
    /// Fixed 36px-height strip at the top of the window.
    pub shell_bar: Rect,
    /// Remaining space below the shell bar for the active application. The dock
    /// floats over it and slides away under a maximized window.
    pub app_viewport: Rect,
}

impl ShellLayout {
    /// Compute the shell layout for a given window size.
    ///
    /// The shell bar occupies the top 36 logical pixels; the app viewport
    /// fills the remainder.
    pub fn calculate(window_width: u32, window_height: u32) -> Self {
        Self::with_bar(window_width, window_height, SHELL_BAR_HEIGHT)
    }

    /// The layout of an app that runs as a window of its own: no shell bar, so the
    /// whole window is the app viewport.
    pub fn without_bar(window_width: u32, window_height: u32) -> Self {
        Self::with_bar(window_width, window_height, 0.0)
    }

    fn with_bar(window_width: u32, window_height: u32, bar_height: f32) -> Self {
        let w = window_width as f32;
        let h = window_height as f32;

        let bar_h = bar_height.min(h);

        Self {
            shell_bar: Rect {
                x: 0.0,
                y: 0.0,
                w,
                h: bar_h,
            },
            app_viewport: Rect {
                x: 0.0,
                y: bar_h,
                w,
                h: (h - bar_h).max(0.0),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_layout() {
        let layout = ShellLayout::calculate(1280, 800);
        assert_eq!(layout.shell_bar.x, 0.0);
        assert_eq!(layout.shell_bar.y, 0.0);
        assert_eq!(layout.shell_bar.w, 1280.0);
        assert_eq!(layout.shell_bar.h, 36.0);

        assert_eq!(layout.app_viewport.x, 0.0);
        assert_eq!(layout.app_viewport.y, 36.0);
        assert_eq!(layout.app_viewport.w, 1280.0);
        assert_eq!(layout.app_viewport.h, 764.0);
    }

    #[test]
    fn tiny_window_clamps_shell_bar() {
        let layout = ShellLayout::calculate(100, 10);
        // Shell bar should not exceed window height.
        assert_eq!(layout.shell_bar.h, 10.0);
        assert_eq!(layout.app_viewport.h, 0.0);
    }

    #[test]
    fn zero_size_window() {
        let layout = ShellLayout::calculate(0, 0);
        assert_eq!(layout.shell_bar.w, 0.0);
        assert_eq!(layout.shell_bar.h, 0.0);
        assert_eq!(layout.app_viewport.w, 0.0);
        assert_eq!(layout.app_viewport.h, 0.0);
    }
}
