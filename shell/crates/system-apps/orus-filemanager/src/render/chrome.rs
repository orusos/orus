//! The frame around the folder's contents: tabs, sidebar, path bar, status bar.

use jag_draw::{Rect, SvgStyle};
use orus_core::palette::Palette;

use super::{bordered, col, fill, fit, text};
use crate::layout::{self, SIDEBAR_W, STATUS_H, TAB_ROW_H};
use crate::model::{self, ViewMode};
use crate::places::Place;
use crate::{FileManagerApp, fsinfo, icons};

pub(super) const ICON: f32 = 22.0;
const PLACE_TEXT: f32 = 15.0;
const PATH_TEXT: f32 = 15.0;
const STATUS_TEXT: f32 = 14.0;
const PLACE_RADIUS: f32 = 10.0;
const TOGGLE_RADIUS: f32 = 10.0;
const CURSOR_BLINK_MS: u128 = 530;
/// Gap either side of the "/" between path segments.
const CRUMB_GAP: f32 = 16.0;

pub(super) fn stroked(hex: u32) -> SvgStyle {
    SvgStyle::new().with_stroke(col(hex))
}

pub(super) fn centered_icon(
    canvas: &mut super::Canvas,
    name: &str,
    center: (f32, f32),
    size: f32,
    style: SvgStyle,
    z: i32,
) {
    canvas.draw_svg_styled(
        name,
        [center.0 - size * 0.5, center.1 - size * 0.5],
        [size, size],
        style,
        z,
    );
}

impl FileManagerApp {
    fn place_row(
        &self,
        canvas: &mut super::Canvas,
        pal: &Palette,
        place: &Place,
        center_y: f32,
        z: i32,
    ) {
        let pill = layout::place_pill(center_y);
        let current = self.tab().dir == place.path;
        if current {
            fill(canvas, pill, PLACE_RADIUS, pal.selection, z);
        }
        let icon_color = if current { pal.accent } else { pal.text_dim };
        centered_icon(
            canvas,
            icons::place_icon(place.kind),
            (37.0, center_y),
            ICON,
            stroked(icon_color),
            z + 1,
        );
        text(
            canvas,
            (63.0, center_y),
            &place.label,
            PLACE_TEXT,
            400.0,
            pal.text,
            z + 1,
        );
    }

    pub(super) fn render_sidebar(&self, canvas: &mut super::Canvas, pal: &Palette, h: f32, z: i32) {
        let top = 0.0;
        fill(
            canvas,
            Rect {
                x: 0.0,
                y: top,
                w: SIDEBAR_W,
                h: h - top - STATUS_H,
            },
            0.0,
            pal.sidebar,
            z,
        );
        fill(
            canvas,
            Rect {
                x: SIDEBAR_W - 1.0,
                y: top,
                w: 1.0,
                h: h - top - STATUS_H,
            },
            0.0,
            pal.border,
            z + 1,
        );

        let side = layout::sidebar(self.places.favorites.len(), self.places.devices.len());
        for (place, cy) in self.places.favorites.iter().zip(&side.favorite_centers) {
            self.place_row(canvas, pal, place, *cy, z + 2);
        }
        fill(
            canvas,
            Rect {
                x: 25.0,
                y: side.separator_y,
                w: 154.0,
                h: 1.0,
            },
            0.0,
            pal.border,
            z + 2,
        );
        text(
            canvas,
            (25.0, side.devices_label_y),
            "Devices",
            PLACE_TEXT - 1.0,
            400.0,
            pal.text_dim,
            z + 2,
        );
        for (place, cy) in self.places.devices.iter().zip(&side.device_centers) {
            self.place_row(canvas, pal, place, *cy, z + 2);
        }
    }

    pub(super) fn render_toolbar(&self, canvas: &mut super::Canvas, pal: &Palette, w: f32, z: i32) {
        // Covers the items scrolled up under the toolbar.
        fill(
            canvas,
            Rect {
                x: SIDEBAR_W,
                y: TAB_ROW_H,
                w: w - SIDEBAR_W,
                h: layout::SCROLL_TOP - TAB_ROW_H,
            },
            0.0,
            pal.surface,
            z,
        );
        let bar = layout::toolbar(w);
        let cy = layout::TOOLBAR_CENTER_Y;
        let back_color = if self.tab().can_go_back() {
            pal.text
        } else {
            pal.text_dim
        };
        let forward_color = if self.tab().can_go_forward() {
            pal.text
        } else {
            pal.text_dim
        };
        centered_icon(
            canvas,
            icons::BACK,
            (layout::BACK_CENTER_X, cy),
            ICON,
            stroked(back_color),
            z + 1,
        );
        centered_icon(
            canvas,
            icons::FORWARD,
            (layout::FORWARD_CENTER_X, cy),
            ICON,
            stroked(forward_color),
            z + 1,
        );

        fill(canvas, bar.pill, layout::PILL_RADIUS, pal.field, z + 1);
        let search = layout::search_rect(bar.pill);
        centered_icon(
            canvas,
            icons::SEARCH,
            (search.x + search.w * 0.5 - 2.0, cy),
            20.0,
            stroked(if self.search.is_some() {
                pal.accent
            } else {
                pal.text
            }),
            z + 2,
        );
        self.render_path_text(canvas, pal, bar.pill, cy, z + 2);

        for (rect, view, icon) in [
            (bar.grid_btn, ViewMode::Grid, icons::GRID),
            (bar.list_btn, ViewMode::List, icons::LIST),
        ] {
            let selected = self.view == view;
            if selected {
                bordered(
                    canvas,
                    rect,
                    TOGGLE_RADIUS,
                    pal.selection,
                    pal.accent,
                    z + 1,
                );
            } else {
                fill(canvas, rect, TOGGLE_RADIUS, pal.field, z + 1);
            }
            let color = if selected { pal.accent } else { pal.text_dim };
            let style = if view == ViewMode::Grid {
                SvgStyle::new().with_fill(col(color))
            } else {
                stroked(color)
            };
            centered_icon(
                canvas,
                icon,
                (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5),
                20.0,
                style,
                z + 2,
            );
        }
    }

