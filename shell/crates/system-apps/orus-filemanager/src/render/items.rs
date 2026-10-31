//! The folder's contents: a heading, then a grid of icons or a list of rows.

use jag_draw::Rect;
use jag_surface::ImageFitMode;
use orus_core::appearance::Theme;
use orus_core::palette::Palette;

use super::{bordered, fill, fit, rrect, text, text_centered};
use crate::layout::{self, CellKind, SCROLL_TOP, STATUS_H};
use crate::model::{DirEntry, ViewMode};
use crate::{EditMode, FileManagerApp, icons, listing};

const LABEL_TEXT: f32 = 15.0;
const ROW_TEXT: f32 = 15.0;
const ROW_META_TEXT: f32 = 14.0;
const TILE_BORDER_RADIUS: f32 = layout::TILE_RADIUS;
const LIST_RADIUS: f32 = 10.0;
/// Room each side of a label inside its cell.
const LABEL_MARGIN: f32 = 8.0;
const FIELD_H: f32 = 28.0;
const EMPTY_TEXT_TOP: f32 = 250.0;
/// Columns of the list view, from the row's right edge.
const DATE_RIGHT_PAD: f32 = 16.0;
const SIZE_RIGHT: f32 = 150.0;
/// The list's icons, scaled from the grid's.
const LIST_FOLDER: (f32, f32) = (35.0, 30.0);
const LIST_FILE: (f32, f32) = (22.0, 28.0);

fn rect_of(x: f32, y: f32, size: (f32, f32)) -> Rect {
    Rect {
        x,
        y,
        w: size.0,
        h: size.1,
    }
}

fn icon_name(entry: &DirEntry, theme: Theme) -> &'static str {
    if entry.is_dir {
        icons::folder_icon(&entry.name, theme)
    } else {
        icons::file_icon(theme)
    }
}

impl FileManagerApp {
    pub(super) fn render_items(&self, canvas: &mut super::Canvas, w: f32, h: f32, z: i32) {
        let pal = orus_core::palette::get();
        canvas.push_clip_rect(Rect {
            x: layout::SIDEBAR_W,
            y: SCROLL_TOP,
            w: w - layout::SIDEBAR_W,
            h: h - STATUS_H - SCROLL_TOP,
        });
        let scroll = self.tab().scroll_y;
        text(
            canvas,
            (layout::HEADING_X, layout::HEADING_CENTER_Y - scroll),
            &self.tab().title(&self.home),
            layout::HEADING_SIZE,
            600.0,
            pal.text,
            z,
        );
        if self.visible.is_empty() && self.search.is_some() {
            let label = "No matches";
            text_centered(
                canvas,
                (
                    layout::SIDEBAR_W + (w - layout::SIDEBAR_W) * 0.5,
                    EMPTY_TEXT_TOP,
                ),
                label,
                LABEL_TEXT,
                400.0,
                pal.text_dim,
                z,
            );
        } else if self.entries.is_empty() && self.new_entry_buffer().is_none() {
            let label = "Empty folder";
            text_centered(
                canvas,
                (
                    layout::SIDEBAR_W + (w - layout::SIDEBAR_W) * 0.5,
                    EMPTY_TEXT_TOP,
                ),
                label,
                LABEL_TEXT,
                400.0,
                pal.text_dim,
                z,
            );
        }
        for (position, &index) in self.visible.iter().enumerate() {
            let rect = self.item_rect(position);
            if rect.y + rect.h < SCROLL_TOP || rect.y > h - STATUS_H {
                continue;
            }
            let selected = self.tab().selected == Some(index);
            let renaming = match &self.edit_mode {
                EditMode::Renaming { index: i, buffer } if *i == index => Some(buffer.as_str()),
                _ => None,
            };
            let entry = &self.entries[index];
            match self.view {
                ViewMode::Grid => {
                    self.grid_cell(canvas, &pal, entry, rect, selected, renaming, z + 1)
                }
                ViewMode::List => {
                    self.list_row(canvas, &pal, entry, rect, selected, renaming, z + 1)
                }
            }
        }
        if let Some(buffer) = self.new_entry_buffer() {
            let is_folder = matches!(self.edit_mode, EditMode::CreatingFolder { .. });
            let entry = DirEntry {
                name: String::new(),
                is_dir: is_folder,
                size: 0,
                modified: None,
            };
            let rect = self.item_rect(self.visible.len());
            match self.view {
                ViewMode::Grid => {
                    self.grid_cell(canvas, &pal, &entry, rect, false, Some(buffer), z + 1)
                }
                ViewMode::List => {
                    self.list_row(canvas, &pal, &entry, rect, false, Some(buffer), z + 1)
                }
            }
        }
        canvas.pop_clip();
    }

