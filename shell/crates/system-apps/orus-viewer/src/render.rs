//! Rendering for the viewer — toolbar, gallery grid, and preview mode.

use crate::{EntryKind, RenderState, ViewMode, ViewerApp};
use jag_draw::{
    Brush, ColorLinPremul, FontStyle, Rect, RoundedRadii, RoundedRect, SvgStyle, Transform2D,
};
use jag_surface::ImageFitMode;

// ── Layout constants ────────────────────────────────────────────

pub const TOOLBAR_HEIGHT: f32 = 44.0;
pub const CELL_SIZE: f32 = 140.0;
pub const THUMB_SIZE: f32 = 120.0;
pub const THUMB_PADDING: f32 = (CELL_SIZE - THUMB_SIZE) / 2.0;
pub const CHANGE_FOLDER_BTN_W: f32 = 100.0;
pub const BACK_BTN_HIT_W: f32 = 100.0;
pub const LABEL_FONT_SIZE: f32 = 11.0;
pub const LABEL_HEIGHT: f32 = 16.0;
pub const THUMB_CORNER: f32 = 6.0;
/// Space between the preview image and the window edges.
pub const PREVIEW_PAD: f32 = 20.0;

pub const PICKER_ROW_HEIGHT: f32 = 36.0;
pub const PICKER_OPEN_BTN_W: f32 = 90.0;

pub const TOOLBAR_FONT_SIZE: f32 = 13.0;
const PICKER_FONT_SIZE: f32 = 13.0;
const ARROW_FONT_SIZE: f32 = 28.0;
const BTN_HEIGHT: f32 = 28.0;
const BTN_CORNER: f32 = 6.0;
const SELECT_CORNER: f32 = 8.0;
const MAX_LABEL_CHARS: usize = 15;
pub const BREADCRUMB_PAD: f32 = 12.0;
const PICKER_ICON_SIZE: f32 = 18.0;
const PICKER_TEXT_PAD: f32 = 40.0;

const FOLDER_ICON_COLOR: ColorLinPremul = ColorLinPremul {
    r: 0.0600,
    g: 0.2800,
    b: 0.9200,
    a: 1.0,
};

const IMAGE_ICON_COLOR: ColorLinPremul = ColorLinPremul {
    r: 0.2000,
    g: 0.2000,
    b: 0.2200,
    a: 1.0,
};

// ── Colors (macOS dark theme, linear premul) ────────────────────

const BG: ColorLinPremul = ColorLinPremul {
    r: 0.0110,
    g: 0.0110,
    b: 0.0130,
    a: 1.0,
};

const TOOLBAR_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0150,
    g: 0.0150,
    b: 0.0170,
    a: 1.0,
};

const SEPARATOR: ColorLinPremul = ColorLinPremul {
    r: 0.0200,
    g: 0.0200,
    b: 0.0230,
    a: 1.0,
};

const TEXT_PRIMARY: ColorLinPremul = ColorLinPremul {
    r: 0.8500,
    g: 0.8500,
    b: 0.8700,
    a: 1.0,
};

const TEXT_SECONDARY: ColorLinPremul = ColorLinPremul {
    r: 0.2000,
    g: 0.2000,
    b: 0.2200,
    a: 1.0,
};

const SELECTED_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0180,
    g: 0.0450,
    b: 0.1700,
    a: 1.0,
};

const THUMB_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0170,
    g: 0.0170,
    b: 0.0190,
    a: 1.0,
};

const BTN_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0200,
    g: 0.0200,
    b: 0.0230,
    a: 1.0,
};

const PREVIEW_BG: ColorLinPremul = ColorLinPremul {
    r: 0.0050,
    g: 0.0050,
    b: 0.0060,
    a: 1.0,
};

// ── Helpers ─────────────────────────────────────────────────────

fn rounded_radii(r: f32) -> RoundedRadii {
    RoundedRadii {
        tl: r,
        tr: r,
        br: r,
        bl: r,
    }
}

/// Truncate a filename to `MAX_LABEL_CHARS` plus ellipsis if needed.
fn truncate_label(name: &str) -> String {
    if name.len() <= MAX_LABEL_CHARS {
        name.to_string()
    } else {
        let mut s = name[..MAX_LABEL_CHARS].to_string();
        s.push('\u{2026}');
        s
    }
}

