//! Keyboard input routing for all editor modes.

use crate::app::{EditorMode, TextEditApp};
use crate::wysiwyg::WysiwygState;

impl TextEditApp {
    /// Handle keyboard input. Returns true if consumed.
    pub fn handle_key(
        &mut self,
        key: &winit::keyboard::Key,
        modifiers: &winit::event::Modifiers,
    ) -> bool {
        use winit::keyboard::Key;

        let cmd = modifiers.state().super_key();
        let shift = modifiers.state().shift_key();
        let alt = modifiers.state().alt_key();

        // Global shortcuts
        if cmd {
            match key {
                // Cmd+Shift+M: toggle source/wysiwyg mode
                Key::Character(ch) if ch.as_str().eq_ignore_ascii_case("m") && shift => {
                    self.toggle_mode();
                    return true;
                }
                Key::Character(ch) if ch.as_str() == "s" => {
                    // Commit focused block before saving so edits are persisted.
                    if let EditorMode::Wysiwyg(ws) = &mut self.mode {
                        ws.commit_focused(&mut self.editor.text);
                    }
                    if shift {
                        self.editor.save_as_dialog();
                    } else {
                        self.editor.save_or_save_as();
                    }
                    return true;
                }
                Key::Character(ch) if ch.as_str() == "o" => {
                    self.editor.open_file_dialog();
                    let is_md = self
                        .editor
                        .file_path
                        .as_ref()
                        .is_some_and(|p| p.ends_with(".md") || p.ends_with(".markdown"));
                    self.mode = if is_md {
                        EditorMode::Wysiwyg(Box::new(WysiwygState::new(&self.editor.text)))
                    } else {
                        EditorMode::Source
                    };
                    return true;
                }
                Key::Character(ch) if ch.as_str() == "n" => {
                    self.editor.new_file();
                    self.mode = EditorMode::Source;
                    return true;
                }
                _ => {}
            }
        }

        // Route based on mode.
        let is_wysiwyg = matches!(&self.mode, EditorMode::Wysiwyg(_));
        if is_wysiwyg {
            self.handle_key_wysiwyg(key, cmd, shift, alt)
        } else {
            self.handle_key_source(key, cmd, shift, alt)
        }
    }

    pub(crate) fn toggle_mode(&mut self) {
        match &mut self.mode {
            EditorMode::Source => {
                self.mode = EditorMode::Wysiwyg(Box::new(WysiwygState::new(&self.editor.text)));
            }
            EditorMode::Wysiwyg(ws) => {
                ws.commit_focused(&mut self.editor.text);
                self.editor.rebuild_layout();
                self.mode = EditorMode::Source;
            }
        }
    }

