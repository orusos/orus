use std::sync::OnceLock;

use jag_draw::ColorLinPremul;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

/// A single syntax-highlighted token within a line.
pub(crate) struct HighlightedToken {
    pub text: String,
    pub color: ColorLinPremul,
    pub bold: bool,
    pub italic: bool,
}

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Highlight a code block, returning tokens per line.
///
/// Falls back to plain text (single unstyled token per line) for unknown languages.
pub(crate) fn highlight_code(code: &str, language: Option<&str>) -> Vec<Vec<HighlightedToken>> {
    let ss = syntax_set();
    let ts = theme_set();

    let syntax = language
        .and_then(|lang| ss.find_syntax_by_token(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);

    code.lines()
        .map(|line| {
            let regions = h.highlight_line(line, ss).unwrap_or_default();
            regions
                .into_iter()
                .map(|(style, text)| syntect_to_token(style, text))
                .collect()
        })
        .collect()
}

fn syntect_to_token(style: Style, text: &str) -> HighlightedToken {
    let fg = style.foreground;
    HighlightedToken {
        text: text.to_string(),
        color: ColorLinPremul::from_srgba_u8([fg.r, fg.g, fg.b, fg.a]),
        bold: style
            .font_style
            .contains(syntect::highlighting::FontStyle::BOLD),
        italic: style
            .font_style
            .contains(syntect::highlighting::FontStyle::ITALIC),
    }
}