// ── Rendering ───────────────────────────────────────────────────

impl ViewerApp {
    /// Render the viewer into the canvas.
    pub fn render(&self, canvas: &mut jag_surface::Canvas, w: f32, h: f32, z_base: i32) {
        canvas.push_transform(Transform2D::translate(self.origin_x, self.origin_y));

        // Background
        canvas.fill_rect(0.0, 0.0, w, h, Brush::Solid(BG), z_base);

        match &self.mode {
            ViewMode::FolderPicker => {
                self.render_picker_toolbar(canvas, w, z_base + 10);
                self.render_picker_list(canvas, w, h, z_base + 30);
            }
            ViewMode::Gallery => {
                self.render_gallery_toolbar(canvas, w, z_base + 10);
                self.render_gallery_grid(canvas, w, h, z_base + 30);
            }
            ViewMode::Preview { index } => {
                let idx = *index;
                self.render_preview_image(canvas, w, h, idx, z_base + 5);
                self.render_preview_toolbar(canvas, w, idx, z_base + 10);
            }
        }

        canvas.pop_transform();
    }

    // ── Shared breadcrumb rendering ─────────────────────────────

    fn render_breadcrumbs(
        &self,
        canvas: &mut jag_surface::Canvas,
        segments: &[(String, std::path::PathBuf)],
        z: i32,
    ) {
        let text_y = (TOOLBAR_HEIGHT + TOOLBAR_FONT_SIZE) / 2.0 - 1.0;
        let chevron_style = SvgStyle::new()
            .with_stroke(TEXT_SECONDARY)
            .with_stroke_width(2.0);
        let mut cx = BREADCRUMB_PAD;

        for (i, (label, _path)) in segments.iter().enumerate() {
            let is_last = i + 1 == segments.len();
            let color = if is_last {
                TEXT_PRIMARY
            } else {
                TEXT_SECONDARY
            };
            let weight = if is_last { 600.0 } else { 400.0 };

            // Home icon for first segment
            if i == 0 {
                let home_style = SvgStyle::new().with_stroke(color).with_stroke_width(2.0);
                canvas.draw_svg_styled(
                    "home.svg",
                    [cx, (TOOLBAR_HEIGHT - 16.0) / 2.0],
                    [16.0, 16.0],
                    home_style,
                    z + 2,
                );
                cx += 20.0;
            }

            canvas.draw_text_run_styled(
                [cx, text_y],
                label.clone(),
                TOOLBAR_FONT_SIZE,
                weight,
                FontStyle::Normal,
                None,
                color,
                z + 2,
            );
            cx += canvas.measure_text_width(label, TOOLBAR_FONT_SIZE);

            if !is_last {
                cx += 6.0;
                canvas.draw_svg_styled(
                    "chevron-right.svg",
                    [cx, (TOOLBAR_HEIGHT - 10.0) / 2.0],
                    [10.0, 10.0],
                    chevron_style,
                    z + 2,
                );
                cx += 14.0;
            }
        }
    }

    // ── Folder picker toolbar ──────────────────────────────────

    fn render_picker_toolbar(&self, canvas: &mut jag_surface::Canvas, w: f32, z: i32) {
        canvas.fill_rect(0.0, 0.0, w, TOOLBAR_HEIGHT, Brush::Solid(TOOLBAR_BG), z);
        canvas.fill_rect(
            0.0,
            TOOLBAR_HEIGHT - 1.0,
            w,
            1.0,
            Brush::Solid(SEPARATOR),
            z + 1,
        );

        // Left: breadcrumbs
        let segments = self.breadcrumb_segments(&self.picker_dir);
        self.render_breadcrumbs(canvas, &segments, z);

        // Right: "Open Here" button
        let btn_x = w - PICKER_OPEN_BTN_W - 8.0;
        let btn_y = (TOOLBAR_HEIGHT - BTN_HEIGHT) / 2.0;
        canvas.rounded_rect(
            RoundedRect {
                rect: Rect {
                    x: btn_x,
                    y: btn_y,
                    w: PICKER_OPEN_BTN_W,
                    h: BTN_HEIGHT,
                },
                radii: rounded_radii(BTN_CORNER),
            },
            Brush::Solid(BTN_BG),
            z + 2,
        );
        let btn_label = "Open Here";
        let btn_label_w = canvas.measure_text_width(btn_label, TOOLBAR_FONT_SIZE);
        canvas.draw_text_run_styled(
            [
                btn_x + (PICKER_OPEN_BTN_W - btn_label_w) / 2.0,
                btn_y + (BTN_HEIGHT + TOOLBAR_FONT_SIZE) / 2.0 - 2.0,
            ],
            btn_label.to_string(),
            TOOLBAR_FONT_SIZE,
            500.0,
            FontStyle::Normal,
            None,
            TEXT_PRIMARY,
            z + 3,
        );
    }