    /// Key handling in WYSIWYG mode.
    fn handle_key_wysiwyg(
        &mut self,
        key: &winit::keyboard::Key,
        cmd: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        use winit::keyboard::{Key, NamedKey};

        let EditorMode::Wysiwyg(ws) = &mut self.mode else {
            return false;
        };

        // With a focused block: route editing keys to the focused editor.
        if ws.focused.is_some() {
            // Cross-block navigation: ArrowUp at first line → previous block,
            // ArrowDown at last line → next block.
            if !cmd && !shift && !alt {
                if matches!(key, Key::Named(NamedKey::ArrowUp))
                    && ws.focused.as_ref().is_some_and(|f| f.is_at_first_line())
                {
                    ws.focus_prev_block(&mut self.editor.text);
                    return true;
                }
                if matches!(key, Key::Named(NamedKey::ArrowDown))
                    && ws.focused.as_ref().is_some_and(|f| f.is_at_last_line())
                {
                    ws.focus_next_block(&mut self.editor.text);
                    return true;
                }
            }

            // Escape: unfocus (commit and return to rich view).
            if matches!(key, Key::Named(NamedKey::Escape)) {
                ws.commit_focused(&mut self.editor.text);
                return true;
            }

            // Enter at end of block → create new paragraph below.
            if matches!(key, Key::Named(NamedKey::Enter))
                && !cmd
                && !shift
                && ws.focused.as_ref().is_some_and(|f| f.is_at_end())
            {
                ws.enter_at_end_of_block(&mut self.editor.text);
                return true;
            }

            // Backspace at start of block → merge with previous block.
            if matches!(key, Key::Named(NamedKey::Backspace))
                && !cmd
                && !alt
                && ws.focused.as_ref().is_some_and(|f| f.is_at_start())
                && ws.backspace_merge_prev(&mut self.editor.text)
            {
                return true;
            }

            let is_list = ws.focused.as_ref().unwrap().is_list;
            let editor = &mut ws.focused.as_mut().unwrap().editor;

            if cmd {
                match key {
                    Key::Character(ch) if ch.as_str() == "a" => {
                        editor.select_all();
                        return true;
                    }
                    Key::Character(ch) if ch.as_str() == "c" => {
                        editor.copy();
                        return true;
                    }
                    Key::Character(ch) if ch.as_str() == "x" => {
                        editor.cut();
                        return true;
                    }
                    Key::Character(ch) if ch.as_str() == "v" => {
                        editor.paste();
                        return true;
                    }
                    Key::Character(ch) if ch.as_str() == "z" => {
                        if shift {
                            editor.redo();
                        } else {
                            editor.undo();
                        }
                        return true;
                    }
                    _ => {}
                }
            }

            match key {
                Key::Named(NamedKey::ArrowLeft) => {
                    dispatch_arrow_left(editor, cmd, shift, alt);
                    return true;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    dispatch_arrow_right(editor, cmd, shift, alt);
                    return true;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    dispatch_arrow_up(editor, cmd, shift);
                    return true;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    dispatch_arrow_down(editor, cmd, shift);
                    return true;
                }
                Key::Named(NamedKey::Home) => {
                    dispatch_home(editor, shift);
                    return true;
                }
                Key::Named(NamedKey::End) => {
                    dispatch_end(editor, shift);
                    return true;
                }
                _ => {}
            }

            match key {
                Key::Named(NamedKey::Enter) => {
                    editor.insert_newline();
                    return true;
                }
                Key::Named(NamedKey::Backspace) => {
                    dispatch_backspace(editor, alt);
                    return true;
                }
                Key::Named(NamedKey::Delete) => {
                    editor.delete_forward();
                    return true;
                }
                Key::Named(NamedKey::Tab) => {
                    // Smart Tab: in list blocks, indent/un-indent the line.
                    if is_list {
                        if shift {
                            editor.unindent_line();
                        } else {
                            editor.indent_line();
                        }
                    } else {
                        editor.insert_tab();
                    }
                    return true;
                }
                Key::Named(NamedKey::Space) => {
                    editor.insert_char(' ');
                    return true;
                }
                Key::Character(ch) => {
                    if !cmd {
                        for c in ch.chars() {
                            editor.insert_char(c);
                        }
                        return true;
                    }
                }
                _ => {}
            }

            return false;
        }

        // No focused block: scroll keys only.
        match key {
            Key::Named(NamedKey::ArrowDown) => {
                self.handle_scroll(-40.0);
                true
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.handle_scroll(40.0);
                true
            }
            Key::Named(NamedKey::PageDown) => {
                self.handle_scroll(-self.viewport_h * 0.8);
                true
            }
            Key::Named(NamedKey::PageUp) => {
                self.handle_scroll(self.viewport_h * 0.8);
                true
            }
            Key::Named(NamedKey::Home) if cmd => {
                if let EditorMode::Wysiwyg(ws) = &mut self.mode {
                    ws.scroll_y = 0.0;
                }
                true
            }
            Key::Named(NamedKey::End) if cmd => {
                if let EditorMode::Wysiwyg(ws) = &mut self.mode {
                    ws.scroll_y = (ws.total_height - self.viewport_h).max(0.0);
                }
                true
            }
            _ => false,
        }
    }

