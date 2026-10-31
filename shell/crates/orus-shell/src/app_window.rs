//! App window decorations: title bar with minimize/maximize/close buttons.
//!
//! Each app runs inside a decorated window frame drawn by the compositor.
//! The window can be dragged by its title bar and resized by its edges.
//!
//! ```text
//! +-- App Name ------------------ [-][o][x] -+
//! |                                        |
//! |            Content Area                |
//! |                                        |
//! +----------------------------------------+
//! ```

use crate::palette::{self, color};
use jag_draw::{Brush, FontStyle, Rect, RoundedRadii, RoundedRect, SvgStyle};

/// Title bar height in logical pixels.
pub const TITLE_BAR_HEIGHT: f32 = 48.0;

/// Width of a title bar button; its icon is drawn centered in it.
const BUTTON_W: f32 = 50.0;

/// Gap between the right-most button and the window edge.
const BUTTON_RIGHT_PAD: f32 = 3.0;

/// Side of the square a button icon is drawn in. The icons are 24-unit art whose
/// glyph spans 14 units, so this draws the glyph about 13px wide, as in the concept.
const BUTTON_ICON: f32 = 22.0;

/// Stroke width of the button icons.
const BUTTON_STROKE: f32 = 1.6;

/// Title text size and its distance from the window's left edge.
const TITLE_FONT_SIZE: f32 = 15.0;
const TITLE_LEFT_PAD: f32 = 20.0;

/// Window corner radius.
const CORNER_RADIUS: f32 = 14.0;

/// State of an app window within the Orus OS compositor.
pub struct AppWindow {
    /// Application name shown in the title bar.
    pub title: String,
    /// Window bounds (outer frame including title bar).
    pub bounds: Rect,
    /// Whether this window currently has focus.
    pub focused: bool,
    /// Z-index base for rendering (focused windows get higher values).
    pub z_base: i32,
    /// Saved bounds before maximize (None = not maximized).
    pre_maximize_bounds: Option<Rect>,
    /// A window with no frame of its own: the compositor draws the title bar and
    /// borders, and the content fills the whole window.
    frameless: bool,
}

impl AppWindow {
    /// Create a new app window.
    pub fn new(title: impl Into<String>, bounds: Rect) -> Self {
        Self {
            title: title.into(),
            bounds,
            focused: true,
            z_base: 1000,
            pre_maximize_bounds: None,
            frameless: false,
        }
    }

    /// A window with no title bar or borders, for an app that runs as a window of
    /// its own under the compositor. It starts maximized, so it follows the size of
    /// the window it fills.
    pub fn frameless(title: impl Into<String>, bounds: Rect) -> Self {
        let mut window = Self::new(title, bounds);
        window.frameless = true;
        window.pre_maximize_bounds = Some(bounds);
        window
    }

    /// Whether the window is currently maximized.
    pub fn is_maximized(&self) -> bool {
        self.pre_maximize_bounds.is_some()
    }

    /// Toggle between maximized (filling `viewport`) and restored bounds.
    ///
    /// `viewport` is the app viewport rect — the area below the shell bar.
    /// When maximizing, the window fills the viewport exactly (no padding).
    pub fn toggle_maximize(&mut self, viewport: Rect) {
        if let Some(saved) = self.pre_maximize_bounds.take() {
            // Restore
            self.bounds = saved;
        } else {
            // Maximize — save current bounds, fill the viewport
            self.pre_maximize_bounds = Some(self.bounds);
            self.bounds = viewport;
        }
    }

    /// The rect available for app content (below chrome, inside borders).
    pub fn content_rect(&self) -> Rect {
        if self.frameless {
            return self.bounds;
        }
        let border = 1.0;
        let chrome_h = TITLE_BAR_HEIGHT;
        Rect {
            x: self.bounds.x + border,
            y: self.bounds.y + chrome_h,
            w: (self.bounds.w - border * 2.0).max(0.0),
            h: (self.bounds.h - chrome_h - border).max(0.0),
        }
    }

    /// The title bar rect.
    fn title_bar_rect(&self) -> Rect {
        Rect {
            x: self.bounds.x,
            y: self.bounds.y,
            w: self.bounds.w,
            h: TITLE_BAR_HEIGHT,
        }
    }

