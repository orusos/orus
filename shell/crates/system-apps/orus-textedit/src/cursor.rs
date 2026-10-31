//! Cursor movement and selection extension.

use jag_text::layout::Selection as RtSelection;

use crate::editor::EditorBuffer;

impl EditorBuffer {
    // --- Cursor movement ---

    pub fn move_cursor_left(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            if !self.selection.is_collapsed() {
                let pos = self.selection.range().start;
                self.cursor_pos = pos;
                self.selection = RtSelection::collapsed(pos);
            } else {
                self.cursor_pos = layout.move_cursor_left(self.cursor_pos);
                self.selection = RtSelection::collapsed(self.cursor_pos);
            }
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            if !self.selection.is_collapsed() {
                let pos = self.selection.range().end;
                self.cursor_pos = pos;
                self.selection = RtSelection::collapsed(pos);
            } else {
                self.cursor_pos = layout.move_cursor_right(self.cursor_pos);
                self.selection = RtSelection::collapsed(self.cursor_pos);
            }
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_cursor_up(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            let (pos, px) = layout.move_cursor_up(self.cursor_pos, self.preferred_x);
            self.cursor_pos = pos;
            self.selection = RtSelection::collapsed(pos);
            self.preferred_x = Some(px);
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_cursor_down(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            let (pos, px) = layout.move_cursor_down(self.cursor_pos, self.preferred_x);
            self.cursor_pos = pos;
            self.selection = RtSelection::collapsed(pos);
            self.preferred_x = Some(px);
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_cursor_home(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.cursor_pos = layout.move_cursor_line_start(self.cursor_pos);
            self.selection = RtSelection::collapsed(self.cursor_pos);
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_cursor_end(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.cursor_pos = layout.move_cursor_line_end(self.cursor_pos);
            self.selection = RtSelection::collapsed(self.cursor_pos);
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_cursor_word_left(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.cursor_pos = layout.move_cursor_left_word(self.cursor_pos);
            self.selection = RtSelection::collapsed(self.cursor_pos);
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_cursor_word_right(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.cursor_pos = layout.move_cursor_right_word(self.cursor_pos);
            self.selection = RtSelection::collapsed(self.cursor_pos);
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_to_document_start(&mut self) {
        self.cursor_pos = 0;
        self.selection = RtSelection::collapsed(0);
        self.preferred_x = None;
        self.caret.reset_manual();
        self.ensure_cursor_visible();
    }

    pub fn move_to_document_end(&mut self) {
        self.cursor_pos = self.text.len();
        self.selection = RtSelection::collapsed(self.cursor_pos);
        self.preferred_x = None;
        self.caret.reset_manual();
        self.ensure_cursor_visible();
    }

    // --- Selection extension ---

    pub fn extend_selection_left(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.selection =
                layout.extend_selection(&self.selection, |offset| layout.move_cursor_left(offset));
            self.cursor_pos = self.selection.active();
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn extend_selection_right(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.selection =
                layout.extend_selection(&self.selection, |offset| layout.move_cursor_right(offset));
            self.cursor_pos = self.selection.active();
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn extend_selection_up(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            let (sel, px) = layout.extend_selection_vertical(
                &self.selection,
                |offset, pref| layout.move_cursor_up(offset, pref),
                self.preferred_x,
            );
            self.selection = sel;
            self.cursor_pos = self.selection.active();
            self.preferred_x = Some(px);
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn extend_selection_down(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            let (sel, px) = layout.extend_selection_vertical(
                &self.selection,
                |offset, pref| layout.move_cursor_down(offset, pref),
                self.preferred_x,
            );
            self.selection = sel;
            self.cursor_pos = self.selection.active();
            self.preferred_x = Some(px);
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn extend_selection_home(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.selection = layout.extend_selection(&self.selection, |offset| {
                layout.move_cursor_line_start(offset)
            });
            self.cursor_pos = self.selection.active();
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn extend_selection_end(&mut self) {
        if let Some(layout) = self.layout.as_ref() {
            self.selection = layout.extend_selection(&self.selection, |offset| {
                layout.move_cursor_line_end(offset)
            });
            self.cursor_pos = self.selection.active();
            self.preferred_x = None;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
        }
    }

    pub fn select_all(&mut self) {
        self.selection = RtSelection::new(0, self.text.len());
        self.cursor_pos = self.text.len();
        self.caret.reset_manual();
    }
}
