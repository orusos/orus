//! Draws the login screen onto a jag canvas.

use std::path::Path;

use jag_draw::{
    Brush, ColorLinPremul, FontStyle, ImageFitMode, Rect, RoundedRadii, RoundedRect, SvgStyle,
};
use jag_surface::Canvas;
use orus_core::system_status::Network;

use crate::form::{Field, Form, Status};
use crate::layout::{CIRCLE_D, EYE_SIZE, FORM_H, Layout, MENU_ROW_H, PILL_H};

pub const BACKGROUND: u32 = 0x10161a;

/// Shown under the name while a login is under way; Enter is the only way to start one.
const SIGNING_IN: &str = "Signing in…";

const FIELD_FILL: u32 = 0x171e25;
const FIELD_BORDER: u32 = 0x4b5860;
const PILL_FILL: u32 = 0x151c22;
const PILL_BORDER: u32 = 0x2e363e;
const CIRCLE_FILL: u32 = 0x12191f;
const CIRCLE_BORDER: u32 = 0x2b333a;
const TEXT: u32 = 0xf4f7f8;
const DATE: u32 = 0xd5dcdf;
const DIM: u32 = 0xaab5ba;
const PLACEHOLDER: u32 = 0x8f9aa0;
const ERROR: u32 = 0xff8a8a;

/// The fields are pills.
const FIELD_RADIUS: f32 = FORM_H * 0.5;
const FIELD_TEXT: f32 = 20.0;
const FIELD_PAD: f32 = 30.0;
/// Space kept free at the field's right for the eye icon.
const FIELD_RIGHT_PAD: f32 = 64.0;
/// The clock line at the top: the date and the time on one line.
const TOP_LINE_SIZE: f32 = 20.0;
const NAME_SIZE: f32 = 32.0;
/// Errors and "Signing in…" under the field.
const MESSAGE_SIZE: f32 = 18.0;
const SMALL_TEXT: f32 = 17.0;
const WORDMARK_SIZE: f32 = 26.0;
const PILL_ICON: f32 = 26.0;
const PILL_PAD: f32 = 16.0;
const PILL_ICON_GAP: f32 = 14.0;
const CIRCLE_ICON: f32 = 24.0;
const BULLET_D: f32 = 10.0;
const BULLET_PITCH: f32 = 20.0;
const CARET_W: f32 = 1.5;
const ICON_STROKE: f32 = 1.6;
/// Baseline offset used across Orus OS to center text on a line.
const BASELINE_NUDGE: f32 = -2.0;

/// The vignette over the wallpaper: black at these opacities from the center
/// outward. Fitted to the concept's wallpaper, whose screen colors are about 52%
/// of the original behind the form and 70% at the edges; the canvas blends in
/// linear light, so the opacities are higher than those ratios suggest. A round
/// gradient cannot be stronger sideways than up and down, so the top and bottom
/// come out a little darker than the concept's.
const VIGNETTE: [(f32, f32); 4] = [(0.0, 0.72), (0.5, 0.62), (0.86, 0.50), (1.0, 0.46)];
const VIGNETTE_CENTER_Y: f32 = 400.0;
const VIGNETTE_RADIUS: f32 = 900.0;

const Z_WALLPAPER: i32 = 0;
const Z_VIGNETTE: i32 = 1;
const Z: i32 = 10;

/// What the screen shows besides the form.
pub struct Scene<'a> {
    pub clock: &'a str,
    pub date: &'a str,
    pub wallpaper: Option<&'a Path>,
    pub keyboard: Option<&'a str>,
    pub network: Network,
    pub power_menu_open: bool,
}

