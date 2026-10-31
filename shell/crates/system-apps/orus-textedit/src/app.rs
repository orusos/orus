use std::sync::Arc;
use std::time::Instant;

use jag_draw::{Brush, Rect, SubpixelOrientation, TextProvider};
use jag_surface::Canvas;

use crate::editor::EditorBuffer;
use crate::render::render_editor;
use crate::theme::Theme;
use crate::wysiwyg::WysiwygState;

const STATUS_BAR_HEIGHT: f32 = 24.0;
const STATUS_TEXT_SIZE: f32 = 12.0;
const STATUS_PAD_X: f32 = 12.0;

/// The two top-level editing modes.
pub(crate) enum EditorMode {
    /// Full source editor (gutter, line numbers, scrollbar).
    Source,
    /// Rich rendered view with per-block inline editing.
    Wysiwyg(Box<WysiwygState>),
}

pub struct TextEditApp {
    pub editor: EditorBuffer,
    pub provider: Arc<dyn TextProvider + Send + Sync>,
    theme: Theme,
    last_frame: Instant,
    pub(crate) mode: EditorMode,
    pub(crate) viewport_h: f32,
    viewport_w: f32,
    /// When Some((ox, oy)), rendering is offset for embedded use.
    origin: Option<(f32, f32)>,
}

impl TextEditApp {
    pub fn new() -> anyhow::Result<Self> {
        let provider: Arc<dyn TextProvider + Send + Sync> = Arc::new(
            jag_draw::DetirTextProvider::from_system_fonts(SubpixelOrientation::RGB)?,
        );
        Self::new_with_provider(provider)
    }

    pub fn new_with_provider(
        provider: Arc<dyn TextProvider + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let mut editor = EditorBuffer::new();
        if let Some(path) = std::env::args().nth(1) {
            let _ = editor.open_file(&path);
        }

        let is_md = editor
            .file_path
            .as_ref()
            .is_some_and(|p| p.ends_with(".md") || p.ends_with(".markdown"));

        let mode = if is_md {
            EditorMode::Wysiwyg(Box::new(WysiwygState::new(&editor.text)))
        } else {
            EditorMode::Source
        };

        Ok(Self {
            editor,
            provider,
            theme: Theme::dark(),
            last_frame: Instant::now(),
            mode,
            viewport_h: 400.0,
            viewport_w: 900.0,
            origin: None,
        })
    }

    pub fn update_viewport(&mut self, w: f32, h: f32) {
        self.viewport_w = w;
        self.viewport_h = h - STATUS_BAR_HEIGHT;
        self.editor.update_viewport(w, h - STATUS_BAR_HEIGHT);
    }

    /// Set the rendering origin for embedded use (e.g. inside a compositor window).
    /// When set, `render` offsets all drawing by (ox, oy) and skips `canvas.clear()`.
    pub fn set_origin(&mut self, ox: f32, oy: f32) {
        self.origin = Some((ox, oy));
        self.editor.rect.x = ox;
        self.editor.rect.y = oy;
    }

    /// Clear the rendering origin (revert to standalone mode at 0,0).
    pub fn clear_origin(&mut self) {
        self.origin = None;
        self.editor.rect.x = 0.0;
        self.editor.rect.y = 0.0;
    }

    /// Handle scroll in the current mode.
    pub fn handle_scroll(&mut self, dy: f32) {
        match &mut self.mode {
            EditorMode::Wysiwyg(ws) => {
                ws.scroll_y -= dy;
                ws.clamp_scroll(self.viewport_h);
            }
            EditorMode::Source => {
                self.editor.handle_scroll(0.0, dy);
            }
        }
    }

    /// Returns true if the editor is in the middle of a mouse text selection.
    pub fn is_mouse_selecting(&self) -> bool {
        match &self.mode {
            EditorMode::Source => self.editor.mouse_selecting,
            EditorMode::Wysiwyg(ws) => ws
                .focused
                .as_ref()
                .is_some_and(|f| f.editor.mouse_selecting),
        }
    }

    // --- Mouse routing ---

    pub fn handle_mouse_click(&mut self, x: f32, y: f32, double: bool) -> bool {
        match &mut self.mode {
            EditorMode::Wysiwyg(ws) => ws.handle_click(x, y, double, &mut self.editor.text),
            EditorMode::Source => self.editor.handle_mouse_click(x, y, double),
        }
    }

    pub fn handle_mouse_drag(&mut self, x: f32, y: f32) -> bool {
        match &mut self.mode {
            EditorMode::Wysiwyg(ws) => ws.handle_drag(x, y),
            EditorMode::Source => self.editor.handle_mouse_drag(x, y),
        }
    }