    /// Render the window frame and decorations onto the canvas.
    ///
    /// The body background renders at `z_base`, and the title bar chrome
    /// (title, buttons, border) renders at `z_base + CHROME_Z_OFFSET` so
    /// that embedded content (IR scenes with internal z layering up to
    /// ~z_base+10000) sits between the body and the title bar.
    pub fn render(&mut self, canvas: &mut jag_surface::Canvas) {
        let z = self.z_base;
        let pal = palette::get();
        if self.frameless {
            // Only the opaque body: the compositor draws everything else.
            let r = self.bounds;
            canvas.fill_rect(r.x, r.y, r.w, r.h, Brush::Solid(color(pal.surface)), z + 5);
            return;
        }
        // Title bar chrome must sit above all embedded scene content.
        // Scene internal z values can reach up to z_base + ~9860.
        let chrome_z = z + 12000;
        let title_bg = color(if self.focused {
            pal.titlebar
        } else {
            pal.titlebar_inactive
        });

        // --- Window body (rounded rect) ---
        canvas.rounded_rect(
            RoundedRect {
                rect: self.bounds,
                radii: RoundedRadii {
                    tl: CORNER_RADIUS,
                    tr: CORNER_RADIUS,
                    br: CORNER_RADIUS,
                    bl: CORNER_RADIUS,
                },
            },
            Brush::Solid(color(pal.surface)),
            z,
        );
        // Solid fill for the content area so the background is opaque
        // even before IR scene content has loaded (prevents bleed-through
        // from windows behind).
        let cr = self.content_rect();
        canvas.fill_rect(
            cr.x,
            cr.y,
            cr.w,
            cr.h,
            Brush::Solid(color(pal.surface)),
            z + 5,
        );

        // --- Border (1px outline) ---
        // Top-left to top-right (top border)
        canvas.fill_rect(
            self.bounds.x,
            self.bounds.y,
            self.bounds.w,
            1.0,
            Brush::Solid(color(pal.border)),
            chrome_z,
        );
        // Bottom border
        canvas.fill_rect(
            self.bounds.x,
            self.bounds.y + self.bounds.h - 1.0,
            self.bounds.w,
            1.0,
            Brush::Solid(color(pal.border)),
            chrome_z,
        );
        // Left border
        canvas.fill_rect(
            self.bounds.x,
            self.bounds.y,
            1.0,
            self.bounds.h,
            Brush::Solid(color(pal.border)),
            chrome_z,
        );
        // Right border
        canvas.fill_rect(
            self.bounds.x + self.bounds.w - 1.0,
            self.bounds.y,
            1.0,
            self.bounds.h,
            Brush::Solid(color(pal.border)),
            chrome_z,
        );

        // --- Title bar background ---
        let tb = self.title_bar_rect();
        canvas.fill_rect(tb.x, tb.y, tb.w, tb.h, Brush::Solid(title_bg), chrome_z + 1);

        // Title bar bottom separator
        canvas.fill_rect(
            tb.x,
            tb.y + tb.h - 1.0,
            tb.w,
            1.0,
            Brush::Solid(color(pal.border)),
            chrome_z + 2,
        );

        // --- Title bar buttons (right-aligned: minimize, maximize, close) ---
        let icon_color = color(if self.focused { pal.text } else { pal.text_dim });
        for (idx, button) in WindowButton::RIGHT_TO_LEFT.iter().enumerate() {
            let rect = self.button_rect(idx);
            canvas.draw_svg_styled(
                button.icon(),
                [
                    rect.x + (rect.w - BUTTON_ICON) * 0.5,
                    rect.y + (rect.h - BUTTON_ICON) * 0.5,
                ],
                [BUTTON_ICON, BUTTON_ICON],
                SvgStyle::new()
                    .with_stroke(icon_color)
                    .with_stroke_width(BUTTON_STROKE),
                chrome_z + 3,
            );
        }

        // --- Title text (centered in title bar) ---
        let font_size = TITLE_FONT_SIZE;
        let title_x = tb.x + TITLE_LEFT_PAD;
        let title_y = tb.y + (tb.h + font_size) * 0.5 - 2.0;
        canvas.draw_text_run_styled(
            [title_x, title_y],
            self.title.clone(),
            font_size,
            600.0,
            FontStyle::Normal,
            None,
            color(pal.text),
            chrome_z + 3,
        );
    }

    /// Rect of the `idx`-th title bar button counted from the right edge.
    fn button_rect(&self, idx: usize) -> Rect {
        let tb = self.title_bar_rect();
        Rect {
            x: tb.x + tb.w - BUTTON_RIGHT_PAD - (idx as f32 + 1.0) * BUTTON_W,
            y: tb.y,
            w: BUTTON_W,
            h: tb.h,
        }
    }

    /// Hit-test the title bar buttons. Returns which button was hit.
    /// Call with coordinates relative to the window's coordinate space.
    pub fn hit_test_buttons(&self, x: f32, y: f32) -> Option<WindowButton> {
        if self.frameless {
            return None;
        }
        WindowButton::RIGHT_TO_LEFT
            .iter()
            .enumerate()
            .find(|(idx, _)| {
                let r = self.button_rect(*idx);
                x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
            })
            .map(|(_, button)| *button)
    }

