use jag_draw::{ColorLinPremul, FontStyle};

use crate::theme::Theme;

/// Resolved style for a text run, accumulated by walking nested InlineSpans.
#[derive(Clone)]
pub(crate) struct SpanStyle {
    pub size: f32,
    pub weight: f32,
    pub font_style: FontStyle,
    pub color: ColorLinPremul,
    pub family: Option<String>,
    pub strikethrough: bool,
    pub underline: bool,
}

impl SpanStyle {
    /// Default body text style.
    pub fn body(theme: &Theme) -> Self {
        Self {
            size: 15.0,
            weight: 400.0,
            font_style: FontStyle::Normal,
            color: theme.editor_fg,
            family: None,
            strikethrough: false,
            underline: false,
        }
    }

    /// Heading style — varies by level.
    pub fn heading(level: u8, theme: &Theme) -> Self {
        let size = match level {
            1 => 32.0,
            2 => 26.0,
            3 => 22.0,
            4 => 18.0,
            5 => 16.0,
            _ => 15.0,
        };
        Self {
            size,
            weight: 700.0,
            font_style: FontStyle::Normal,
            color: theme.md_heading,
            family: None,
            strikethrough: false,
            underline: false,
        }
    }

    /// Derive a bold variant of this style.
    pub fn with_bold(&self) -> Self {
        let mut s = self.clone();
        s.weight = 700.0;
        s
    }

    /// Derive an italic variant of this style.
    pub fn with_italic(&self) -> Self {
        let mut s = self.clone();
        s.font_style = FontStyle::Italic;
        s
    }

    /// Derive an inline-code variant.
    pub fn with_code(&self, theme: &Theme) -> Self {
        let mut s = self.clone();
        s.color = theme.md_code_fg;
        s.family = Some("monospace".to_string());
        s.size = (self.size - 1.0).max(12.0);
        s
    }

    /// Derive a strikethrough variant.
    pub fn with_strikethrough(&self, theme: &Theme) -> Self {
        let mut s = self.clone();
        s.strikethrough = true;
        s.color = theme.md_strikethrough;
        s
    }

    /// Derive a link variant.
    pub fn with_link(&self, theme: &Theme) -> Self {
        let mut s = self.clone();
        s.color = theme.md_link;
        s.underline = true;
        s
    }
}
