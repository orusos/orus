use std::path::Path;

use jag_draw::Brush;
use jag_surface::Canvas;

use crate::markdown::model::{BlockKind, InlineSpan, ListItem};
use crate::theme::Theme;

use super::inline::{Pen, line_height_for_size, render_spans};
use super::style::SpanStyle;
use super::table::render_table;

/// Spacing constants for block layout.
const BLOCK_GAP: f32 = 12.0;
const H1_TOP_MARGIN: f32 = 24.0;
const H2_TOP_MARGIN: f32 = 20.0;
const H3_TOP_MARGIN: f32 = 16.0;
const LIST_INDENT: f32 = 24.0;
const BLOCKQUOTE_INDENT: f32 = 20.0;
const BLOCKQUOTE_BORDER_WIDTH: f32 = 3.0;
const CODE_BLOCK_PAD: f32 = 12.0;

/// Render a single block at the given y position. Returns height consumed.
///
/// `depth` tracks list nesting level for bullet style variation.
/// `base_dir` is used to resolve relative image paths.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_block(
    canvas: &mut Canvas,
    kind: &BlockKind,
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
    depth: u32,
    base_dir: Option<&Path>,
) -> f32 {
    match kind {
        BlockKind::Paragraph { spans } => {
            render_paragraph(canvas, spans, x, y, max_width, theme, z, base_dir)
        }
        BlockKind::Heading { level, spans } => {
            render_heading(canvas, *level, spans, x, y, max_width, theme, z, base_dir)
        }
        BlockKind::CodeBlock { language, code } => {
            render_code_block(canvas, language.as_deref(), code, x, y, max_width, theme, z)
        }
        BlockKind::BlockQuote { children } => {
            render_blockquote(canvas, children, x, y, max_width, theme, z, depth, base_dir)
        }
        BlockKind::OrderedList { start, items } => render_ordered_list(
            canvas, *start, items, x, y, max_width, theme, z, depth, base_dir,
        ),
        BlockKind::UnorderedList { items } => {
            render_unordered_list(canvas, items, x, y, max_width, theme, z, depth, base_dir)
        }
        BlockKind::Table {
            headers,
            rows,
            alignments,
        } => render_table(canvas, headers, rows, alignments, x, y, max_width, theme, z),
        BlockKind::ThematicBreak => render_thematic_break(canvas, x, y, max_width, theme, z),
    }
}

/// Top margin before a block (depends on block type and whether it's the first).
pub(crate) fn top_margin(kind: &BlockKind, is_first: bool) -> f32 {
    if is_first {
        return 0.0;
    }
    match kind {
        BlockKind::Heading { level: 1, .. } => H1_TOP_MARGIN,
        BlockKind::Heading { level: 2, .. } => H2_TOP_MARGIN,
        BlockKind::Heading { level: 3, .. } => H3_TOP_MARGIN,
        _ => BLOCK_GAP,
    }
}

// ---------------------------------------------------------------------------
// Block renderers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn render_paragraph(
    canvas: &mut Canvas,
    spans: &[InlineSpan],
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
    base_dir: Option<&Path>,
) -> f32 {
    let style = SpanStyle::body(theme);
    let lh = line_height_for_size(style.size);
    let mut pen = Pen::new(x, y, max_width, lh);
    render_spans(canvas, spans, &mut pen, &style, theme, z, base_dir);
    pen.height_from(y)
}

#[allow(clippy::too_many_arguments)]
fn render_heading(
    canvas: &mut Canvas,
    level: u8,
    spans: &[InlineSpan],
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
    base_dir: Option<&Path>,
) -> f32 {
    let style = SpanStyle::heading(level, theme);
    let lh = line_height_for_size(style.size);
    let mut pen = Pen::new(x, y, max_width, lh);
    render_spans(canvas, spans, &mut pen, &style, theme, z, base_dir);
    let h = pen.height_from(y);

    // Underline for h1 and h2.
    if level <= 2 {
        let line_y = y + h + 4.0;
        canvas.fill_rect(x, line_y, max_width, 1.0, Brush::Solid(theme.md_hr), z);
        return h + 8.0;
    }
    h
}

#[allow(clippy::too_many_arguments)]
fn render_code_block(
    canvas: &mut Canvas,
    language: Option<&str>,
    code: &str,
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
) -> f32 {
    let font_size = 13.0;
    let lh = line_height_for_size(font_size);
    let line_count = code.lines().count().max(1);
    let content_h = line_count as f32 * lh;
    let total_h = content_h + CODE_BLOCK_PAD * 2.0;

    // Background.
    canvas.fill_rect(
        x,
        y,
        max_width,
        total_h,
        Brush::Solid(theme.md_code_block_bg),
        z,
    );

    // Left accent border.
    canvas.fill_rect(
        x,
        y,
        3.0,
        total_h,
        Brush::Solid(theme.md_code_block_border),
        z + 1,
    );

    // Syntax-highlighted code lines (monospace, no wrapping).
    let text_x = x + CODE_BLOCK_PAD + 3.0;
    let highlighted = super::code_highlight::highlight_code(code, language);

    for (i, tokens) in highlighted.iter().enumerate() {
        let baseline_y = y + CODE_BLOCK_PAD + i as f32 * lh + lh * 0.75;
        let mut token_x = text_x;

        for token in tokens {
            if token.text.is_empty() {
                continue;
            }
            let weight = if token.bold { 700.0 } else { 400.0 };
            let font_style = if token.italic {
                jag_draw::FontStyle::Italic
            } else {
                jag_draw::FontStyle::Normal
            };
            canvas.draw_text_run_styled(
                [token_x, baseline_y],
                token.text.clone(),
                font_size,
                weight,
                font_style,
                Some("monospace".to_string()),
                token.color,
                z + 2,
            );
            let w = canvas.measure_text_width_styled(
                &token.text,
                font_size,
                weight,
                font_style,
                Some("monospace"),
            );
            token_x += w;
        }
    }

    total_h
}