    /// Returns true if the point is within the title bar (for drag detection).
    pub fn hit_test_title_bar(&self, x: f32, y: f32) -> bool {
        if self.frameless {
            return false;
        }
        let tb = self.title_bar_rect();
        x >= tb.x && x <= tb.x + tb.w && y >= tb.y && y <= tb.y + tb.h
    }

    /// Returns true if the point is within the window bounds.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.bounds.x
            && x <= self.bounds.x + self.bounds.w
            && y >= self.bounds.y
            && y <= self.bounds.y + self.bounds.h
    }

    /// Hit-test the resize edges. Returns which edge(s) the point is near.
    pub fn hit_test_resize(&self, x: f32, y: f32) -> Option<ResizeEdge> {
        if self.frameless {
            return None;
        }
        let b = &self.bounds;
        let grab = RESIZE_GRAB;
        let near_left = x >= b.x - grab && x <= b.x + grab;
        let near_right = x >= b.x + b.w - grab && x <= b.x + b.w + grab;
        let near_top = y >= b.y - grab && y <= b.y + grab;
        let near_bottom = y >= b.y + b.h - grab && y <= b.y + b.h + grab;
        let in_x = x >= b.x - grab && x <= b.x + b.w + grab;
        let in_y = y >= b.y - grab && y <= b.y + b.h + grab;

        match (
            near_left && in_y,
            near_right && in_y,
            near_top && in_x,
            near_bottom && in_x,
        ) {
            (true, _, true, _) => Some(ResizeEdge::TopLeft),
            (true, _, _, true) => Some(ResizeEdge::BottomLeft),
            (_, true, true, _) => Some(ResizeEdge::TopRight),
            (_, true, _, true) => Some(ResizeEdge::BottomRight),
            (true, _, _, _) => Some(ResizeEdge::Left),
            (_, true, _, _) => Some(ResizeEdge::Right),
            (_, _, true, _) => Some(ResizeEdge::Top),
            (_, _, _, true) => Some(ResizeEdge::Bottom),
            _ => None,
        }
    }

    /// Apply a resize delta. Returns the new bounds, clamped to minimum size.
    pub fn apply_resize(&mut self, edge: ResizeEdge, dx: f32, dy: f32) {
        let min_w = 200.0;
        let min_h = TITLE_BAR_HEIGHT + 60.0;
        let b = &mut self.bounds;

        match edge {
            ResizeEdge::Right => b.w = (b.w + dx).max(min_w),
            ResizeEdge::Bottom => b.h = (b.h + dy).max(min_h),
            ResizeEdge::BottomRight => {
                b.w = (b.w + dx).max(min_w);
                b.h = (b.h + dy).max(min_h);
            }
            ResizeEdge::Left => {
                let new_w = (b.w - dx).max(min_w);
                let actual_dx = b.w - new_w;
                b.x += actual_dx;
                b.w = new_w;
            }
            ResizeEdge::Top => {
                let new_h = (b.h - dy).max(min_h);
                let actual_dy = b.h - new_h;
                b.y += actual_dy;
                b.h = new_h;
            }
            ResizeEdge::TopLeft => {
                let new_w = (b.w - dx).max(min_w);
                let actual_dx = b.w - new_w;
                b.x += actual_dx;
                b.w = new_w;
                let new_h = (b.h - dy).max(min_h);
                let actual_dy = b.h - new_h;
                b.y += actual_dy;
                b.h = new_h;
            }
            ResizeEdge::TopRight => {
                b.w = (b.w + dx).max(min_w);
                let new_h = (b.h - dy).max(min_h);
                let actual_dy = b.h - new_h;
                b.y += actual_dy;
                b.h = new_h;
            }
            ResizeEdge::BottomLeft => {
                let new_w = (b.w - dx).max(min_w);
                let actual_dx = b.w - new_w;
                b.x += actual_dx;
                b.w = new_w;
                b.h = (b.h + dy).max(min_h);
            }
        }
    }
}

/// Resize grab zone width in pixels.
const RESIZE_GRAB: f32 = 5.0;

/// Window control buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowButton {
    Close,
    Minimize,
    Maximize,
}

impl WindowButton {
    /// The order buttons sit in, starting at the right edge.
    pub const RIGHT_TO_LEFT: [WindowButton; 3] = [
        WindowButton::Close,
        WindowButton::Maximize,
        WindowButton::Minimize,
    ];

    /// Name of the embedded icon drawn for this button.
    fn icon(self) -> &'static str {
        match self {
            WindowButton::Close => "window-close.svg",
            WindowButton::Minimize => "window-minimize.svg",
            WindowButton::Maximize => "window-maximize.svg",
        }
    }
}

/// Window resize edges and corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
