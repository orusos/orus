use std::ops::Range;

use jag_draw::{Brush, FillRule, Path, PathCmd, Rect};
use jag_surface::Canvas;
use jag_text::layout::CursorPosition;

use super::syntax::highlight_line;
use crate::editor::{EditorBuffer, TEXT_SIZE};
use crate::theme::Theme;

const FOCUSED_PAD_X: f32 = 8.0;
const FOCUSED_PAD_Y: f32 = 4.0;
const BORDER_WIDTH: f32 = 3.0;

/// A single markdown block opened for raw-text editing.
///
/// Wraps an `EditorBuffer` positioned over the block's screen area so all
/// editing operations (cursor, selection, undo/redo, clipboard) are reused.
pub(crate) struct FocusedBlock {
    pub block_index: usize,
    pub source_range: Range<usize>,
    pub editor: EditorBuffer,
    /// True if this block is a list (ordered or unordered). Used for smart Tab indent.
    pub is_list: bool,
}

impl FocusedBlock {
    /// Create a focused block from a block's source range and raw text.
    pub fn new(
        block_index: usize,
        source_range: Range<usize>,
        raw_text: &str,
        rect: Rect,
        is_list: bool,
    ) -> Self {
        let mut editor = EditorBuffer::new();
        // Position the editor at the block location (no gutter needed).
        editor.rect = rect;
        editor.set_text(raw_text.to_string());
        // Place cursor at end of text for immediate editing.
        editor.cursor_pos = raw_text.len();
        editor.selection = jag_text::layout::Selection::collapsed(raw_text.len());
        editor.focused = true;
        Self {
            block_index,
            source_range,
            editor,
            is_list,
        }
    }

    /// Reposition the editor rect (called when scroll/resize changes geometry).
    pub fn update_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let new_rect = Rect { x, y, w, h };
        let old_w = self.editor.rect.w;
        self.editor.rect = new_rect;
        if (old_w - w).abs() > 1.0 {
            self.editor.rebuild_layout();
        }
    }

    /// The edited text (for committing back to source).
    pub fn committed_text(&self) -> &str {
        &self.editor.text
    }

    /// Returns true if the screen point (x, y) falls inside this block's area.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let r = &self.editor.rect;
        x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
    }

    /// True if cursor is on the first visual line of this block.
    pub fn is_at_first_line(&self) -> bool {
        self.editor.current_line() == 0
    }

    /// True if cursor is on the last visual line of this block.
    pub fn is_at_last_line(&self) -> bool {
        self.editor.current_line() >= self.editor.line_count().saturating_sub(1)
    }

    /// True if cursor is at the very start of the block (byte offset 0).
    pub fn is_at_start(&self) -> bool {
        self.editor.cursor_pos == 0 && self.editor.selection.is_collapsed()
    }

    /// True if cursor is at the very end of the block text.
    pub fn is_at_end(&self) -> bool {
        self.editor.cursor_pos >= self.editor.text.len() && self.editor.selection.is_collapsed()
    }

    /// Total height of the focused block's editor content.
    pub fn height(&self) -> f32 {
        let content_h = self.editor.total_height();
        content_h + FOCUSED_PAD_Y * 2.0
    }

    /// Render the focused block: subtle background, left accent, text, cursor.
    pub fn render(&self, canvas: &mut Canvas, theme: &Theme, z: i32) {
        let r = &self.editor.rect;

        // Background.
        canvas.fill_rect(
            r.x - FOCUSED_PAD_X,
            r.y - FOCUSED_PAD_Y,
            r.w + FOCUSED_PAD_X * 2.0,
            self.height(),
            Brush::Solid(theme.md_focused_bg),
            z,
        );

        // Left accent border.
        canvas.fill_rect(
            r.x - FOCUSED_PAD_X,
            r.y - FOCUSED_PAD_Y,
            BORDER_WIDTH,
            self.height(),
            Brush::Solid(theme.md_focused_border),
            z + 1,
        );

        let Some(layout) = self.editor.layout.as_ref() else {
            return;
        };

        let line_h = self.editor.line_height();

        // Clip to block area.
        canvas.push_clip_rect(Rect {
            x: r.x - FOCUSED_PAD_X,
            y: r.y - FOCUSED_PAD_Y,
            w: r.w + FOCUSED_PAD_X * 2.0,
            h: self.height(),
        });

        // Selection rectangles.
        if !self.editor.selection.is_collapsed() {
            for sel_rect in layout.selection_rects(&self.editor.selection) {
                let hx = r.x + sel_rect.x;
                let hy = r.y + sel_rect.y;
                canvas.fill_rect(
                    hx,
                    hy,
                    sel_rect.width,
                    sel_rect.height,
                    Brush::Solid(theme.selection_bg),
                    z + 2,
                );
            }
        }

        // Text lines with syntax highlighting.
        for (i, line) in layout.lines().iter().enumerate() {
            let line_top = r.y + i as f32 * line_h;
            let range = line.text_range.clone();
            if range.start > self.editor.text.len() || range.end > self.editor.text.len() {
                continue;
            }
            let line_text = &self.editor.text[range];
            let baseline_y = line_top + line_h * 0.75;

            let spans = highlight_line(line_text, theme);
            let mut pen_x = r.x;
            for span in &spans {
                let text = &line_text[span.start..span.end];
                if text.is_empty() {
                    continue;
                }
                canvas.draw_text_run_styled(
                    [pen_x, baseline_y],
                    text.to_string(),
                    TEXT_SIZE,
                    span.weight,
                    span.style,
                    None,
                    span.color,
                    z + 3,
                );
                pen_x += canvas.measure_text_width(text, TEXT_SIZE);
            }
        }

        // Caret.
        if self.editor.caret.visible && self.editor.selection.is_collapsed() {
            let pos = CursorPosition::new(self.editor.cursor_pos.min(layout.text().len()));
            if let Some(cursor_rect) = layout.cursor_rect_at_position(pos) {
                let cx = r.x + cursor_rect.x;
                let cy0 = r.y + cursor_rect.y;
                let cy1 = cy0 + cursor_rect.height;
                let mut path = Path {
                    cmds: Vec::new(),
                    fill_rule: FillRule::NonZero,
                };
                path.cmds.push(PathCmd::MoveTo([cx, cy0]));
                path.cmds.push(PathCmd::LineTo([cx, cy1]));
                canvas.stroke_path(path, 1.5, theme.caret_color, z + 4);
            }
        }

        canvas.pop_clip();
    }
}
