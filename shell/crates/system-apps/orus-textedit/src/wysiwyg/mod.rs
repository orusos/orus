pub(crate) mod focused_block;
pub(crate) mod syntax;

use jag_draw::Rect;
use jag_surface::Canvas;

use crate::markdown::model::{Block, BlockKind, Document};
use crate::markdown::parser::parse;
use crate::markdown::source_map::SourceMap;
use crate::renderer::blocks::{render_block, top_margin};
use crate::renderer::{BlockGeometry, MD_PAD_TOP, MD_PAD_X};
use crate::theme::Theme;

use focused_block::FocusedBlock;

/// WYSIWYG state: manages the rich view with per-block inline editing.
pub(crate) struct WysiwygState {
    pub md_doc: Document,
    pub source_map: SourceMap,
    pub block_geos: Vec<BlockGeometry>,
    pub focused: Option<FocusedBlock>,
    pub scroll_y: f32,
    pub total_height: f32,
    last_parsed_text: String,
}

impl WysiwygState {
    /// Create from current source text.
    pub fn new(source: &str) -> Self {
        let md_doc = parse(source);
        let source_map = SourceMap::from_document(&md_doc);
        Self {
            md_doc,
            source_map,
            block_geos: Vec::new(),
            focused: None,
            scroll_y: 0.0,
            total_height: 0.0,
            last_parsed_text: source.to_string(),
        }
    }

    /// Re-parse if the source text changed (e.g. after toggling from source mode).
    pub fn reparse_if_needed(&mut self, source: &str) {
        if source != self.last_parsed_text {
            self.full_reparse(source);
        }
    }

    /// Render the WYSIWYG view: rich blocks + focused raw editor.
    pub fn render_wysiwyg(
        &mut self,
        canvas: &mut Canvas,
        source: &str,
        rect: Rect,
        theme: &Theme,
        z: i32,
        base_dir: Option<&std::path::Path>,
    ) {
        self.reparse_if_needed(source);

        let content_x = rect.x + MD_PAD_X;
        let max_width = (rect.w - MD_PAD_X * 2.0).max(100.0);

        canvas.push_clip_rect(rect);

        let mut cursor_y = MD_PAD_TOP;
        let mut geos = Vec::with_capacity(self.md_doc.blocks.len());

        for (i, block) in self.md_doc.blocks.iter().enumerate() {
            let margin = top_margin(&block.kind, i == 0);
            cursor_y += margin;

            let screen_y = rect.y + cursor_y - self.scroll_y;

            let h = if self.focused.as_ref().is_some_and(|f| f.block_index == i) {
                // Focused block: render as raw editor.
                let focused = self.focused.as_mut().unwrap();
                focused.update_rect(content_x, screen_y, max_width, focused.height());
                focused.render(canvas, theme, z + 1);
                focused.height()
            } else if screen_y + 500.0 < rect.y || screen_y > rect.y + rect.h {
                crate::renderer::estimate_block_height(&block.kind, max_width)
            } else {
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

        self.total_height = cursor_y + MD_PAD_TOP;
        self.block_geos = geos;
    }

    /// Handle a mouse click in the WYSIWYG area. Returns true if consumed.
    pub fn handle_click(&mut self, x: f32, y: f32, double: bool, source: &mut String) -> bool {
        // If there's a focused block and the click is inside it, delegate.
        if let Some(ref mut focused) = self.focused {
            if focused.contains(x, y) {
                focused.editor.handle_mouse_click(x, y, double);
                return true;
            }
            // Click outside focused block → commit.
            self.commit_focused(source);
        }

        // Hit-test blocks.
        if let Some(idx) = self.hit_test_block(x, y) {
            self.focus_block(idx, source);
            return true;
        }

        false
    }

    /// Handle mouse drag (only relevant when a focused block is active).
    pub fn handle_drag(&mut self, x: f32, y: f32) -> bool {
        if let Some(ref mut focused) = self.focused {
            return focused.editor.handle_mouse_drag(x, y);
        }
        false
    }

    /// Handle mouse release.
    pub fn handle_release(&mut self) {
        if let Some(ref mut focused) = self.focused {
            focused.editor.handle_mouse_release();
        }
    }

    /// Update blink timer for the focused block's caret.
    pub fn update_blink(&mut self, dt: f32) {
        if let Some(ref mut focused) = self.focused {
            focused.editor.update_blink(dt);
        }
    }

    /// Find which block index the point (x, y) falls in.
    fn hit_test_block(&self, _x: f32, y: f32) -> Option<usize> {
        for geo in &self.block_geos {
            if y >= geo.screen_y && y < geo.screen_y + geo.height {
                return Some(geo.block_index);
            }
        }
        None
    }

    /// Focus a block: extract its raw source and create a FocusedBlock.
    fn focus_block(&mut self, block_index: usize, source: &str) {
        let Some(range) = self.source_map.range_of(block_index) else {
            return;
        };
        let raw = &source[range.clone()];

        // Use the block's current screen geometry for positioning.
        let geo = self
            .block_geos
            .iter()
            .find(|g| g.block_index == block_index);
        let (sx, sy, sw) = if let Some(g) = geo {
            let content_x = MD_PAD_X; // relative to rect.x, but we use absolutes from geos
            // Reconstruct from stored screen_y. Width uses the same max_width.
            (content_x, g.screen_y, 0.0)
        } else {
            (MD_PAD_X, 0.0, 0.0)
        };
        // We'll use actual values from the geometry; update_rect will fix it on next render.
        let _ = (sx, sy, sw);
        let placeholder_rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 200.0,
        };

        let is_list = self.md_doc.blocks.get(block_index).is_some_and(|b| {
            matches!(
                b.kind,
                crate::markdown::model::BlockKind::OrderedList { .. }
                    | crate::markdown::model::BlockKind::UnorderedList { .. }
            )
        });

        self.focused = Some(FocusedBlock::new(
            block_index,
            range,
            raw,
            placeholder_rect,
            is_list,
        ));
    }

    /// Commit the focused block's edits back into the source string.
    pub fn commit_focused(&mut self, source: &mut String) {
        let Some(focused) = self.focused.take() else {
            return;
        };
        let new_text = focused.committed_text().to_string();
        let old_len = focused.source_range.len();
        let byte_delta = new_text.len() as isize - old_len as isize;
        let center = focused.block_index;

        source.replace_range(focused.source_range.clone(), &new_text);

        // Incremental reparse of the neighborhood.
        self.reparse_neighborhood(source, center, byte_delta);
    }

    /// Move focus to the previous block. Returns true if focus moved.
    pub fn focus_prev_block(&mut self, source: &mut String) -> bool {
        let current = match &self.focused {
            Some(f) => f.block_index,
            None => return false,
        };
        if current == 0 {
            return false;
        }
        self.commit_focused(source);
        self.focus_block_at_end(current - 1, source);
        true
    }

    /// Move focus to the next block. Returns true if focus moved.
    pub fn focus_next_block(&mut self, source: &mut String) -> bool {
        let current = match &self.focused {
            Some(f) => f.block_index,
            None => return false,
        };
        if current + 1 >= self.md_doc.blocks.len() {
            return false;
        }
        self.commit_focused(source);
        let next = (current + 1).min(self.md_doc.blocks.len() - 1);
        self.focus_block(next, source);
        true
    }

    /// Focus a block with cursor placed at the end (used when navigating down→up).
    fn focus_block_at_end(&mut self, block_index: usize, source: &str) {
        let Some(range) = self.source_map.range_of(block_index) else {
            return;
        };
        let raw = &source[range.clone()];
        let placeholder_rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 200.0,
        };
        let is_list = self.md_doc.blocks.get(block_index).is_some_and(|b| {
            matches!(
                b.kind,
                crate::markdown::model::BlockKind::OrderedList { .. }
                    | crate::markdown::model::BlockKind::UnorderedList { .. }
            )
        });
        let mut fb = FocusedBlock::new(block_index, range, raw, placeholder_rect, is_list);
        // Place cursor at end of last line.
        fb.editor.cursor_pos = raw.len();
        fb.editor.selection = jag_text::layout::Selection::collapsed(raw.len());
        self.focused = Some(fb);
    }

