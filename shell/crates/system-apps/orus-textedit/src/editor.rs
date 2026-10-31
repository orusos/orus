use jag_draw::Rect;
use jag_text::font::load_system_default_font;
use jag_text::layout::{
    CursorPosition, Point as RtPoint, Selection as RtSelection, TextLayout as RtTextLayout,
    WrapMode,
};

pub(crate) const TEXT_SIZE: f32 = 13.0;
pub(crate) const LINE_HEIGHT_FACTOR: f32 = 1.4;
const SCROLL_MARGIN: f32 = 12.0;

// Gutter (line numbers).
pub(crate) const GUTTER_TEXT_SIZE: f32 = 12.0;
pub(crate) const GUTTER_LEFT_PAD: f32 = 10.0;
pub(crate) const GUTTER_RIGHT_PAD: f32 = 16.0;
const GUTTER_DIGIT_WIDTH: f32 = 8.0;

// Content area padding (after gutter, before scrollbar).
pub(crate) const CONTENT_PAD_LEFT: f32 = 14.0;
pub(crate) const CONTENT_PAD_RIGHT: f32 = 14.0;
pub(crate) const CONTENT_PAD_TOP: f32 = 4.0;

// Scrollbar.
pub(crate) const SCROLLBAR_WIDTH: f32 = 8.0;
pub(crate) const SCROLLBAR_MIN_THUMB: f32 = 24.0;

// Caret blink state (inlined from detir-scene to avoid heavy dependency).
#[derive(Clone, Copy, Debug)]
pub(crate) struct CaretBlink {
    pub visible: bool,
    blink_time: f32,
    blink_interval: f32,
    focused: bool,
    was_focused: bool,
}

impl CaretBlink {
    pub fn new(initial_focused: bool) -> Self {
        Self {
            visible: true,
            blink_time: 0.0,
            blink_interval: 0.5,
            focused: initial_focused,
            was_focused: initial_focused,
        }
    }

    pub fn update(&mut self, delta_time: f32, focused: bool) {
        self.focused = focused;
        if self.focused && !self.was_focused {
            self.visible = true;
            self.blink_time = 0.0;
        } else if !self.focused && self.was_focused {
            self.visible = false;
            self.blink_time = 0.0;
        }
        self.was_focused = self.focused;

        if self.focused {
            self.blink_time += delta_time;
            if self.blink_time >= self.blink_interval {
                self.blink_time -= self.blink_interval;
                self.visible = !self.visible;
            }
        }
    }

    pub fn reset_manual(&mut self) {
        self.visible = true;
        self.blink_time = 0.0;
    }
}

/// The core text editor buffer with layout, cursor, selection, and scrolling.
pub struct EditorBuffer {
    // Content
    pub text: String,
    pub(crate) layout: Option<RtTextLayout>,
    pub(crate) selection: RtSelection,
    pub(crate) cursor_pos: usize,

    // Visual state
    pub(crate) rect: Rect,
    pub(crate) scroll_x: f32,
    pub(crate) scroll_y: f32,
    pub(crate) caret: CaretBlink,
    pub focused: bool,
    pub(crate) preferred_x: Option<f32>,

    // File metadata
    pub file_path: Option<String>,
    pub dirty: bool,

