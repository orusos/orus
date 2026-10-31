use jag_draw::{Brush, ColorLinPremul, FontStyle, Rect, TextProvider, TextRun};

use crate::TerminalCore;
use crate::config::TerminalConfig;
use crate::grid::CellAttrs;

/// Precomputed cell metrics for a given font configuration.
pub struct CellMetrics {
    pub cell_width: f32,
    pub cell_height: f32,
    pub ascent: f32,
}

impl CellMetrics {
    /// Compute cell dimensions from a text provider and config.
    pub fn measure(provider: &dyn TextProvider, cfg: &TerminalConfig) -> Self {
        let metrics = provider
            .line_metrics(cfg.font_size)
            .expect("font metrics unavailable");
        let cell_height = metrics.ascent + metrics.descent + metrics.line_gap;
        let ascent = metrics.ascent;

        let cell_width = {
            let run = TextRun {
                text: "MM".to_string(),
                pos: [0.0, 0.0],
                size: cfg.font_size,
                logical_size: 0.0,
                color: ColorLinPremul::from_srgba_u8([255, 255, 255, 255]),
                weight: 400.0,
                style: FontStyle::Normal,
                family: Some(cfg.font_family.clone()),
            };
            let glyphs = provider.rasterize_run(&run);
            if glyphs.len() >= 2 {
                (glyphs[1].offset[0] - glyphs[0].offset[0]).abs()
            } else {
                cfg.font_size * 0.6
            }
        };

        Self {
            cell_width,
            cell_height,
            ascent,
        }
    }

    /// How many columns fit in the given width (with padding).
    pub fn cols_for_width(&self, width: f32, padding: f32) -> usize {
        ((width - padding * 2.0) / self.cell_width).floor().max(1.0) as usize
    }

    /// How many rows fit in the given height (with padding).
    pub fn rows_for_height(&self, height: f32, padding: f32) -> usize {
        ((height - padding * 2.0) / self.cell_height)
            .floor()
            .max(1.0) as usize
    }
}

/// Renders terminal grid content into a Canvas region.
///
/// Create once with a config + text provider, then call `render()` each frame.
/// This does NOT own the terminal state or the window — the compositor does.
pub struct TerminalRenderer {
    pub metrics: CellMetrics,
    pub cfg: TerminalConfig,
}

impl TerminalRenderer {
    pub fn new(provider: &dyn TextProvider, cfg: TerminalConfig) -> Self {
        let metrics = CellMetrics::measure(provider, &cfg);
        Self { metrics, cfg }
    }

    /// Render the terminal grid into `region` on the canvas at the given z-base.
    ///
    /// `region` is the content area (inside the window frame).
    /// All drawing is clipped to this region conceptually (coordinates offset).
    pub fn render(
        &self,
        core: &TerminalCore,
        canvas: &mut jag_surface::Canvas,
        region: Rect,
        z: i32,
    ) {
        let pad = self.cfg.padding;
        let cw = self.metrics.cell_width;
        let ch = self.metrics.cell_height;
        let ascent = self.metrics.ascent;
        let family = Some(self.cfg.font_family.clone());

        // Background fill
        let bg_color = ColorLinPremul::from_srgba_u8(self.cfg.bg);
        canvas.fill_rect(
            region.x,
            region.y,
            region.w,
            region.h,
            Brush::Solid(bg_color),
            z,
        );

        let grid = &core.state.grid;
        let total = grid.scrollback_len() + grid.rows;
        let max_visible = ((region.h - pad * 2.0) / ch).floor() as usize;
        let view_start = total
            .saturating_sub(max_visible)
            .saturating_sub(core.scroll_offset);

        for view_row in 0..max_visible.min(total) {
            let global_row = view_start + view_row;
            let row_data = if global_row < grid.scrollback_len() {
                &grid.scrollback[global_row]
            } else {
                &grid.cells[global_row - grid.scrollback_len()]
            };

            let y_base = region.y + pad + (view_row as f32) * ch;
            let y_text = y_base + ascent;

            let mut col = 0;
            while col < row_data.len() {
                let span_attrs = row_data[col].attrs;
                let span_start = col;
                while col < row_data.len() && row_data[col].attrs == span_attrs {
                    col += 1;
                }
                let span_len = col - span_start;

                let (fg, bg) = if span_attrs.flags & CellAttrs::INVERSE != 0 {
                    (span_attrs.bg, span_attrs.fg)
                } else {
                    (span_attrs.fg, span_attrs.bg)
                };

                let x = region.x + pad + span_start as f32 * cw;

                if bg != CellAttrs::DEFAULT_BG {
                    canvas.fill_rect(
                        x,
                        y_base,
                        span_len as f32 * cw,
                        ch,
                        Brush::Solid(ColorLinPremul::from_srgba_u8([bg[0], bg[1], bg[2], 255])),
                        z + 1,
                    );
                }

                let text: String = row_data[span_start..col].iter().map(|c| c.c).collect();
                let weight = if span_attrs.flags & CellAttrs::BOLD != 0 {
                    700.0
                } else if span_attrs.flags & CellAttrs::DIM != 0 {
                    300.0
                } else {
                    400.0
                };
                let style = if span_attrs.flags & CellAttrs::ITALIC != 0 {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                };
                let fg_color = ColorLinPremul::from_srgba_u8([fg[0], fg[1], fg[2], 255]);

                canvas.draw_text_run_styled(
                    [x, y_text],
                    text,
                    self.cfg.font_size,
                    weight,
                    style,
                    family.clone(),
                    fg_color,
                    z + 2,
                );
            }
        }

        // Cursor
        if core.scroll_offset == 0 && core.state.cursor_visible {
            let (cr, cc) = core.state.cursor;
            let cursor_x = region.x + pad + cc as f32 * cw;
            let cursor_row_on_screen = max_visible.saturating_sub(grid.rows) + cr;
            let cursor_y = region.y + pad + cursor_row_on_screen as f32 * ch;

            if cursor_row_on_screen < max_visible {
                let cursor_color = ColorLinPremul::from_srgba_u8([204, 204, 204, 200]);
                canvas.fill_rect(
                    cursor_x,
                    cursor_y,
                    cw,
                    ch,
                    Brush::Solid(cursor_color),
                    z + 3,
                );
            }
        }

        // Scrollbar
        if total > max_visible && max_visible > 0 {
            let bar_w = 6.0_f32;
            let track_x = region.x + region.w - bar_w - 2.0;
            let track_h = region.h;

            canvas.fill_rect(
                track_x,
                region.y,
                bar_w,
                track_h,
                Brush::Solid(ColorLinPremul::from_srgba_u8([50, 50, 50, 100])),
                z + 4,
            );

            let ratio = max_visible as f32 / total as f32;
            let thumb_h = (track_h * ratio).max(20.0);
            let max_off = total.saturating_sub(max_visible) as f32;
            let scroll_frac = if max_off > 0.0 {
                1.0 - (core.scroll_offset as f32 / max_off)
            } else {
                1.0
            };
            let thumb_y = region.y + (track_h - thumb_h) * scroll_frac;
            canvas.fill_rect(
                track_x,
                thumb_y,
                bar_w,
                thumb_h,
                Brush::Solid(ColorLinPremul::from_srgba_u8([150, 150, 150, 180])),
                z + 5,
            );
        }
    }
}