    /// Enter at end of focused block: commit, insert a blank line + empty
    /// paragraph in the source, re-parse, and focus the new block.
    pub fn enter_at_end_of_block(&mut self, source: &mut String) -> bool {
        let block_index = match &self.focused {
            Some(f) if f.is_at_end() => f.block_index,
            _ => return false,
        };

        // Commit current block edits first (uses incremental reparse).
        self.commit_focused(source);

        // After commit, block_index may be out of bounds if the edit changed structure.
        if block_index >= self.md_doc.blocks.len() {
            return false;
        }

        // Insert "\n\n" after the current block's source range to create a new paragraph.
        let insert_pos = self
            .source_map
            .range_of(block_index)
            .map(|r| r.end)
            .unwrap_or(source.len());

        source.insert_str(insert_pos, "\n\n");

        // Incremental reparse: +2 bytes from "\n\n".
        self.reparse_neighborhood(source, block_index, 2);

        // Focus the new block (the one after the current).
        let new_index = block_index + 1;
        if new_index < self.md_doc.blocks.len() {
            self.focus_block(new_index, source);
        }

        true
    }

    /// Backspace at start of focused block: merge with the previous block.
    pub fn backspace_merge_prev(&mut self, source: &mut String) -> bool {
        let block_index = match &self.focused {
            Some(f) if f.is_at_start() && f.block_index > 0 => f.block_index,
            _ => return false,
        };

        // Commit current block edits (uses incremental reparse).
        self.commit_focused(source);

        // After commit, recheck bounds.
        if block_index == 0 || block_index >= self.md_doc.blocks.len() {
            return false;
        }

        // Find the gap between previous block's end and this block's start.
        let prev_range = self.source_map.range_of(block_index - 1);
        let curr_range = self.source_map.range_of(block_index);

        if let (Some(prev), Some(curr)) = (prev_range, curr_range) {
            // Remove the whitespace/newlines between the two blocks,
            // replacing with a single newline to merge them.
            if prev.end < curr.start {
                let gap_len = curr.start - prev.end;
                let byte_delta = 1isize - gap_len as isize;
                source.replace_range(prev.end..curr.start, "\n");

                // Incremental reparse centered on the merge point.
                self.reparse_neighborhood(source, block_index - 1, byte_delta);
            }
        } else {
            self.full_reparse(source);
        }

        // Focus the previous block with cursor at end.
        let target = block_index - 1;
        if target < self.md_doc.blocks.len() {
            self.focus_block_at_end(target, source);
        }

        true
    }