    // Mouse state
    pub(crate) mouse_selecting: bool,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            layout: None,
            selection: RtSelection::collapsed(0),
            cursor_pos: 0,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 600.0,
                h: 400.0,
            },
            scroll_x: 0.0,
            scroll_y: 0.0,
            caret: CaretBlink::new(true),
            focused: true,
            preferred_x: None,
            file_path: None,
            dirty: false,
            mouse_selecting: false,
        }
    }

    /// Load text content and rebuild layout.
    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cursor_pos = 0;
        self.selection = RtSelection::collapsed(0);
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.dirty = false;
        self.rebuild_layout();
    }

    pub fn rebuild_layout(&mut self) {
        if let Ok(font) = load_system_default_font() {
            let line_height = TEXT_SIZE * LINE_HEIGHT_FACTOR;
            let max_width = self.content_rect().w.max(100.0);
            self.layout = Some(RtTextLayout::with_wrap_and_line_height(
                self.text.clone(),
                &font,
                TEXT_SIZE,
                Some(max_width),
                WrapMode::BreakWord,
                line_height,
            ));
        }
    }

    /// Update wrap width when the window resizes.
    pub fn update_viewport(&mut self, w: f32, h: f32) {
        let old_w = self.rect.w;
        self.rect.w = w;
        self.rect.h = h;
        if (old_w - w).abs() > 1.0 {
            self.rebuild_layout();
        }
    }

    // --- Gutter ---

    /// Gutter width based on logical line count (newlines in text).
    pub(crate) fn gutter_width(&self) -> f32 {
        let logical_lines = self.text.bytes().filter(|&b| b == b'\n').count() + 1;
        let digits = format!("{logical_lines}").len().max(3);
        GUTTER_LEFT_PAD + digits as f32 * GUTTER_DIGIT_WIDTH + GUTTER_RIGHT_PAD
    }

    // --- Metrics ---

    pub(crate) fn line_height(&self) -> f32 {
        self.layout
            .as_ref()
            .and_then(|l| l.lines().first().map(|line| line.height))
            .unwrap_or(TEXT_SIZE * LINE_HEIGHT_FACTOR)
    }

    pub(crate) fn line_count(&self) -> usize {
        self.layout.as_ref().map(|l| l.lines().len()).unwrap_or(1)
    }

    pub(crate) fn current_line(&self) -> usize {
        let Some(layout) = self.layout.as_ref() else {
            return 0;
        };
        for (i, line) in layout.lines().iter().enumerate() {
            if self.cursor_pos <= line.text_range.end {
                return i;
            }
        }
        self.line_count().saturating_sub(1)
    }

    pub(crate) fn total_height(&self) -> f32 {
        self.line_count() as f32 * self.line_height()
    }

    /// Content rect: the area where text is drawn (after gutter, with padding).
    pub(crate) fn content_rect(&self) -> Rect {
        let gw = self.gutter_width();
        Rect {
            x: self.rect.x + gw + CONTENT_PAD_LEFT,
            y: self.rect.y + CONTENT_PAD_TOP,
            w: (self.rect.w - gw - CONTENT_PAD_LEFT - CONTENT_PAD_RIGHT - SCROLLBAR_WIDTH)
                .max(10.0),
            h: (self.rect.h - CONTENT_PAD_TOP * 2.0).max(10.0),
        }
    }

    pub(crate) fn screen_to_layout(&self, screen_x: f32, screen_y: f32) -> RtPoint {
        let cr = self.content_rect();
        RtPoint {
            x: screen_x - cr.x + self.scroll_x,
            y: screen_y - cr.y + self.scroll_y,
        }
    }

    // --- Scrolling ---

    pub fn ensure_cursor_visible(&mut self) {
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let cr = self.content_rect();
        let line_h = self.line_height();
        let cur_line = self.current_line();
        let line_top = cur_line as f32 * line_h;
        let line_bottom = line_top + line_h;

        if line_top < self.scroll_y + SCROLL_MARGIN {
            self.scroll_y = (line_top - SCROLL_MARGIN).max(0.0);
        }
        if line_bottom > self.scroll_y + cr.h - SCROLL_MARGIN {
            self.scroll_y = line_bottom - cr.h + SCROLL_MARGIN;
        }

        let cursor_pos = CursorPosition::new(self.cursor_pos.min(layout.text().len()));
        if let Some(cursor_rect) = layout.cursor_rect_at_position(cursor_pos) {
            let cx = cursor_rect.x;
            if cx < self.scroll_x + SCROLL_MARGIN {
                self.scroll_x = (cx - SCROLL_MARGIN).max(0.0);
            }
            if cx > self.scroll_x + cr.w - SCROLL_MARGIN {
                self.scroll_x = cx - cr.w + SCROLL_MARGIN;
            }
        }

        let max_y = (self.total_height() - cr.h).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, max_y);
        self.scroll_x = self.scroll_x.clamp(0.0, 0.0); // word-wrap: no h-scroll
    }

    pub fn handle_scroll(&mut self, _dx: f32, dy: f32) -> bool {
        let cr = self.content_rect();
        self.scroll_y -= dy;
        let max_y = (self.total_height() - cr.h).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, max_y);
        true
    }

    // --- Mouse handling ---

    pub fn handle_mouse_click(&mut self, x: f32, y: f32, double: bool) -> bool {
        let cr = self.content_rect();
        if x < cr.x || x > cr.x + cr.w || y < cr.y || y > cr.y + cr.h {
            return false;
        }

        let point = self.screen_to_layout(x, y);
        if let Some(layout) = self.layout.as_ref() {
            if double {
                if let Some(sel) = layout.start_word_selection(point) {
                    self.selection = sel;
                    self.cursor_pos = self.selection.active();
                }
            } else if let Some(sel) = layout.start_mouse_selection(point) {
                self.selection = sel;
                self.cursor_pos = self.selection.active();
            }
            self.mouse_selecting = !double;
            self.caret.reset_manual();
            self.ensure_cursor_visible();
            return true;
        }
        false
    }

    pub fn handle_mouse_drag(&mut self, x: f32, y: f32) -> bool {
        if !self.mouse_selecting {
            return false;
        }
        let point = self.screen_to_layout(x, y);
        if let Some(layout) = self.layout.as_ref() {
            self.selection = layout.extend_mouse_selection(&self.selection, point);
            self.cursor_pos = self.selection.active();
            self.caret.reset_manual();
            self.ensure_cursor_visible();
            return true;
        }
        false
    }

    pub fn handle_mouse_release(&mut self) {
        self.mouse_selecting = false;
    }

    pub fn update_blink(&mut self, dt: f32) {
        self.caret.update(dt, self.focused);
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
        self.caret.reset_manual();
        self.preferred_x = None;
    }

    // --- Info for title bar / status bar ---

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let line = self.current_line();
        let col = if let Some(layout) = self.layout.as_ref() {
            if let Some(lbox) = layout.lines().get(line) {
                self.cursor_pos.saturating_sub(lbox.text_range.start)
            } else {
                0
            }
        } else {
            0
        };
        (line + 1, col + 1)
    }

    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }
}