    /// The path in the pill, or the search text while searching. Also records where
    /// each segment is, for clicks.
    fn render_path_text(
        &self,
        canvas: &mut super::Canvas,
        pal: &Palette,
        pill: Rect,
        cy: f32,
        z: i32,
    ) {
        let mut hits = self.crumb_hits.borrow_mut();
        hits.clear();
        let text_x = pill.x + 50.0;
        if let Some(query) = &self.search {
            centered_icon(
                canvas,
                icons::SEARCH,
                (pill.x + 25.0, cy),
                ICON,
                stroked(pal.text_dim),
                z,
            );
            if query.is_empty() {
                text(
                    canvas,
                    (text_x, cy),
                    "Search",
                    PATH_TEXT,
                    400.0,
                    pal.text_dim,
                    z,
                );
            } else {
                text(canvas, (text_x, cy), query, PATH_TEXT, 400.0, pal.text, z);
            }
            let on =
                self.cursor_epoch.elapsed().as_millis() % (CURSOR_BLINK_MS * 2) < CURSOR_BLINK_MS;
            if on {
                let tw = if query.is_empty() {
                    0.0
                } else {
                    canvas.measure_text_width(query, PATH_TEXT)
                };
                fill(
                    canvas,
                    Rect {
                        x: text_x + tw + 1.0,
                        y: cy - 9.0,
                        w: 1.5,
                        h: 18.0,
                    },
                    0.0,
                    pal.text,
                    z,
                );
            }
            return;
        }
        centered_icon(
            canvas,
            icons::HOME,
            (pill.x + 25.0, cy),
            ICON,
            stroked(pal.text),
            z,
        );
        let crumbs = model::breadcrumbs(&self.home, &self.tab().dir);
        let right_limit = pill.x + pill.w - 60.0;
        let mut x = text_x;
        for (i, (label, path)) in crumbs.iter().enumerate() {
            let label = fit(canvas, label, PATH_TEXT, (right_limit - x).max(0.0));
            let tw = canvas.measure_text_width(&label, PATH_TEXT);
            text(canvas, (x, cy), &label, PATH_TEXT, 400.0, pal.text, z);
            hits.push((
                Rect {
                    x,
                    y: pill.y,
                    w: tw,
                    h: pill.h,
                },
                path.clone(),
            ));
            x += tw;
            if i + 1 < crumbs.len() {
                x += CRUMB_GAP;
                text(canvas, (x, cy), "/", PATH_TEXT, 400.0, pal.text_dim, z);
                x += canvas.measure_text_width("/", PATH_TEXT) + CRUMB_GAP;
            }
            if x >= right_limit {
                break;
            }
        }
    }

    pub(super) fn render_status(
        &self,
        canvas: &mut super::Canvas,
        pal: &Palette,
        w: f32,
        h: f32,
        z: i32,
    ) {
        let top = h - STATUS_H;
        fill(
            canvas,
            Rect {
                x: 0.0,
                y: top,
                w,
                h: STATUS_H,
            },
            0.0,
            pal.sidebar,
            z,
        );
        fill(
            canvas,
            Rect {
                x: 0.0,
                y: top,
                w,
                h: 1.0,
            },
            0.0,
            pal.border,
            z + 1,
        );
        let cy = top + STATUS_H * 0.5 + 1.0;
        let count = self.visible.len();
        let items = format!("{count} {}", if count == 1 { "item" } else { "items" });
        text(
            canvas,
            (layout::STATUS_PAD, cy),
            &items,
            STATUS_TEXT,
            400.0,
            pal.text_dim,
            z + 2,
        );
        if let Some(bytes) = self.available {
            let label = fsinfo::format_available(bytes);
            let tw = canvas.measure_text_width(&label, STATUS_TEXT);
            text(
                canvas,
                (w - layout::STATUS_PAD - tw, cy),
                &label,
                STATUS_TEXT,
                400.0,
                pal.text_dim,
                z + 2,
            );
        }
    }
}