    /// The text typed so far for a file or folder being created.
    fn new_entry_buffer(&self) -> Option<&str> {
        match &self.edit_mode {
            EditMode::CreatingFile { buffer } | EditMode::CreatingFolder { buffer } => Some(buffer),
            _ => None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn grid_cell(
        &self,
        canvas: &mut super::Canvas,
        pal: &Palette,
        entry: &DirEntry,
        cell: Rect,
        selected: bool,
        editing: Option<&str>,
        z: i32,
    ) {
        let kind = icons::cell_kind(entry);
        let center_x = cell.x + cell.w * 0.5;
        if selected {
            let tile = layout::tile_rect(cell, kind);
            bordered(
                canvas,
                tile,
                TILE_BORDER_RADIUS,
                pal.selection,
                pal.accent,
                z,
            );
        }
        let (iw, ih) = kind.icon_size();
        let icon = rect_of(
            center_x - iw * 0.5,
            cell.y + layout::CELL_ICON_TOP,
            (iw, ih),
        );
        if kind == CellKind::Thumbnail {
            let path = self.tab().dir.join(&entry.name);
            canvas.push_clip_rounded_rect(rrect(icon, layout::THUMB_RADIUS));
            canvas.draw_image(
                path,
                [icon.x, icon.y],
                [icon.w, icon.h],
                ImageFitMode::Cover,
                z + 1,
            );
            canvas.pop_clip();
        } else {
            canvas.draw_svg(
                icon_name(entry, pal.theme),
                [icon.x, icon.y],
                [icon.w, icon.h],
                z + 1,
            );
        }
        let label_y = cell.y + kind.label_center();
        match editing {
            Some(buffer) => {
                let field = Rect {
                    x: center_x - layout::TILE_W * 0.5,
                    y: label_y - FIELD_H * 0.5,
                    w: layout::TILE_W,
                    h: FIELD_H,
                };
                self.edit_field(canvas, pal, field, buffer, z + 2);
            }
            None => {
                let label = fit(
                    canvas,
                    &entry.name,
                    LABEL_TEXT,
                    layout::TILE_W - LABEL_MARGIN * 2.0,
                );
                text_centered(
                    canvas,
                    (center_x, label_y),
                    &label,
                    LABEL_TEXT,
                    400.0,
                    pal.text,
                    z + 2,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn list_row(
        &self,
        canvas: &mut super::Canvas,
        pal: &Palette,
        entry: &DirEntry,
        row: Rect,
        selected: bool,
        editing: Option<&str>,
        z: i32,
    ) {
        if selected {
            fill(canvas, row, LIST_RADIUS, pal.selection, z);
        }
        let cy = row.y + row.h * 0.5;
        let size = if entry.is_dir { LIST_FOLDER } else { LIST_FILE };
        let icon = rect_of(
            row.x + 16.0 + (LIST_FOLDER.0 - size.0) * 0.5,
            cy - size.1 * 0.5,
            size,
        );
        canvas.draw_svg(
            icon_name(entry, pal.theme),
            [icon.x, icon.y],
            [icon.w, icon.h],
            z + 1,
        );
        let name_x = row.x + 64.0;
        match editing {
            Some(buffer) => {
                let field = Rect {
                    x: name_x - 8.0,
                    y: cy - FIELD_H * 0.5,
                    w: 280.0,
                    h: FIELD_H,
                };
                self.edit_field(canvas, pal, field, buffer, z + 2);
            }
            None => {
                text(
                    canvas,
                    (name_x, cy),
                    &entry.name,
                    ROW_TEXT,
                    400.0,
                    pal.text,
                    z + 2,
                );
                let right = row.x + row.w - DATE_RIGHT_PAD;
                if let Some(modified) = entry.modified {
                    let date = modified.format("%b %d, %Y").to_string();
                    let dw = canvas.measure_text_width(&date, ROW_META_TEXT);
                    text(
                        canvas,
                        (right - dw, cy),
                        &date,
                        ROW_META_TEXT,
                        400.0,
                        pal.text_dim,
                        z + 2,
                    );
                }
                if !entry.is_dir && entry.size > 0 {
                    let size = listing::format_size(entry.size);
                    let sw = canvas.measure_text_width(&size, ROW_META_TEXT);
                    text(
                        canvas,
                        (row.x + row.w - SIZE_RIGHT - sw, cy),
                        &size,
                        ROW_META_TEXT,
                        400.0,
                        pal.text_dim,
                        z + 2,
                    );
                }
            }
        }
    }
}