    // ── Folder picker list ───────────────────────────────────────

    fn render_picker_list(&self, canvas: &mut jag_surface::Canvas, w: f32, h: f32, z: i32) {
        let list_top = TOOLBAR_HEIGHT;
        let list_h = h - list_top;

        canvas.push_clip_rect(Rect {
            x: 0.0,
            y: list_top,
            w,
            h: list_h,
        });

        if self.picker_entries.is_empty() {
            let msg = "No folders or images";
            let msg_w = canvas.measure_text_width(msg, PICKER_FONT_SIZE);
            canvas.draw_text_run_styled(
                [(w - msg_w) / 2.0, list_top + 60.0],
                msg.to_string(),
                PICKER_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                TEXT_SECONDARY,
                z,
            );
            canvas.pop_clip();
            return;
        }

        for (i, entry) in self.picker_entries.iter().enumerate() {
            let row_y = list_top + i as f32 * PICKER_ROW_HEIGHT - self.picker_scroll_y;

            if row_y + PICKER_ROW_HEIGHT < list_top || row_y > list_top + list_h {
                continue;
            }

            // Selection highlight
            if self.picker_selected == Some(i) {
                canvas.fill_rect(
                    0.0,
                    row_y,
                    w,
                    PICKER_ROW_HEIGHT,
                    Brush::Solid(SELECTED_BG),
                    z,
                );
            }

            // Separator
            canvas.fill_rect(
                12.0,
                row_y + PICKER_ROW_HEIGHT - 1.0,
                w - 24.0,
                1.0,
                Brush::Solid(SEPARATOR),
                z + 1,
            );

            let text_y = row_y + (PICKER_ROW_HEIGHT + PICKER_FONT_SIZE) / 2.0 - 2.0;
            let icon_y = row_y + (PICKER_ROW_HEIGHT - PICKER_ICON_SIZE) / 2.0;

            if entry.is_dir {
                // Folder SVG icon
                let style = SvgStyle::new()
                    .with_stroke(FOLDER_ICON_COLOR)
                    .with_stroke_width(2.0);
                canvas.draw_svg_styled(
                    "folder-open.svg",
                    [16.0, icon_y],
                    [PICKER_ICON_SIZE, PICKER_ICON_SIZE],
                    style,
                    z + 2,
                );
                canvas.draw_text_run_styled(
                    [PICKER_TEXT_PAD, text_y],
                    entry.name.clone(),
                    PICKER_FONT_SIZE,
                    400.0,
                    FontStyle::Normal,
                    None,
                    TEXT_PRIMARY,
                    z + 2,
                );
                // Right chevron for directories
                let chevron_style = SvgStyle::new()
                    .with_stroke(TEXT_SECONDARY)
                    .with_stroke_width(2.0);
                canvas.draw_svg_styled(
                    "chevron-right.svg",
                    [w - 24.0, row_y + (PICKER_ROW_HEIGHT - 10.0) / 2.0],
                    [10.0, 10.0],
                    chevron_style,
                    z + 2,
                );
            } else {
                // Image SVG icon
                let style = SvgStyle::new()
                    .with_stroke(IMAGE_ICON_COLOR)
                    .with_stroke_width(2.0);
                canvas.draw_svg_styled(
                    "image.svg",
                    [16.0, icon_y],
                    [PICKER_ICON_SIZE, PICKER_ICON_SIZE],
                    style,
                    z + 2,
                );
                canvas.draw_text_run_styled(
                    [PICKER_TEXT_PAD, text_y],
                    entry.name.clone(),
                    PICKER_FONT_SIZE,
                    400.0,
                    FontStyle::Normal,
                    None,
                    TEXT_SECONDARY,
                    z + 2,
                );
            }
        }

        canvas.pop_clip();
    }

