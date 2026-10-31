//! Drawing. Colors come from `orus_core::palette` for the current theme and
//! accent; sizes and positions from `layout`.

mod chrome;
mod items;
mod overlay;
mod tabs;

use jag_draw::{Brush, ColorLinPremul, FontStyle, Rect, RoundedRadii, RoundedRect, Transform2D};

use crate::FileManagerApp;

pub(crate) type Canvas = jag_surface::Canvas;

/// Text is placed by its baseline. A line of mixed text looks centered on a
/// point when the baseline sits this fraction of the font size below it (half of
/// a typical cap height); the concept's label centers were read the same way.
const BASELINE_BELOW_CENTER: f32 = 0.36;

pub(crate) fn col(hex: u32) -> ColorLinPremul {
    ColorLinPremul::from_srgba_u8([(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255])
}

pub(crate) fn rrect(rect: Rect, radius: f32) -> RoundedRect {
    RoundedRect {
        rect,
        radii: RoundedRadii {
            tl: radius,
            tr: radius,
            br: radius,
            bl: radius,
        },
    }
}

pub(crate) fn fill(canvas: &mut Canvas, rect: Rect, radius: f32, hex: u32, z: i32) {
    canvas.rounded_rect(rrect(rect, radius), Brush::Solid(col(hex)), z);
}

/// A 1px border drawn as a rect behind a slightly smaller fill.
pub(crate) fn bordered(
    canvas: &mut Canvas,
    rect: Rect,
    radius: f32,
    fill_hex: u32,
    border_hex: u32,
    z: i32,
) {
    fill(canvas, rect, radius, border_hex, z);
    let inner = Rect {
        x: rect.x + 1.0,
        y: rect.y + 1.0,
        w: rect.w - 2.0,
        h: rect.h - 2.0,
    };
    fill(canvas, inner, (radius - 1.0).max(0.0), fill_hex, z + 1);
}

/// Draw `text` starting at `x`, vertically centered on `center_y`.
pub(crate) fn text(
    canvas: &mut Canvas,
    (x, center_y): (f32, f32),
    text: &str,
    size: f32,
    weight: f32,
    hex: u32,
    z: i32,
) {
    canvas.draw_text_run_styled(
        [x, center_y + size * BASELINE_BELOW_CENTER],
        text.to_string(),
        size,
        weight,
        FontStyle::Normal,
        None,
        col(hex),
        z,
    );
}

/// Draw `text` centered on `center_x`; returns its width.
pub(crate) fn text_centered(
    canvas: &mut Canvas,
    (center_x, center_y): (f32, f32),
    label: &str,
    size: f32,
    weight: f32,
    hex: u32,
    z: i32,
) {
    let w = canvas.measure_text_width(label, size);
    text(
        canvas,
        (center_x - w * 0.5, center_y),
        label,
        size,
        weight,
        hex,
        z,
    );
}

/// Shorten `label` with an ellipsis until it fits `max_w`.
pub(crate) fn fit(canvas: &Canvas, label: &str, size: f32, max_w: f32) -> String {
    if canvas.measure_text_width(label, size) <= max_w {
        return label.to_string();
    }
    let mut chars: Vec<char> = label.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "\u{2026}";
        if canvas.measure_text_width(&candidate, size) <= max_w {
            return candidate;
        }
    }
    "\u{2026}".to_string()
}

impl FileManagerApp {
    /// Render the file manager into the canvas.
    pub fn render(&self, canvas: &mut Canvas, w: f32, h: f32, z_base: i32) {
        canvas.push_transform(Transform2D::translate(self.origin_x, self.origin_y));
        let pal = orus_core::palette::get();
        fill(
            canvas,
            Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
            },
            0.0,
            pal.surface,
            z_base,
        );

        self.render_items(canvas, w, h, z_base + 10);
        self.render_sidebar(canvas, &pal, h, z_base + 40);
        self.render_toolbar(canvas, &pal, w, z_base + 40);
        self.render_tabs(canvas, &pal, w, z_base + 60);
        self.render_status(canvas, &pal, w, h, z_base + 60);
        self.render_overlays(canvas, &pal, w, h, z_base + 100);

        canvas.pop_transform();
    }
}
