//! The tab row.

use jag_draw::{Brush, Rect, RoundedRadii, RoundedRect, SvgStyle};
use orus_core::palette::Palette;

use super::chrome::{ICON, centered_icon, stroked};
use super::{col, fill, fit, text};
use crate::layout::{self, TAB_ROW_H};
use crate::{FileManagerApp, icons};

const TAB_TEXT: f32 = 14.0;
const TAB_RADIUS: f32 = 10.0;

fn top_rounded(canvas: &mut super::Canvas, rect: Rect, radius: f32, hex: u32, z: i32) {
    canvas.rounded_rect(
        RoundedRect {
            rect,
            radii: RoundedRadii {
                tl: radius,
                tr: radius,
                br: 0.0,
                bl: 0.0,
            },
        },
        Brush::Solid(col(hex)),
        z,
    );
}

impl FileManagerApp {
    pub(super) fn render_tabs(&self, canvas: &mut super::Canvas, pal: &Palette, w: f32, z: i32) {
        fill(
            canvas,
            Rect {
                x: layout::SIDEBAR_W,
                y: 0.0,
                w: w - layout::SIDEBAR_W,
                h: TAB_ROW_H,
            },
            0.0,
            pal.sidebar,
            z,
        );
        fill(
            canvas,
            Rect {
                x: layout::SIDEBAR_W,
                y: TAB_ROW_H - 1.0,
                w: w - layout::SIDEBAR_W,
                h: 1.0,
            },
            0.0,
            pal.border,
            z + 1,
        );

        for (i, tab) in self.tabs.iter().enumerate() {
            let rect = layout::tab_rect(i, self.active);
            if i == self.active {
                top_rounded(canvas, rect, TAB_RADIUS, pal.tab_active, z + 2);
            } else {
                top_rounded(canvas, rect, TAB_RADIUS, pal.border, z + 2);
                let inner = Rect {
                    x: rect.x + 1.0,
                    y: rect.y + 1.0,
                    w: rect.w - 2.0,
                    h: rect.h - 1.0,
                };
                top_rounded(canvas, inner, TAB_RADIUS - 1.0, pal.tab_inactive, z + 3);
            }
            let cy = rect.y + (rect.h - 1.0) * 0.5;
            let at_home = tab.dir == self.home;
            let (icon, style) = if at_home {
                (icons::HOME, stroked(pal.text))
            } else {
                (icons::TAB_FOLDER, SvgStyle::new())
            };
            centered_icon(canvas, icon, (rect.x + 26.0, cy), ICON, style, z + 4);
            let close = layout::tab_close_rect(rect);
            let title = fit(
                canvas,
                &tab.title(&self.home),
                TAB_TEXT,
                close.x - rect.x - 56.0,
            );
            text(
                canvas,
                (rect.x + 50.0, cy),
                &title,
                TAB_TEXT,
                400.0,
                pal.text,
                z + 4,
            );
            centered_icon(
                canvas,
                icons::CLOSE,
                (close.x + close.w * 0.5, close.y + close.h * 0.5),
                16.0,
                stroked(pal.text),
                z + 4,
            );
        }
        let plus = layout::new_tab_rect(self.tabs.len(), self.active);
        centered_icon(
            canvas,
            icons::PLUS,
            (plus.x + plus.w * 0.5, plus.y + plus.h * 0.5),
            20.0,
            stroked(pal.text),
            z + 2,
        );
    }
}