    // ── Gallery toolbar ─────────────────────────────────────────

    fn render_gallery_toolbar(&self, canvas: &mut jag_surface::Canvas, w: f32, z: i32) {
        // Bar background
        canvas.fill_rect(0.0, 0.0, w, TOOLBAR_HEIGHT, Brush::Solid(TOOLBAR_BG), z);

        // Bottom separator
        canvas.fill_rect(
            0.0,
            TOOLBAR_HEIGHT - 1.0,
            w,
            1.0,
            Brush::Solid(SEPARATOR),
            z + 1,
        );

        // Left: breadcrumbs
        let segments = self.breadcrumb_segments(&self.current_dir);
        self.render_breadcrumbs(canvas, &segments, z);

        // Right: "Open Folder" button
        let btn_x = w - CHANGE_FOLDER_BTN_W - 8.0;
        let btn_y = (TOOLBAR_HEIGHT - BTN_HEIGHT) / 2.0;
        canvas.rounded_rect(
            RoundedRect {
                rect: Rect {
                    x: btn_x,
                    y: btn_y,
                    w: CHANGE_FOLDER_BTN_W,
                    h: BTN_HEIGHT,
                },
                radii: rounded_radii(BTN_CORNER),
            },
            Brush::Solid(BTN_BG),
            z + 2,
        );
        let btn_label = "Open Folder";
        let btn_label_w = canvas.measure_text_width(btn_label, TOOLBAR_FONT_SIZE);
        canvas.draw_text_run_styled(
            [
                btn_x + (CHANGE_FOLDER_BTN_W - btn_label_w) / 2.0,
                btn_y + (BTN_HEIGHT + TOOLBAR_FONT_SIZE) / 2.0 - 2.0,
            ],
            btn_label.to_string(),
            TOOLBAR_FONT_SIZE,
            500.0,
            FontStyle::Normal,
            None,
            TEXT_PRIMARY,
            z + 3,
        );

        // Between breadcrumbs and button: image count
        if !self.entries.is_empty() {
            let text_y = (TOOLBAR_HEIGHT + TOOLBAR_FONT_SIZE) / 2.0 - 1.0;
            let count_label = format!("{} images", self.entries.len());
            let count_w = canvas.measure_text_width(&count_label, TOOLBAR_FONT_SIZE);
            let count_x = btn_x - count_w - 16.0;
            canvas.draw_text_run_styled(
                [count_x, text_y],
                count_label,
                TOOLBAR_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                TEXT_SECONDARY,
                z + 2,
            );
        }
    }

    // ── Gallery grid ────────────────────────────────────────────