#[allow(clippy::too_many_arguments)]
fn render_blockquote(
    canvas: &mut Canvas,
    children: &[crate::markdown::model::Block],
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
    depth: u32,
    base_dir: Option<&Path>,
) -> f32 {
    // Left border.
    canvas.fill_rect(
        x,
        y,
        BLOCKQUOTE_BORDER_WIDTH,
        1.0, // placeholder height — will be extended below
        Brush::Solid(theme.md_blockquote_border),
        z,
    );

    let inner_x = x + BLOCKQUOTE_INDENT;
    let inner_width = (max_width - BLOCKQUOTE_INDENT).max(50.0);
    let mut cursor_y = y;

    for (i, child) in children.iter().enumerate() {
        cursor_y += top_margin(&child.kind, i == 0);
        let h = render_block(
            canvas,
            &child.kind,
            inner_x,
            cursor_y,
            inner_width,
            theme,
            z,
            depth,
            base_dir,
        );
        cursor_y += h;
    }

    let total_h = (cursor_y - y).max(line_height_for_size(15.0));

    // Extend border to full height.
    canvas.fill_rect(
        x,
        y,
        BLOCKQUOTE_BORDER_WIDTH,
        total_h,
        Brush::Solid(theme.md_blockquote_border),
        z,
    );

    total_h
}

#[allow(clippy::too_many_arguments)]
fn render_ordered_list(
    canvas: &mut Canvas,
    start: u64,
    items: &[ListItem],
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
    depth: u32,
    base_dir: Option<&Path>,
) -> f32 {
    let body_style = SpanStyle::body(theme);
    let lh = line_height_for_size(body_style.size);
    let mut cursor_y = y;

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            cursor_y += 4.0;
        }
        let num = start + i as u64;
        let marker = format!("{num}.");
        let baseline = cursor_y + lh * 0.75;

        // Number marker.
        let marker_w = canvas.measure_text_width(&marker, body_style.size);
        canvas.draw_text_run_styled(
            [x + LIST_INDENT - marker_w - 4.0, baseline],
            marker,
            body_style.size,
            400.0,
            jag_draw::FontStyle::Normal,
            None,
            theme.md_bullet,
            z,
        );

        let item_h = render_list_item_content(
            canvas,
            item,
            x + LIST_INDENT,
            cursor_y,
            max_width - LIST_INDENT,
            theme,
            z,
            depth,
            base_dir,
        );
        cursor_y += item_h;
    }

    cursor_y - y
}

#[allow(clippy::too_many_arguments)]
fn render_unordered_list(
    canvas: &mut Canvas,
    items: &[ListItem],
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
    depth: u32,
    base_dir: Option<&Path>,
) -> f32 {
    let body_style = SpanStyle::body(theme);
    let lh = line_height_for_size(body_style.size);
    let mut cursor_y = y;

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            cursor_y += 4.0;
        }
        let baseline = cursor_y + lh * 0.75;

        if let Some(checked) = item.checked {
            // Task list checkbox.
            let marker = if checked { "\u{2611}" } else { "\u{2610}" };
            canvas.draw_text_run_styled(
                [x + 2.0, baseline],
                marker.to_string(),
                body_style.size,
                400.0,
                jag_draw::FontStyle::Normal,
                None,
                theme.md_checkbox,
                z,
            );
        } else {
            // Depth-varying bullet: disc → circle → en-dash.
            let bullet = match depth % 3 {
                0 => "\u{2022}", // disc •
                1 => "\u{25E6}", // circle ◦
                _ => "\u{2013}", // en-dash –
            };
            let bullet_w = canvas.measure_text_width(bullet, body_style.size);
            canvas.draw_text_run_styled(
                [x + LIST_INDENT * 0.5 - bullet_w * 0.5, baseline],
                bullet.to_string(),
                body_style.size,
                400.0,
                jag_draw::FontStyle::Normal,
                None,
                theme.md_bullet,
                z,
            );
        }

        let item_h = render_list_item_content(
            canvas,
            item,
            x + LIST_INDENT,
            cursor_y,
            max_width - LIST_INDENT,
            theme,
            z,
            depth,
            base_dir,
        );
        cursor_y += item_h;
    }

    cursor_y - y
}

/// Render the children of a list item.
#[allow(clippy::too_many_arguments)]
fn render_list_item_content(
    canvas: &mut Canvas,
    item: &ListItem,
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
    depth: u32,
    base_dir: Option<&Path>,
) -> f32 {
    let mut cursor_y = y;
    for (i, child) in item.children.iter().enumerate() {
        cursor_y += top_margin(&child.kind, i == 0);
        // Nested lists increment depth for bullet style variation.
        let child_depth = match &child.kind {
            BlockKind::OrderedList { .. } | BlockKind::UnorderedList { .. } => depth + 1,
            _ => depth,
        };
        let h = render_block(
            canvas,
            &child.kind,
            x,
            cursor_y,
            max_width,
            theme,
            z,
            child_depth,
            base_dir,
        );
        cursor_y += h;
    }
    (cursor_y - y).max(line_height_for_size(15.0))
}

fn render_thematic_break(
    canvas: &mut Canvas,
    x: f32,
    y: f32,
    max_width: f32,
    theme: &Theme,
    z: i32,
) -> f32 {
    let pad = 8.0;
    canvas.fill_rect(x, y + pad, max_width, 1.0, Brush::Solid(theme.md_hr), z);
    pad * 2.0 + 1.0
}
