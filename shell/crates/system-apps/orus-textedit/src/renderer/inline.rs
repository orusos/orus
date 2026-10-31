use std::path::Path;

use jag_draw::Brush;
use jag_surface::{Canvas, ImageFitMode};

use crate::markdown::model::InlineSpan;
use crate::theme::Theme;

use super::style::SpanStyle;

/// Pen state for inline layout (tracks current x/y position during rendering).
pub(crate) struct Pen {
    pub x: f32,
    pub y: f32,
    pub start_x: f32,
    pub max_width: f32,
    pub line_height: f32,
}

impl Pen {
    pub fn new(x: f32, y: f32, max_width: f32, line_height: f32) -> Self {
        Self {
            x,
            y,
            start_x: x,
            max_width,
            line_height,
        }
    }

    fn wrap(&mut self) {
        self.x = self.start_x;
        self.y += self.line_height;
    }

    /// Total height consumed from the initial y to the bottom of the current line.
    pub fn height_from(&self, start_y: f32) -> f32 {
        (self.y - start_y) + self.line_height
    }
}

/// Render a slice of InlineSpans at the given pen position.
pub(crate) fn render_spans(
    canvas: &mut Canvas,
    spans: &[InlineSpan],
    pen: &mut Pen,
    style: &SpanStyle,
    theme: &Theme,
    z: i32,
    base_dir: Option<&Path>,
) {
    for span in spans {
        render_span(canvas, span, pen, style, theme, z, base_dir);
    }
}

fn render_span(
    canvas: &mut Canvas,
    span: &InlineSpan,
    pen: &mut Pen,
    style: &SpanStyle,
    theme: &Theme,
    z: i32,
    base_dir: Option<&Path>,
) {
    match span {
        InlineSpan::Text(text) => {
            render_text_with_wrap(canvas, text, pen, style, z);
        }
        InlineSpan::Code(code) => {
            let code_style = style.with_code(theme);
            render_inline_code(canvas, code, pen, &code_style, theme, z);
        }
        InlineSpan::Bold(children) => {
            let bold = style.with_bold();
            render_spans(canvas, children, pen, &bold, theme, z, base_dir);
        }
        InlineSpan::Italic(children) => {
            let italic = style.with_italic();
            render_spans(canvas, children, pen, &italic, theme, z, base_dir);
        }
        InlineSpan::Strikethrough(children) => {
            let st = style.with_strikethrough(theme);
            render_spans(canvas, children, pen, &st, theme, z, base_dir);
        }
        InlineSpan::Link { text, .. } => {
            let link_style = style.with_link(theme);
            render_spans(canvas, text, pen, &link_style, theme, z, base_dir);
        }
        InlineSpan::Image { alt, url } => {
            render_image(canvas, url, alt, pen, style, theme, z, base_dir);
        }
        InlineSpan::SoftBreak => {
            let space_w = measure_styled(canvas, " ", style);
            pen.x += space_w;
        }
        InlineSpan::HardBreak => {
            pen.wrap();
        }
    }
}

/// Maximum height for rendered images.
const IMAGE_MAX_HEIGHT: f32 = 300.0;

/// Render a markdown image — either as an actual image or as an alt-text placeholder.
#[allow(clippy::too_many_arguments)]
fn render_image(
    canvas: &mut Canvas,
    url: &str,
    alt: &str,
    pen: &mut Pen,
    style: &SpanStyle,
    theme: &Theme,
    z: i32,
    base_dir: Option<&Path>,
) {
    let resolved = resolve_image_path(url, base_dir);
    let is_valid = resolved.as_ref().is_some_and(|p| p.exists());

    if is_valid {
        let path = resolved.unwrap();
        // Force a line break so the image starts on its own line.
        if pen.x > pen.start_x {
            pen.wrap();
        }
        canvas.draw_image(
            path,
            [pen.x, pen.y],
            [pen.max_width, IMAGE_MAX_HEIGHT],
            ImageFitMode::Contain,
            z,
        );
        pen.y += IMAGE_MAX_HEIGHT;
        pen.x = pen.start_x;
    } else {
        // Fallback: render alt text as dimmed placeholder.
        let placeholder = format!("[image: {alt}]");
        let img_style = SpanStyle {
            color: theme.md_strikethrough,
            ..style.clone()
        };
        render_text_with_wrap(canvas, &placeholder, pen, &img_style, z);
    }
}

fn resolve_image_path(url: &str, base_dir: Option<&Path>) -> Option<std::path::PathBuf> {
    let path = Path::new(url);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    base_dir.map(|dir| dir.join(url))
}

/// Measure text width using the full style (weight, italic, family) so that
/// measurement matches rendering exactly.
fn measure_styled(canvas: &Canvas, text: &str, style: &SpanStyle) -> f32 {
    canvas.measure_text_width_styled(
        text,
        style.size,
        style.weight,
        style.font_style,
        style.family.as_deref(),
    )
}