    /// Clamp scroll_y to valid range.
    pub fn clamp_scroll(&mut self, viewport_h: f32) {
        let max_y = (self.total_height - viewport_h).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, max_y);
    }

    // ------------------------------------------------------------------
    // Reparse helpers
    // ------------------------------------------------------------------

    /// Full reparse of the entire source text.
    fn full_reparse(&mut self, source: &str) {
        self.md_doc = parse(source);
        self.source_map = SourceMap::from_document(&self.md_doc);
        self.last_parsed_text = source.to_string();
    }

    /// Incremental reparse: re-parse only ±1 blocks around `center_block`,
    /// splice results back, and shift byte offsets for subsequent blocks.
    ///
    /// `byte_delta` is the signed change in source length caused by the edit
    /// (positive = insertion, negative = deletion).
    ///
    /// Falls back to `full_reparse` if bounds are invalid.
    fn reparse_neighborhood(&mut self, source: &str, center_block: usize, byte_delta: isize) {
        let old_count = self.md_doc.blocks.len();
        if old_count == 0 {
            self.full_reparse(source);
            return;
        }

        let center = center_block.min(old_count - 1);
        let replace_start = center.saturating_sub(1);
        let replace_end = (center + 2).min(old_count);

        // Byte range in the NEW source for the reparse window.
        let region_byte_start = self.md_doc.blocks[replace_start].source_range.start;
        let region_byte_end = if replace_end < old_count {
            let old_next_start = self.md_doc.blocks[replace_end].source_range.start;
            let new_next_start = old_next_start as isize + byte_delta;
            if new_next_start < 0 {
                self.full_reparse(source);
                return;
            }
            new_next_start as usize
        } else {
            source.len()
        };

        // Validate bounds.
        if region_byte_start > source.len()
            || region_byte_end > source.len()
            || region_byte_start > region_byte_end
        {
            self.full_reparse(source);
            return;
        }

        // Parse the sub-region.
        let region_doc = parse(&source[region_byte_start..region_byte_end]);
        let new_count = region_doc.blocks.len();

        // Offset region-relative ranges to absolute document positions.
        let mut new_blocks = region_doc.blocks;
        for block in &mut new_blocks {
            offset_block_ranges(block, region_byte_start);
        }

        // Splice into the document.
        self.md_doc
            .blocks
            .splice(replace_start..replace_end, new_blocks);

        // Shift remaining blocks after the splice window by byte_delta.
        if byte_delta != 0 {
            let shift_from = replace_start + new_count;
            for block in &mut self.md_doc.blocks[shift_from..] {
                shift_block_ranges(block, byte_delta);
            }
        }

        self.source_map = SourceMap::from_document(&self.md_doc);
        self.last_parsed_text = source.to_string();
    }
}

// ---------------------------------------------------------------------------
// Block range adjustment helpers
// ---------------------------------------------------------------------------

/// Offset a block's source_range (and all nested children) by `offset` bytes.
/// Used to convert region-relative ranges to absolute document positions.
fn offset_block_ranges(block: &mut Block, offset: usize) {
    block.source_range = (block.source_range.start + offset)..(block.source_range.end + offset);
    match &mut block.kind {
        BlockKind::BlockQuote { children } => {
            for child in children {
                offset_block_ranges(child, offset);
            }
        }
        BlockKind::OrderedList { items, .. } | BlockKind::UnorderedList { items, .. } => {
            for item in items {
                for child in &mut item.children {
                    offset_block_ranges(child, offset);
                }
            }
        }
        _ => {}
    }
}

/// Shift a block's source_range (and all nested children) by `delta` bytes.
/// Used to adjust blocks after an edit point.
fn shift_block_ranges(block: &mut Block, delta: isize) {
    let start = (block.source_range.start as isize + delta).max(0) as usize;
    let end = (block.source_range.end as isize + delta).max(0) as usize;
    block.source_range = start..end;
    match &mut block.kind {
        BlockKind::BlockQuote { children } => {
            for child in children {
                shift_block_ranges(child, delta);
            }
        }
        BlockKind::OrderedList { items, .. } | BlockKind::UnorderedList { items, .. } => {
            for item in items {
                for child in &mut item.children {
                    shift_block_ranges(child, delta);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "reparse_tests.rs"]
mod reparse_tests;
