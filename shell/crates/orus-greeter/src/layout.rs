//! Where each part of the login screen sits. The login is a short, spaced-out stack in the
//! middle of the screen: the account's picture and name, a small pill-shaped password field,
//! and "Switch user". The clock is one quiet line at the top and the controls are in the
//! corners. Sizes are written for the 1672-wide mock-up; the drawing layer draws them
//! at 80% (`orus_core::design::SCALE`); the stack is centered on the screen, and the bottom
//! controls follow the bottom edge.

use jag_draw::Rect;

/// Width and height of the password field (and the account name field). The field is a pill.
pub const FORM_W: f32 = 300.0;
pub const FORM_H: f32 = 48.0;
pub const AVATAR_D: f32 = 120.0;
pub const CIRCLE_D: f32 = 50.0;
pub const PILL_H: f32 = 44.0;
pub const EYE_SIZE: f32 = 24.0;
/// Margin from the screen edges to the corner elements.
pub const EDGE_MARGIN: f32 = 40.0;
/// Centre of the eye icon, from the form's right edge.
const EYE_INSET: f32 = 30.0;
/// Gap between the two circles at the bottom right.
const CIRCLE_GAP: f32 = 16.0;
const BOTTOM_ROW_CENTER_FROM_BOTTOM: f32 = 60.0;

/// Power menu rows.
pub const MENU_W: f32 = 170.0;
pub const MENU_ROW_H: f32 = 44.0;
const MENU_GAP: f32 = 10.0;

/// Where the middle of the stack sits, as a share of the screen's height: a little above the
/// centre, as login screens usually do.
const STACK_CENTER: f32 = 0.45;
/// From that middle up to the top of the picture.
const AVATAR_ABOVE_CENTER: f32 = 150.0;
/// Bottom of the picture to the middle of the name.
const NAME_BELOW_AVATAR: f32 = 40.0;
/// Middle of the name to the top of the password field. It also leaves room for the account
/// name field, which stands where the name does, to sit above the password field.
const FIELD_BELOW_NAME: f32 = 40.0;
/// Bottom of the field to the middle of an error or "Signing in…".
const MESSAGE_BELOW_FIELD: f32 = 30.0;
/// Bottom of the field to the middle of "Switch user".
const SWITCH_BELOW_FIELD: f32 = 72.0;
/// Size of the "Switch user" click target around its text.
const SWITCH_HIT_W: f32 = 150.0;
const SWITCH_HIT_H: f32 = 40.0;
/// The pill row's top, from the bottom of the screen.
const PILL_TOP_FROM_BOTTOM: f32 = 80.0;

// The mark and the name at the top left, and the clock line across from them.
const MARK_X: f32 = 46.0;
const MARK_Y: f32 = 33.0;
const MARK_W: f32 = 26.0;
const MARK_H: f32 = 28.0;
const WORDMARK_X: f32 = 90.0;
const TOP_ROW_CENTER_Y: f32 = 47.0;

pub struct Layout {
    pub mark: Rect,
    pub wordmark_x: f32,
    pub top_center_y: f32,
    pub avatar: Rect,
    pub name_center_y: f32,
    /// Where an error or "Signing in…" goes, under the password field.
    pub message_center_y: f32,
    /// The typed account name's field, in the place of the name.
    pub username_field: Rect,
    pub password_field: Rect,
    pub eye: Rect,
    pub switch_center_y: f32,
    pub switch_hit: Rect,
    /// Top-left of the keyboard pill; its width depends on its text.
    pub pill_origin: [f32; 2],
    pub wifi: Rect,
    pub power: Rect,
    pub menu: Rect,
}

impl Layout {
    pub fn new(viewport: Rect) -> Self {
        let cx = viewport.x + viewport.w * 0.5;
        let left = cx - FORM_W * 0.5;
        let field = |y: f32| Rect {
            x: left,
            y,
            w: FORM_W,
            h: FORM_H,
        };
        let middle = viewport.y + viewport.h * STACK_CENTER;
        let avatar_top = (middle - AVATAR_ABOVE_CENTER).round();
        let name_center_y = (avatar_top + AVATAR_D + NAME_BELOW_AVATAR).round();
        let password_field = field((name_center_y + FIELD_BELOW_NAME).round());
        let field_bottom = password_field.y + FORM_H;
        let switch_center_y = (field_bottom + SWITCH_BELOW_FIELD).round();

        let bottom_center_y = viewport.y + viewport.h - BOTTOM_ROW_CENTER_FROM_BOTTOM;
        let circle = |center_x: f32| Rect {
            x: center_x - CIRCLE_D * 0.5,
            y: bottom_center_y - CIRCLE_D * 0.5,
            w: CIRCLE_D,
            h: CIRCLE_D,
        };
        let right = viewport.x + viewport.w - EDGE_MARGIN;
        let power = circle(right - CIRCLE_D * 0.5);
        let wifi = circle(power.x - CIRCLE_GAP - CIRCLE_D * 0.5);
        let menu_h = MENU_ROW_H * 2.0;
        Self {
            mark: Rect {
                x: MARK_X,
                y: MARK_Y,
                w: MARK_W,
                h: MARK_H,
            },
            wordmark_x: WORDMARK_X,
            top_center_y: TOP_ROW_CENTER_Y,
            avatar: Rect {
                x: cx - AVATAR_D * 0.5,
                y: avatar_top,
                w: AVATAR_D,
                h: AVATAR_D,
            },
            name_center_y,
            message_center_y: (field_bottom + MESSAGE_BELOW_FIELD).round(),
            // The name field is as tall as the name's line: centered on it.
            username_field: field((name_center_y - FORM_H * 0.5).round()),
            eye: Rect {
                x: password_field.x + password_field.w - EYE_INSET - EYE_SIZE * 0.5,
                y: password_field.y + (FORM_H - EYE_SIZE) * 0.5,
                w: EYE_SIZE,
                h: EYE_SIZE,
            },
            password_field,
            switch_center_y,
            switch_hit: Rect {
                x: cx - SWITCH_HIT_W * 0.5,
                y: switch_center_y - SWITCH_HIT_H * 0.5,
                w: SWITCH_HIT_W,
                h: SWITCH_HIT_H,
            },
            pill_origin: [
                viewport.x + EDGE_MARGIN,
                viewport.y + viewport.h - PILL_TOP_FROM_BOTTOM,
            ],
            wifi,
            power,
            menu: Rect {
                x: power.x + power.w - MENU_W,
                y: power.y - MENU_GAP - menu_h,
                w: MENU_W,
                h: menu_h,
            },
        }
    }
}

pub fn contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x < rect.x + rect.w && y >= rect.y && y < rect.y + rect.h
}