    /// Key handling in Source mode.
    fn handle_key_source(
        &mut self,
        key: &winit::keyboard::Key,
        cmd: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        use winit::keyboard::{Key, NamedKey};

        if cmd {
            match key {
                Key::Character(ch) if ch.as_str() == "a" => {
                    self.editor.select_all();
                    return true;
                }
                Key::Character(ch) if ch.as_str() == "c" => {
                    self.editor.copy();
                    return true;
                }
                Key::Character(ch) if ch.as_str() == "x" => {
                    self.editor.cut();
                    return true;
                }
                Key::Character(ch) if ch.as_str() == "v" => {
                    self.editor.paste();
                    return true;
                }
                Key::Character(ch) if ch.as_str() == "z" => {
                    if shift {
                        self.editor.redo();
                    } else {
                        self.editor.undo();
                    }
                    return true;
                }
                _ => {}
            }
        }

        match key {
            Key::Named(NamedKey::ArrowLeft) => {
                dispatch_arrow_left(&mut self.editor, cmd, shift, alt);
                return true;
            }
            Key::Named(NamedKey::ArrowRight) => {
                dispatch_arrow_right(&mut self.editor, cmd, shift, alt);
                return true;
            }
            Key::Named(NamedKey::ArrowUp) => {
                dispatch_arrow_up(&mut self.editor, cmd, shift);
                return true;
            }
            Key::Named(NamedKey::ArrowDown) => {
                dispatch_arrow_down(&mut self.editor, cmd, shift);
                return true;
            }
            Key::Named(NamedKey::Home) => {
                dispatch_home(&mut self.editor, shift);
                return true;
            }
            Key::Named(NamedKey::End) => {
                dispatch_end(&mut self.editor, shift);
                return true;
            }
            _ => {}
        }

        match key {
            Key::Named(NamedKey::Enter) => {
                self.editor.insert_newline();
                return true;
            }
            Key::Named(NamedKey::Backspace) => {
                dispatch_backspace(&mut self.editor, alt);
                return true;
            }
            Key::Named(NamedKey::Delete) => {
                self.editor.delete_forward();
                return true;
            }
            Key::Named(NamedKey::Tab) => {
                self.editor.insert_tab();
                return true;
            }
            Key::Named(NamedKey::Space) => {
                self.editor.insert_char(' ');
                return true;
            }
            Key::Character(ch) => {
                if !cmd {
                    for c in ch.chars() {
                        self.editor.insert_char(c);
                    }
                    return true;
                }
            }
            _ => {}
        }

        false
    }
}

// --- Shared key dispatch helpers ---

use crate::editor::EditorBuffer;

fn dispatch_arrow_left(editor: &mut EditorBuffer, cmd: bool, shift: bool, alt: bool) {
    if cmd && shift {
        editor.extend_selection_home();
    } else if cmd {
        editor.move_cursor_home();
    } else if shift {
        editor.extend_selection_left();
    } else if alt {
        editor.move_cursor_word_left();
    } else {
        editor.move_cursor_left();
    }
}

fn dispatch_arrow_right(editor: &mut EditorBuffer, cmd: bool, shift: bool, alt: bool) {
    if cmd && shift {
        editor.extend_selection_end();
    } else if cmd {
        editor.move_cursor_end();
    } else if shift {
        editor.extend_selection_right();
    } else if alt {
        editor.move_cursor_word_right();
    } else {
        editor.move_cursor_right();
    }
}

fn dispatch_arrow_up(editor: &mut EditorBuffer, cmd: bool, shift: bool) {
    if cmd {
        editor.move_to_document_start();
    } else if shift {
        editor.extend_selection_up();
    } else {
        editor.move_cursor_up();
    }
}

fn dispatch_arrow_down(editor: &mut EditorBuffer, cmd: bool, shift: bool) {
    if cmd {
        editor.move_to_document_end();
    } else if shift {
        editor.extend_selection_down();
    } else {
        editor.move_cursor_down();
    }
}

fn dispatch_home(editor: &mut EditorBuffer, shift: bool) {
    if shift {
        editor.extend_selection_home();
    } else {
        editor.move_cursor_home();
    }
}

fn dispatch_end(editor: &mut EditorBuffer, shift: bool) {
    if shift {
        editor.extend_selection_end();
    } else {
        editor.move_cursor_end();
    }
}

fn dispatch_backspace(editor: &mut EditorBuffer, alt: bool) {
    if alt {
        editor.delete_word_backward();
    } else {
        editor.delete_backward();
    }
}
