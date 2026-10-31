pub(crate) mod blocks;
pub(crate) mod code_highlight;
pub(crate) mod inline;
pub(crate) mod style;
pub(crate) mod table;

use jag_draw::Rect;
use jag_surface::Canvas;

use crate::markdown::model::Document;
use crate::theme::Theme;

use blocks::{render_block, top_margin};

/// Content padding for the markdown preview area.
pub(crate) const MD_PAD_X: f32 = 32.0;
pub(crate) const MD_PAD_TOP: f32 = 16.0;

/// Screen-space geometry of a rendered block (for hit-testing clicks).
pub(crate) struct BlockGeometry {
    pub block_index: usize,
    pub screen_y: f32,
    pub height: f32,
}

/// Render a parsed markdown Document into the given rect.
///
/// Clips to `rect`, applies `scroll_y` offset, and returns `(total_height,
/// block_geometries)` — the total content height and per-block screen rects.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_document(
    canvas: &mut Canvas,
    doc: &Document,
    rect: Rect,
    scroll_y: f32,
    theme: &Theme,
    z: i32,
    base_dir: Option<&std::path::Path>,
) -> (f32, Vec<BlockGeometry>) {
    let content_x = rect.x + MD_PAD_X;
    let max_width = (rect.w - MD_PAD_X * 2.0).max(100.0);

    canvas.push_clip_rect(rect);

    let mut cursor_y = MD_PAD_TOP;
    let mut geos = Vec::with_capacity(doc.blocks.len());

    for (i, block) in doc.blocks.iter().enumerate() {
        let margin = top_margin(&block.kind, i == 0);
        cursor_y += margin;

        // Screen-space y for this block (accounting for scroll).
        let screen_y = rect.y + cursor_y - scroll_y;

        // Skip blocks entirely above the viewport.
        // We still need to compute height for correct total_height.
        // Estimate: render but let clipping hide it. For large docs, a proper
        // culling pass would pre-compute heights, but for Phase 3 correctness
        // we render all blocks and rely on clip.
        let h = if screen_y + 500.0 < rect.y {
            // Far above — estimate height to skip rendering.
            estimate_block_height(&block.kind, max_width)
        } else if screen_y > rect.y + rect.h {
            // Below viewport — also estimate.
            estimate_block_height(&block.kind, max_width)
        } else {
            // Visible — actually render.
            render_block(
                canvas,
                &block.kind,
                content_x,
                screen_y,
                max_width,
                theme,
                z,
                0,
                base_dir,
            )
        };

        geos.push(BlockGeometry {
            block_index: i,
            screen_y,
            height: h,
        });

        cursor_y += h;
    }

    canvas.pop_clip();

    (cursor_y + MD_PAD_TOP, geos) // total content height (top + bottom padding)
}

/// Rough height estimate for off-screen blocks (avoids rendering cost).
pub(crate) fn estimate_block_height(
    kind: &crate::markdown::model::BlockKind,
    _max_width: f32,
) -> f32 {
    use crate::markdown::model::BlockKind;
    match kind {
        BlockKind::Paragraph { spans } => {
            let char_count: usize = spans.iter().map(span_char_count).sum();
            let lines = (char_count as f32 / 80.0).ceil().max(1.0);
            lines * 22.5
        }
        BlockKind::Heading { level, .. } => {
            let size = match level {
                1 => 32.0,
                2 => 26.0,
                3 => 22.0,
                _ => 18.0,
            };
            size * 1.5 + 8.0
        }
        BlockKind::CodeBlock { code, .. } => {
            let lines = code.lines().count().max(1);
            lines as f32 * 19.5 + 24.0
        }
        BlockKind::BlockQuote { children } => children.len().max(1) as f32 * 22.5 + 8.0,
        BlockKind::OrderedList { items, .. } | BlockKind::UnorderedList { items } => {
            items.len() as f32 * 26.0
        }
        BlockKind::Table { rows, .. } => (rows.len() + 1) as f32 * 35.0,
        BlockKind::ThematicBreak => 17.0,
    }
}

fn span_char_count(span: &crate::markdown::model::InlineSpan) -> usize {
    use crate::markdown::model::InlineSpan;
    match span {
        InlineSpan::Text(t) | InlineSpan::Code(t) => t.len(),
        InlineSpan::Bold(ch) | InlineSpan::Italic(ch) | InlineSpan::Strikethrough(ch) => {
            ch.iter().map(span_char_count).sum()
        }
        InlineSpan::Link { text, .. } => text.iter().map(span_char_count).sum(),
        InlineSpan::Image { alt, .. } => alt.len(),
        InlineSpan::SoftBreak | InlineSpan::HardBreak => 1,
    }
}
