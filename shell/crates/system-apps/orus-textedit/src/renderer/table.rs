use jag_draw::Brush;
use jag_surface::Canvas;

use crate::markdown::model::{Alignment, InlineSpan};
use crate::theme::Theme;

use super::inline::{Pen, line_height_for_size, measure_span_width, render_spans};
use super::style::SpanStyle;

const TABLE_CELL_PAD_X: f32 = 10.0;
const TABLE_CELL_PAD_Y: f32 = 6.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_table(
    canvas: &mut Canvas,
    headers: &[Vec<InlineSpan>],
    rows: &[Vec<Vec<InlineSpan>>],
    alignments: &[Alignment],
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
) -> f32 {
    let col_count = headers.len().max(1);
    let col_w = (max_width / col_count as f32).min(200.0);
    let body_style = SpanStyle::body(theme);
    let lh = line_height_for_size(body_style.size);
    let row_h = lh + TABLE_CELL_PAD_Y * 2.0;

    let table_w = col_w * col_count as f32;
    let mut cursor_y = y;

    // Header row background.
    canvas.fill_rect(
        x,
        cursor_y,
        table_w,
        row_h,
        Brush::Solid(theme.md_table_header_bg),
        z,
    );

    // Header cells.
    let header_style = body_style.with_bold();
    let cell_inner_w = col_w - TABLE_CELL_PAD_X * 2.0;
    for (ci, cell) in headers.iter().enumerate() {
        let align = alignments.get(ci).copied().unwrap_or(Alignment::None);
        let text_w: f32 = cell
            .iter()
            .map(|s| measure_span_width(canvas, s, &header_style))
            .sum();
        let offset = alignment_offset(align, cell_inner_w, text_w);
        let cx = x + ci as f32 * col_w + TABLE_CELL_PAD_X + offset;
        let mut pen = Pen::new(cx, cursor_y + TABLE_CELL_PAD_Y, cell_inner_w - offset, lh);
        render_spans(canvas, cell, &mut pen, &header_style, theme, z + 2, None);
    }
    cursor_y += row_h;

    // Header bottom border.
    canvas.fill_rect(
        x,
        cursor_y,
        table_w,
        1.0,
        Brush::Solid(theme.md_table_border),
        z + 1,
    );
    cursor_y += 1.0;

    // Data rows.
    for row in rows {
        for (ci, cell) in row.iter().enumerate() {
            let align = alignments.get(ci).copied().unwrap_or(Alignment::None);
            let text_w: f32 = cell
                .iter()
                .map(|s| measure_span_width(canvas, s, &body_style))
                .sum();
            let offset = alignment_offset(align, cell_inner_w, text_w);
            let cx = x + ci as f32 * col_w + TABLE_CELL_PAD_X + offset;
            let mut pen = Pen::new(cx, cursor_y + TABLE_CELL_PAD_Y, cell_inner_w - offset, lh);
            render_spans(canvas, cell, &mut pen, &body_style, theme, z + 2, None);
        }
        cursor_y += row_h;

        // Row border.
        canvas.fill_rect(
            x,
            cursor_y,
            table_w,
            1.0,
            Brush::Solid(theme.md_table_border),
            z + 1,
        );
        cursor_y += 1.0;
    }

    // Vertical column borders.
    let total_h = cursor_y - y;
    for ci in 0..=col_count {
        let bx = x + ci as f32 * col_w;
        canvas.fill_rect(
            bx,
            y,
            1.0,
            total_h,
            Brush::Solid(theme.md_table_border),
            z + 1,
        );
    }

    total_h
}

fn alignment_offset(align: Alignment, cell_w: f32, text_w: f32) -> f32 {
    match align {
        Alignment::Center => ((cell_w - text_w) / 2.0).max(0.0),
        Alignment::Right => (cell_w - text_w).max(0.0),
        Alignment::None | Alignment::Left => 0.0,
    }
}