    fn render_gallery_grid(&self, canvas: &mut jag_surface::Canvas, w: f32, h: f32, z: i32) {
        let grid_top = TOOLBAR_HEIGHT;
        let grid_h = h - grid_top;

        canvas.push_clip_rect(Rect {
            x: 0.0,
            y: grid_top,
            w,
            h: grid_h,
        });

        // Empty / loading state
        if self.entries.is_empty() {
            let msg = if self.loading {
                "Loading\u{2026}"
            } else if self.current_dir.as_os_str().is_empty() {
                "Select a folder to browse images"
            } else {
                "No images found in this folder"
            };
            let msg_w = canvas.measure_text_width(msg, TOOLBAR_FONT_SIZE);
            canvas.draw_text_run_styled(
                [(w - msg_w) / 2.0, grid_top + 80.0],
                msg.to_string(),
                TOOLBAR_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                TEXT_SECONDARY,
                z,
            );
            canvas.pop_clip();
            return;
        }

        let cols = self.gallery_cols().max(1);

        for (i, entry) in self.entries.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cell_x = col as f32 * CELL_SIZE;
            let cell_y = grid_top + row as f32 * CELL_SIZE - self.scroll_y;

            // Skip cells outside visible area
            if cell_y + CELL_SIZE < grid_top || cell_y > grid_top + grid_h {
                continue;
            }

            // Selection highlight
            if self.selected == Some(i) {
                canvas.rounded_rect(
                    RoundedRect {
                        rect: Rect {
                            x: cell_x + 2.0,
                            y: cell_y + 2.0,
                            w: CELL_SIZE - 4.0,
                            h: CELL_SIZE - 4.0,
                        },
                        radii: rounded_radii(SELECT_CORNER),
                    },
                    Brush::Solid(SELECTED_BG),
                    z,
                );
            }

            // Thumbnail background placeholder
            let thumb_x = cell_x + THUMB_PADDING;
            let thumb_y = cell_y + THUMB_PADDING;
            canvas.rounded_rect(
                RoundedRect {
                    rect: Rect {
                        x: thumb_x,
                        y: thumb_y,
                        w: THUMB_SIZE,
                        h: THUMB_SIZE,
                    },
                    radii: rounded_radii(THUMB_CORNER),
                },
                Brush::Solid(THUMB_BG),
                z + 1,
            );

            // Image thumbnail
            let path = entry.path.to_string_lossy().to_string();
            match entry.kind {
                EntryKind::Svg => {
                    canvas.draw_svg(path, [thumb_x, thumb_y], [THUMB_SIZE, THUMB_SIZE], z + 2)
                }
                EntryKind::Image => canvas.draw_image(
                    path,
                    [thumb_x, thumb_y],
                    [THUMB_SIZE, THUMB_SIZE],
                    ImageFitMode::Contain,
                    z + 2,
                ),
                EntryKind::Pdf => match self.render_state(&entry.path, 0, crate::THUMB_PX) {
                    Some(RenderState::Ready(png)) => canvas.draw_image(
                        png.to_string_lossy().to_string(),
                        [thumb_x, thumb_y],
                        [THUMB_SIZE, THUMB_SIZE],
                        ImageFitMode::Contain,
                        z + 2,
                    ),
                    // Not drawn yet, or unreadable: the label says what the file is.
                    _ => {
                        let label_w = canvas.measure_text_width("PDF", TOOLBAR_FONT_SIZE);
                        canvas.draw_text_run_styled(
                            [
                                thumb_x + (THUMB_SIZE - label_w) / 2.0,
                                thumb_y + THUMB_SIZE / 2.0,
                            ],
                            "PDF".to_string(),
                            TOOLBAR_FONT_SIZE,
                            600.0,
                            FontStyle::Normal,
                            None,
                            TEXT_SECONDARY,
                            z + 2,
                        );
                    }
                },
            }

            // Filename label, centered below thumbnail
            let label = truncate_label(&entry.name);
            let label_w = canvas.measure_text_width(&label, LABEL_FONT_SIZE);
            let label_x = cell_x + (CELL_SIZE - label_w) / 2.0;
            let label_y = thumb_y + THUMB_SIZE + LABEL_HEIGHT + 4.0;
            canvas.draw_text_run_styled(
                [label_x, label_y],
                label,
                LABEL_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                TEXT_PRIMARY,
                z + 2,
            );
        }

