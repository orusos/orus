//! Text editing operations: insert, delete, clipboard, undo/redo.

use jag_text::font::load_system_default_font;
use jag_text::layout::{Selection as RtSelection, WrapMode};

use crate::editor::{EditorBuffer, TEXT_SIZE};

impl EditorBuffer {
    fn wrap_width(&self) -> Option<f32> {
        Some(self.content_rect().w.max(100.0))
    }

    pub(crate) fn with_layout_edit<F>(&mut self, f: F)
    where
        F: FnOnce(
            &mut jag_text::layout::TextLayout,
            &jag_text::font::FontFace,
            &RtSelection,
        ) -> usize,
    {
        let font = match load_system_default_font() {
            Ok(f) => f,
            Err(_) => return,
        };
        if let Some(layout) = self.layout.as_mut() {
            let new_pos = f(layout, &font, &self.selection);
            self.text = layout.text().to_string();
            self.cursor_pos = new_pos;
            self.selection = RtSelection::collapsed(new_pos);
            self.mark_dirty();
            self.ensure_cursor_visible();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let max_w = self.wrap_width();
        self.with_layout_edit(|layout, font, sel| {
            if !sel.is_collapsed() {
                layout.replace_selection(
                    sel,
                    &ch.to_string(),
                    font,
                    TEXT_SIZE,
                    max_w,
                    WrapMode::BreakWord,
                )
            } else {
                layout.insert_char(
                    sel.active(),
                    ch,
                    font,
                    TEXT_SIZE,
                    max_w,
                    WrapMode::BreakWord,
                )
            }
        });
    }

    pub fn insert_newline(&mut self) {
        let max_w = self.wrap_width();
        self.with_layout_edit(|layout, font, sel| {
            if !sel.is_collapsed() {
                layout.replace_selection(sel, "\n", font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            } else {
                layout.insert_newline(sel.active(), font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            }
        });
    }

    pub fn insert_tab(&mut self) {
        let max_w = self.wrap_width();
        self.with_layout_edit(|layout, font, sel| {
            if !sel.is_collapsed() {
                layout.replace_selection(sel, "    ", font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            } else {
                layout.insert_str(
                    sel.active(),
                    "    ",
                    font,
                    TEXT_SIZE,
                    max_w,
                    WrapMode::BreakWord,
                )
            }
        });
    }

    /// Indent the current line by prepending 2 spaces at its start.
    /// Cursor position is adjusted to stay in the same relative position.
    pub fn indent_line(&mut self) {
        let line_start = self.text[..self.cursor_pos]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);

        self.text.insert_str(line_start, "  ");
        self.cursor_pos += 2;
        self.selection = RtSelection::collapsed(self.cursor_pos);
        self.rebuild_layout();
        self.mark_dirty();
        self.ensure_cursor_visible();
    }

    /// Un-indent the current line by removing up to 2 leading spaces.
    pub fn unindent_line(&mut self) {
        let line_start = self.text[..self.cursor_pos]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);

        let spaces = self.text[line_start..]
            .bytes()
            .take_while(|&b| b == b' ')
            .count()
            .min(2);

        if spaces > 0 {
            self.text.replace_range(line_start..line_start + spaces, "");
            self.cursor_pos = self.cursor_pos.saturating_sub(spaces);
            self.selection = RtSelection::collapsed(self.cursor_pos);
            self.rebuild_layout();
            self.mark_dirty();
            self.ensure_cursor_visible();
        }
    }

    pub fn delete_backward(&mut self) {
        let max_w = self.wrap_width();
        self.with_layout_edit(|layout, font, sel| {
            if !sel.is_collapsed() {
                layout.delete_selection(sel, font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            } else {
                layout.delete_backward(sel.active(), font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            }
        });
    }

    pub fn delete_forward(&mut self) {
        let max_w = self.wrap_width();
        self.with_layout_edit(|layout, font, sel| {
            if !sel.is_collapsed() {
                layout.delete_selection(sel, font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            } else {
                layout.delete_forward(sel.active(), font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            }
        });
    }

    pub fn delete_word_backward(&mut self) {
        let max_w = self.wrap_width();
        self.with_layout_edit(|layout, font, sel| {
            if !sel.is_collapsed() {
                layout.delete_selection(sel, font, TEXT_SIZE, max_w, WrapMode::BreakWord)
            } else {
                layout.delete_word_backward(
                    sel.active(),
                    font,
                    TEXT_SIZE,
                    max_w,
                    WrapMode::BreakWord,
                )
            }
        });
    }

    // --- Clipboard ---

    pub fn copy(&self) {
        if let Some(layout) = self.layout.as_ref() {
            let _ = layout.copy_to_clipboard(&self.selection);
        }
    }

    pub fn cut(&mut self) {
        let font = match load_system_default_font() {
            Ok(f) => f,
            Err(_) => return,
        };
        let max_w = self.wrap_width();
        if let Some(layout) = self.layout.as_mut()
            && let Ok(new_pos) = layout.cut_to_clipboard(
                &self.selection,
                &font,
                TEXT_SIZE,
                max_w,
                WrapMode::BreakWord,
            )
        {
            self.text = layout.text().to_string();
            self.cursor_pos = new_pos;
            self.selection = RtSelection::collapsed(new_pos);
            self.mark_dirty();
            self.ensure_cursor_visible();
        }
    }

    pub fn paste(&mut self) {
        let font = match load_system_default_font() {
            Ok(f) => f,
            Err(_) => return,
        };
        let max_w = self.wrap_width();
        if let Some(layout) = self.layout.as_mut() {
            let result = if !self.selection.is_collapsed() {
                layout.paste_replace_selection(
                    &self.selection,
                    &font,
                    TEXT_SIZE,
                    max_w,
                    WrapMode::BreakWord,
                )
            } else {
                layout.paste_from_clipboard(
                    self.cursor_pos,
                    &font,
                    TEXT_SIZE,
                    max_w,
                    WrapMode::BreakWord,
                )
            };
            if let Ok(new_pos) = result {
                self.text = layout.text().to_string();
                self.cursor_pos = new_pos;
                self.selection = RtSelection::collapsed(new_pos);
                self.mark_dirty();
                self.ensure_cursor_visible();
            }
        }
    }

    // --- Undo / Redo ---

    pub fn undo(&mut self) {
        let font = match load_system_default_font() {
            Ok(f) => f,
            Err(_) => return,
        };
        let max_w = self.wrap_width();
        if let Some(layout) = self.layout.as_mut()
            && let Some((new_pos, new_sel)) = layout.undo(
                &self.selection,
                &font,
                TEXT_SIZE,
                max_w,
                WrapMode::BreakWord,
            )
        {
            self.text = layout.text().to_string();
            self.cursor_pos = new_pos;
            self.selection = new_sel;
            self.mark_dirty();
            self.ensure_cursor_visible();
        }
    }

    pub fn redo(&mut self) {
        let font = match load_system_default_font() {
            Ok(f) => f,
            Err(_) => return,
        };
        let max_w = self.wrap_width();
        if let Some(layout) = self.layout.as_mut()
            && let Some((new_pos, new_sel)) = layout.redo(
                &self.selection,
                &font,
                TEXT_SIZE,
                max_w,
                WrapMode::BreakWord,
            )
        {
            self.text = layout.text().to_string();
            self.cursor_pos = new_pos;
            self.selection = new_sel;
            self.mark_dirty();
            self.ensure_cursor_visible();
        }
    }
}