    pub fn handle_mouse_release(&mut self) {
        match &mut self.mode {
            EditorMode::Wysiwyg(ws) => ws.handle_release(),
            EditorMode::Source => self.editor.handle_mouse_release(),
        }
    }

    // --- Rendering ---

    /// Render the editor. `z_base` is the base z-index for all draw calls
    /// (use 0 for standalone, or the window's z_base + offset when embedded).
    pub fn render(&mut self, canvas: &mut Canvas, width: f32, height: f32, z_base: i32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        let (ox, oy) = self.origin.unwrap_or((0.0, 0.0));

        // Update blink for active editor.
        match &mut self.mode {
            EditorMode::Wysiwyg(ws) => ws.update_blink(dt),
            EditorMode::Source => self.editor.update_blink(dt),
        }

        // Background
        if self.origin.is_none() {
            canvas.clear(self.theme.bg);
        }
        canvas.fill_rect(ox, oy, width, height, Brush::Solid(self.theme.bg), z_base);

        match &mut self.mode {
            EditorMode::Wysiwyg(ws) => {
                let content_rect = Rect {
                    x: ox,
                    y: oy,
                    w: width,
                    h: height - STATUS_BAR_HEIGHT,
                };
                let base_dir = self
                    .editor
                    .file_path
                    .as_ref()
                    .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
                ws.render_wysiwyg(
                    canvas,
                    &self.editor.text,
                    content_rect,
                    &self.theme,
                    z_base + 1,
                    base_dir.as_deref(),
                );
            }
            EditorMode::Source => {
                render_editor(
                    &self.editor,
                    canvas,
                    z_base + 1,
                    self.provider.as_ref(),
                    &self.theme,
                );
            }
        }

        // Status bar
        self.render_status_bar(canvas, width, height, z_base);
    }

    fn render_status_bar(&self, canvas: &mut Canvas, width: f32, height: f32, z_base: i32) {
        let (ox, oy) = self.origin.unwrap_or((0.0, 0.0));
        let status_y = oy + height - STATUS_BAR_HEIGHT;
        let z_status = z_base + 10;

        // Top separator (1px line).
        canvas.fill_rect(
            ox,
            status_y,
            width,
            1.0,
            Brush::Solid(self.theme.status_border),
            z_status,
        );

        // Background.
        canvas.fill_rect(
            ox,
            status_y + 1.0,
            width,
            STATUS_BAR_HEIGHT - 1.0,
            Brush::Solid(self.theme.status_bg),
            z_status,
        );

        let baseline_y = status_y + STATUS_BAR_HEIGHT * 0.7;
        let provider = self.provider.as_ref();

        // Left: mode indicator + cursor position.
        let left_text = match &self.mode {
            EditorMode::Wysiwyg(ws) => {
                if ws.focused.is_some() {
                    "WYSIWYG (editing)".to_string()
                } else {
                    "WYSIWYG".to_string()
                }
            }
            EditorMode::Source => {
                let (line, col) = self.editor.cursor_line_col();
                format!("Ln {line}, Col {col}")
            }
        };
        canvas.draw_text_direct(
            [ox + STATUS_PAD_X, baseline_y],
            &left_text,
            STATUS_TEXT_SIZE,
            self.theme.status_fg,
            provider,
            z_status + 1,
        );

        // Center: filename (dimmed)
        if let Some(ref path) = self.editor.file_path {
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !name.is_empty() {
                let name_w = canvas.measure_text_width(&name, STATUS_TEXT_SIZE);
                canvas.draw_text_direct(
                    [ox + (width - name_w) * 0.5, baseline_y],
                    &name,
                    STATUS_TEXT_SIZE,
                    self.theme.status_fg,
                    provider,
                    11,
                );
            }
        }

        // Right: word count | encoding | file type | toggle hint
        let words = self.editor.word_count();
        let file_type = self
            .editor
            .file_path
            .as_ref()
            .and_then(|p| {
                std::path::Path::new(p)
                    .extension()
                    .map(|e| e.to_string_lossy().to_uppercase())
            })
            .unwrap_or_else(|| "TXT".to_string());

        let right_text = format!("{words} words   UTF-8   {file_type}   Cmd+Shift+M toggle");
        let right_w = canvas.measure_text_width(&right_text, STATUS_TEXT_SIZE);
        canvas.draw_text_direct(
            [ox + width - right_w - STATUS_PAD_X, baseline_y],
            &right_text,
            STATUS_TEXT_SIZE,
            self.theme.status_fg,
            provider,
            z_status + 1,
        );
    }
}