        canvas.pop_clip();
    }

    // ── Preview toolbar ─────────────────────────────────────────

    fn render_preview_toolbar(
        &self,
        canvas: &mut jag_surface::Canvas,
        w: f32,
        index: usize,
        z: i32,
    ) {
        // Bar background
        canvas.fill_rect(0.0, 0.0, w, TOOLBAR_HEIGHT, Brush::Solid(TOOLBAR_BG), z);

        // Bottom separator
        canvas.fill_rect(
            0.0,
            TOOLBAR_HEIGHT - 1.0,
            w,
            1.0,
            Brush::Solid(SEPARATOR),
            z + 1,
        );

        let text_y = (TOOLBAR_HEIGHT + TOOLBAR_FONT_SIZE) / 2.0 - 1.0;

        // Left: back button
        let back_label = "\u{2190} Gallery";
        canvas.draw_text_run_styled(
            [12.0, text_y],
            back_label.to_string(),
            TOOLBAR_FONT_SIZE,
            500.0,
            FontStyle::Normal,
            None,
            TEXT_PRIMARY,
            z + 2,
        );

        // Center: filename
        if let Some(entry) = self.entries.get(index) {
            let name_w = canvas.measure_text_width(&entry.name, TOOLBAR_FONT_SIZE);
            canvas.draw_text_run_styled(
                [(w - name_w) / 2.0, text_y],
                entry.name.clone(),
                TOOLBAR_FONT_SIZE,
                600.0,
                FontStyle::Normal,
                None,
                TEXT_PRIMARY,
                z + 2,
            );
        }

        // Right: position indicator
        let mut pos_label = format!("{} / {}", index + 1, self.entries.len());
        if let Some(entry) = self.entries.get(index)
            && entry.kind == EntryKind::Pdf
        {
            let pages = self
                .page_count(&entry.path)
                .map_or(String::new(), |n| format!(" of {n}"));
            pos_label = format!("Page {}{pages}  ·  {pos_label}", self.preview_page + 1);
        }
        let pos_w = canvas.measure_text_width(&pos_label, TOOLBAR_FONT_SIZE);
        canvas.draw_text_run_styled(
            [w - pos_w - 12.0, text_y],
            pos_label,
            TOOLBAR_FONT_SIZE,
            400.0,
            FontStyle::Normal,
            None,
            TEXT_SECONDARY,
            z + 2,
        );
    }

    // ── Preview image ───────────────────────────────────────────

    fn render_preview_image(
        &self,
        canvas: &mut jag_surface::Canvas,
        w: f32,
        h: f32,
        index: usize,
        z: i32,
    ) {
        let img_top = TOOLBAR_HEIGHT;
        let img_h = h - img_top;

        // Dark background for image area
        canvas.fill_rect(0.0, img_top, w, img_h, Brush::Solid(PREVIEW_BG), z);

        let entry = match self.entries.get(index) {
            Some(e) => e,
            None => return,
        };

        let pad = PREVIEW_PAD;
        let draw_x = pad;
        let draw_y = img_top + pad;
        let draw_w = w - pad * 2.0;
        let draw_h = img_h - pad * 2.0;

        if draw_w <= 0.0 || draw_h <= 0.0 {
            return;
        }

        let path = entry.path.to_string_lossy().to_string();
        match entry.kind {
            EntryKind::Svg => canvas.draw_svg(path, [draw_x, draw_y], [draw_w, draw_h], z + 1),
            EntryKind::Image => canvas.draw_image(
                path,
                [draw_x, draw_y],
                [draw_w, draw_h],
                ImageFitMode::Contain,
                z + 1,
            ),
            EntryKind::Pdf => {
                let state =
                    self.render_state(&entry.path, self.preview_page, self.preview_box_px());
                match state {
                    Some(RenderState::Ready(png)) => {
                        canvas.draw_image(
                            png.to_string_lossy().to_string(),
                            [draw_x, draw_y],
                            [draw_w, draw_h],
                            ImageFitMode::Contain,
                            z + 1,
                        );
                        self.render_page_arrows(canvas, w, h, &entry.path, z + 2);
                    }
                    Some(RenderState::Failed(message)) => {
                        self.render_notice(canvas, w, img_top, message, z + 1)
                    }
                    _ => self.render_notice(canvas, w, img_top, "Rendering\u{2026}", z + 1),
                }
            }
        }
    }

    /// A line of text in the middle of the preview area.
    fn render_notice(
        &self,
        canvas: &mut jag_surface::Canvas,
        w: f32,
        top: f32,
        text: &str,
        z: i32,
    ) {
        let text_w = canvas.measure_text_width(text, TOOLBAR_FONT_SIZE);
        canvas.draw_text_run_styled(
            [((w - text_w) / 2.0).max(PREVIEW_PAD), top + 80.0],
            text.to_string(),
            TOOLBAR_FONT_SIZE,
            400.0,
            FontStyle::Normal,
            None,
            TEXT_PRIMARY,
            z,
        );
    }

    /// A `‹` and a `›` at the sides of the page, where a click turns it; each is left
    /// out at the end of the document it would go past.
    fn render_page_arrows(
        &self,
        canvas: &mut jag_surface::Canvas,
        w: f32,
        h: f32,
        path: &std::path::Path,
        z: i32,
    ) {
        let last = self.page_count(path).map_or(0, |n| n.saturating_sub(1));
        let y = (TOOLBAR_HEIGHT + h) / 2.0;
        let mut arrow = |x: f32, glyph: &str| {
            canvas.draw_text_run_styled(
                [x, y],
                glyph.to_string(),
                ARROW_FONT_SIZE,
                400.0,
                FontStyle::Normal,
                None,
                TEXT_SECONDARY,
                z,
            )
        };
        if self.preview_page > 0 {
            arrow(PREVIEW_PAD / 2.0, "\u{2039}");
        }
        if self.preview_page < last {
            arrow(w - PREVIEW_PAD * 1.5, "\u{203a}");
        }
    }
}
