//! Things drawn over the rest: the name field, the right-click menu, the delete prompt.

use jag_draw::{Brush, ColorLinPremul, Rect};
use orus_core::palette::Palette;

use super::{bordered, fill, text};
use crate::input::{MENU_ITEM_H, MENU_PAD};
use crate::{ContextAction, EditMode, FileManagerApp};

const CURSOR_BLINK_MS: u128 = 530;
const FIELD_RADIUS: f32 = 8.0;
const FIELD_TEXT: f32 = 14.0;
const MENU_RADIUS: f32 = 12.0;
const MENU_TEXT: f32 = 14.0;
const PROMPT_W: f32 = 340.0;
const PROMPT_H: f32 = 96.0;
const PROMPT_TOP: f32 = 140.0;
/// Delete's color. The concepts show no delete control, so this is not sampled
/// from them; it is a plain red that reads on both themes.
const DESTRUCTIVE: u32 = 0xe5484d;
/// Dimming behind the delete prompt.
const SCRIM_ALPHA: f32 = 0.45;

impl FileManagerApp {
    /// A box with `buffer` typed in it and a blinking cursor.
    pub(super) fn edit_field(
        &self,
        canvas: &mut super::Canvas,
        pal: &Palette,
        rect: Rect,
        buffer: &str,
        z: i32,
    ) {
        bordered(canvas, rect, FIELD_RADIUS, pal.field, pal.accent, z);
        let cy = rect.y + rect.h * 0.5;
        if buffer.is_empty() {
            text(
                canvas,
                (rect.x + 10.0, cy),
                "Name",
                FIELD_TEXT,
                400.0,
                pal.text_dim,
                z + 2,
            );
        } else {
            text(
                canvas,
                (rect.x + 10.0, cy),
                buffer,
                FIELD_TEXT,
                400.0,
                pal.text,
                z + 2,
            );
        }
        let on = self.cursor_epoch.elapsed().as_millis() % (CURSOR_BLINK_MS * 2) < CURSOR_BLINK_MS;
        if on {
            let tw = if buffer.is_empty() {
                0.0
            } else {
                canvas.measure_text_width(buffer, FIELD_TEXT)
            };
            fill(
                canvas,
                Rect {
                    x: rect.x + 11.0 + tw,
                    y: cy - 8.0,
                    w: 1.5,
                    h: 16.0,
                },
                0.0,
                pal.text,
                z + 2,
            );
        }
    }

    pub(super) fn render_overlays(
        &self,
        canvas: &mut super::Canvas,
        pal: &Palette,
        w: f32,
        h: f32,
        z: i32,
    ) {
        if let EditMode::ConfirmDelete { index } = &self.edit_mode
            && let Some(entry) = self.entries.get(*index)
        {
            canvas.fill_rect(
                0.0,
                0.0,
                w,
                h,
                Brush::Solid(ColorLinPremul {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: SCRIM_ALPHA,
                }),
                z,
            );
            let panel = Rect {
                x: (w - PROMPT_W) * 0.5,
                y: PROMPT_TOP,
                w: PROMPT_W,
                h: PROMPT_H,
            };
            bordered(canvas, panel, MENU_RADIUS, pal.sidebar, pal.border, z + 1);
            let message = format!("Delete \u{201c}{}\u{201d}?", entry.name);
            text(
                canvas,
                (panel.x + 24.0, panel.y + 36.0),
                &message,
                15.0,
                600.0,
                DESTRUCTIVE,
                z + 3,
            );
            text(
                canvas,
                (panel.x + 24.0, panel.y + 66.0),
                "Enter to delete, Esc to cancel",
                13.0,
                400.0,
                pal.text_dim,
                z + 3,
            );
        }
        if let Some(menu) = &self.context_menu {
            let rect = Self::menu_rect(menu);
            bordered(canvas, rect, MENU_RADIUS, pal.sidebar, pal.border, z + 10);
            for (i, (label, action)) in Self::menu_items(menu).iter().enumerate() {
                let cy = rect.y + MENU_PAD + i as f32 * MENU_ITEM_H + MENU_ITEM_H * 0.5;
                let color = if *action == ContextAction::Delete {
                    DESTRUCTIVE
                } else {
                    pal.text
                };
                text(
                    canvas,
                    (rect.x + 16.0, cy),
                    label,
                    MENU_TEXT,
                    400.0,
                    color,
                    z + 12,
                );
            }
        }
    }
}