/// Render text with word-wrapping at the current pen position.
fn render_text_with_wrap(
    canvas: &mut Canvas,
    text: &str,
    pen: &mut Pen,
    style: &SpanStyle,
    z: i32,
) {
    if text.is_empty() {
        return;
    }

    // Split into word chunks, preserving spaces.
    for chunk in WordChunks::new(text) {
        let chunk_w = measure_styled(canvas, chunk, style);

        // Wrap if this chunk won't fit and we're not at the start of a line.
        if pen.x > pen.start_x && pen.x + chunk_w > pen.start_x + pen.max_width {
            pen.wrap();
        }

        let baseline_y = pen.y + pen.line_height * 0.75;
        draw_styled_text(canvas, chunk, pen.x, baseline_y, style, z);

        // Strikethrough decoration.
        if style.strikethrough {
            let strike_y = pen.y + pen.line_height * 0.5;
            canvas.fill_rect(
                pen.x,
                strike_y,
                chunk_w,
                1.0,
                Brush::Solid(style.color),
                z + 1,
            );
        }

        // Underline decoration (links).
        if style.underline {
            let underline_y = pen.y + pen.line_height * 0.85;
            canvas.fill_rect(
                pen.x,
                underline_y,
                chunk_w,
                1.0,
                Brush::Solid(style.color),
                z + 1,
            );
        }

        pen.x += chunk_w;
    }
}

/// Render inline code with a subtle background.
fn render_inline_code(
    canvas: &mut Canvas,
    code: &str,
    pen: &mut Pen,
    style: &SpanStyle,
    theme: &Theme,
    z: i32,
) {
    let pad_x = 4.0;
    let pad_y = 2.0;
    let code_w = measure_styled(canvas, code, style);
    let total_w = code_w + pad_x * 2.0;

    // Wrap if needed.
    if pen.x > pen.start_x && pen.x + total_w > pen.start_x + pen.max_width {
        pen.wrap();
    }

    // Background pill.
    let bg_y = pen.y + pen.line_height * 0.75 - style.size - pad_y;
    let bg_h = style.size + pad_y * 2.0;
    canvas.fill_rect(
        pen.x,
        bg_y,
        total_w,
        bg_h,
        Brush::Solid(theme.md_code_bg),
        z,
    );

    let baseline_y = pen.y + pen.line_height * 0.75;
    draw_styled_text(canvas, code, pen.x + pad_x, baseline_y, style, z + 1);

    pen.x += total_w;
}

/// Draw a single text run with full style control.
fn draw_styled_text(
    canvas: &mut Canvas,
    text: &str,
    x: f32,
    baseline_y: f32,
    style: &SpanStyle,
    z: i32,
) {
    canvas.draw_text_run_styled(
        [x, baseline_y],
        text.to_string(),
        style.size,
        style.weight,
        style.font_style,
        style.family.clone(),
        style.color,
        z,
    );
}

/// Measure the width of a single inline span (without rendering).
pub(crate) fn measure_span_width(canvas: &Canvas, span: &InlineSpan, style: &SpanStyle) -> f32 {
    match span {
        InlineSpan::Text(t) => measure_styled(canvas, t, style),
        InlineSpan::Code(c) => measure_styled(canvas, c, style) + 8.0,
        InlineSpan::Bold(ch) | InlineSpan::Italic(ch) | InlineSpan::Strikethrough(ch) => ch
            .iter()
            .map(|s| measure_span_width(canvas, s, style))
            .sum(),
        InlineSpan::Link { text, .. } => text
            .iter()
            .map(|s| measure_span_width(canvas, s, style))
            .sum(),
        InlineSpan::Image { alt, .. } => measure_styled(canvas, &format!("[image: {alt}]"), style),
        InlineSpan::SoftBreak => measure_styled(canvas, " ", style),
        InlineSpan::HardBreak => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Word chunking iterator
// ---------------------------------------------------------------------------

/// Splits text into chunks suitable for word-wrapping: alternating between
/// whitespace runs and non-whitespace runs. This preserves trailing spaces
/// so words are measured with their natural spacing.
struct WordChunks<'a> {
    remaining: &'a str,
}

impl<'a> WordChunks<'a> {
    fn new(text: &'a str) -> Self {
        Self { remaining: text }
    }
}

impl<'a> Iterator for WordChunks<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        let bytes = self.remaining.as_bytes();
        let is_space = bytes[0].is_ascii_whitespace();

        let end = bytes
            .iter()
            .position(|&b| b.is_ascii_whitespace() != is_space)
            .unwrap_or(bytes.len());

        let chunk = &self.remaining[..end];
        self.remaining = &self.remaining[end..];
        Some(chunk)
    }
}

/// Convenience: compute the total line height for a given font size.
pub(crate) fn line_height_for_size(size: f32) -> f32 {
    size * 1.5
}