fn color(hex: u32) -> ColorLinPremul {
    ColorLinPremul::from_srgba_u8([(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255])
}

fn round(rect: Rect, radius: f32) -> RoundedRect {
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

fn grow(rect: Rect, by: f32) -> Rect {
    Rect {
        x: rect.x - by,
        y: rect.y - by,
        w: rect.w + by * 2.0,
        h: rect.h + by * 2.0,
    }
}

fn text(canvas: &mut Canvas, x: f32, center_y: f32, s: &str, size: f32, weight: f32, hex: u32) {
    canvas.draw_text_run_styled(
        [x, center_y + size * 0.5 + BASELINE_NUDGE],
        s.to_string(),
        size,
        weight,
        FontStyle::Normal,
        None,
        color(hex),
        Z + 3,
    );
}

fn centered_text(canvas: &mut Canvas, cx: f32, cy: f32, s: &str, size: f32, weight: f32, hex: u32) {
    let w = canvas.measure_text_width(s, size);
    text(canvas, cx - w * 0.5, cy, s, size, weight, hex);
}

/// A box with a 1px border; the border is a slightly larger box behind it.
fn boxed(canvas: &mut Canvas, rect: Rect, radius: f32, fill: u32, border: u32) {
    canvas.rounded_rect(
        round(grow(rect, 1.0), radius + 1.0),
        Brush::Solid(color(border)),
        Z,
    );
    canvas.rounded_rect(round(rect, radius), Brush::Solid(color(fill)), Z + 1);
}

fn line_icon(canvas: &mut Canvas, name: &str, x: f32, y: f32, size: f32, hex: u32) {
    canvas.draw_svg_styled(
        name,
        [x, y],
        [size, size],
        SvgStyle::new()
            .with_stroke(color(hex))
            .with_stroke_width(ICON_STROKE),
        Z + 3,
    );
}

pub fn render(canvas: &mut Canvas, viewport: Rect, form: &Form, scene: &Scene) {
    let l = Layout::new(viewport);
    let cx = viewport.x + viewport.w * 0.5;

    if let Some(path) = scene.wallpaper {
        canvas.draw_image(
            path.to_path_buf(),
            [viewport.x, viewport.y],
            [viewport.w, viewport.h],
            ImageFitMode::Cover,
            Z_WALLPAPER,
        );
    }
    let shade = |a: f32| ColorLinPremul {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a,
    };
    canvas.fill_rect(
        viewport.x,
        viewport.y,
        viewport.w,
        viewport.h,
        Brush::RadialGradient {
            center: [cx, VIGNETTE_CENTER_Y],
            radius: VIGNETTE_RADIUS,
            stops: VIGNETTE.iter().map(|(t, a)| (*t, shade(*a))).collect(),
        },
        Z_VIGNETTE,
    );

    // Top left: the K mark and the name.
    canvas.draw_svg(
        "orus-k-login.svg",
        [l.mark.x, l.mark.y],
        [l.mark.w, l.mark.h],
        Z + 3,
    );
    text(
        canvas,
        l.wordmark_x,
        l.top_center_y,
        "Orus OS",
        WORDMARK_SIZE,
        500.0,
        TEXT,
    );

    // Top middle: the date and the time, one quiet line.
    centered_text(
        canvas,
        cx,
        l.top_center_y,
        &format!("{}   {}", scene.date, scene.clock),
        TOP_LINE_SIZE,
        400.0,
        DATE,
    );

    canvas.draw_svg(
        "avatar.svg",
        [l.avatar.x, l.avatar.y],
        [l.avatar.w, l.avatar.h],
        Z + 2,
    );

    if form.is_typing_username() {
        username_field(canvas, l.username_field, form);
    } else {
        centered_text(
            canvas,
            cx,
            l.name_center_y,
            form.display_name(),
            NAME_SIZE,
            600.0,
            TEXT,
        );
    }
    // Under the field: an error, or that a login is under way. Nothing otherwise.
    match &form.status {
        Status::Failed(message) => centered_text(
            canvas,
            cx,
            l.message_center_y,
            message,
            MESSAGE_SIZE,
            400.0,
            ERROR,
        ),
        _ if form.is_busy() => centered_text(
            canvas,
            cx,
            l.message_center_y,
            SIGNING_IN,
            MESSAGE_SIZE,
            600.0,
            TEXT,
        ),
        _ => {}
    }

    // Password.
    boxed(
        canvas,
        l.password_field,
        FIELD_RADIUS,
        FIELD_FILL,
        FIELD_BORDER,
    );
    password_content(canvas, l.password_field, form);
    line_icon(
        canvas,
        if form.show_password {
            "eye-off.svg"
        } else {
            "eye.svg"
        },
        l.eye.x,
        l.eye.y,
        EYE_SIZE,
        TEXT,
    );

    if form.can_switch_user() {
        centered_text(
            canvas,
            cx,
            l.switch_center_y,
            "Switch user",
            SMALL_TEXT,
            400.0,
            DIM,
        );
    }

    bottom_controls(canvas, &l, scene);
}

fn username_field(canvas: &mut Canvas, rect: Rect, form: &Form) {
    boxed(canvas, rect, FIELD_RADIUS, FIELD_FILL, FIELD_BORDER);
    let cy = rect.y + FORM_H * 0.5;
    let x = rect.x + FIELD_PAD;
    let width = if form.typed_username.is_empty() {
        text(canvas, x, cy, "Username", FIELD_TEXT, 400.0, PLACEHOLDER);
        0.0
    } else {
        text(canvas, x, cy, &form.typed_username, FIELD_TEXT, 400.0, TEXT);
        canvas.measure_text_width(&form.typed_username, FIELD_TEXT)
    };
    if form.focus == Field::Username && form.caret_visible() && !form.is_busy() {
        caret(canvas, x + width, rect.y);
    }
}

fn caret(canvas: &mut Canvas, x: f32, field_top: f32) {
    let inset = (FORM_H - FIELD_TEXT * 1.4) * 0.5;
    canvas.fill_rect(
        x + 1.0,
        field_top + inset,
        CARET_W,
        FORM_H - inset * 2.0,
        Brush::Solid(color(TEXT)),
        Z + 3,
    );
}

fn password_content(canvas: &mut Canvas, rect: Rect, form: &Form) {
    let cy = rect.y + FORM_H * 0.5;
    let x = rect.x + FIELD_PAD;
    let avail = rect.w - FIELD_PAD - FIELD_RIGHT_PAD;
    let used = if form.password.is_empty() {
        text(
            canvas,
            x,
            cy,
            "Password",
            FIELD_TEXT,
            400.0,
            PLACEHOLDER,
        );
        0.0
    } else if form.show_password {
        // Plain text, shifted left once it is wider than the field.
        let w = canvas.measure_text_width(&form.password, FIELD_TEXT);
        let shift = (w - avail).max(0.0);
        canvas.push_clip_rect(Rect {
            x,
            y: rect.y,
            w: avail,
            h: FORM_H,
        });
        text(
            canvas,
            x - shift,
            cy,
            &form.password,
            FIELD_TEXT,
            400.0,
            TEXT,
        );
        canvas.pop_clip();
        w.min(avail)
    } else {
        // Bullets: only the last ones that fit, so a long password stays in the box.
        let fit = ((avail / BULLET_PITCH) as usize).max(1);
        let shown = form.password.chars().count().min(fit);
        for i in 0..shown {
            canvas.rounded_rect(
                round(
                    Rect {
                        x: x + i as f32 * BULLET_PITCH,
                        y: cy - BULLET_D * 0.5,
                        w: BULLET_D,
                        h: BULLET_D,
                    },
                    BULLET_D * 0.5,
                ),
                Brush::Solid(color(TEXT)),
                Z + 3,
            );
        }
        shown as f32 * BULLET_PITCH
    };
    if form.focus == Field::Password && form.caret_visible() && !form.is_busy() {
        caret(canvas, x + used, rect.y);
    }
}

fn bottom_controls(canvas: &mut Canvas, l: &Layout, scene: &Scene) {
    // Keyboard layout, as information: nothing here changes it yet.
    if let Some(label) = scene.keyboard {
        let [x, y] = l.pill_origin;
        let text_w = canvas.measure_text_width(label, SMALL_TEXT);
        let pill = Rect {
            x,
            y,
            w: PILL_PAD + PILL_ICON + PILL_ICON_GAP + text_w + PILL_PAD,
            h: PILL_H,
        };
        boxed(canvas, pill, PILL_H * 0.5, PILL_FILL, PILL_BORDER);
        line_icon(
            canvas,
            "keyboard.svg",
            pill.x + PILL_PAD,
            y + (PILL_H - PILL_ICON) * 0.5,
            PILL_ICON,
            TEXT,
        );
        text(
            canvas,
            pill.x + PILL_PAD + PILL_ICON + PILL_ICON_GAP,
            y + PILL_H * 0.5,
            label,
            SMALL_TEXT,
            400.0,
            TEXT,
        );
    }

    for (rect, icon) in [
        (
            l.wifi,
            match scene.network {
                Network::Wifi => "status-wifi.svg",
                Network::Wired => "status-ethernet.svg",
                Network::Offline => "status-wifi-off.svg",
            },
        ),
        (l.power, "power.svg"),
    ] {
        boxed(canvas, rect, CIRCLE_D * 0.5, CIRCLE_FILL, CIRCLE_BORDER);
        line_icon(
            canvas,
            icon,
            rect.x + (rect.w - CIRCLE_ICON) * 0.5,
            rect.y + (rect.h - CIRCLE_ICON) * 0.5,
            CIRCLE_ICON,
            TEXT,
        );
    }

    if scene.power_menu_open {
        boxed(canvas, l.menu, FIELD_RADIUS, PILL_FILL, PILL_BORDER);
        for (i, label) in ["Shut down", "Restart"].iter().enumerate() {
            text(
                canvas,
                l.menu.x + FIELD_PAD * 0.75,
                l.menu.y + (i as f32 + 0.5) * MENU_ROW_H,
                label,
                SMALL_TEXT,
                400.0,
                TEXT,
            );
        }
    }
}
