//! Lightweight markdown syntax highlighting for the raw-mode focused block editor.
//!
//! Produces a list of styled spans for a single line of raw markdown text.
//! This is intentionally approximate — fast per-line tokenization, not a full parse.

use jag_draw::{ColorLinPremul, FontStyle};

use crate::theme::Theme;

/// A styled span within a single line.
pub(crate) struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub color: ColorLinPremul,
    pub weight: f32,
    pub style: FontStyle,
}

/// Highlight a single line of raw markdown, returning styled spans.
///
/// Spans cover the entire line with no gaps. Unhighlighted regions use the
/// default foreground color.
pub(crate) fn highlight_line(line: &str, theme: &Theme) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return spans;
    }

    // --- Heading: line starts with one or more `#` followed by space ---
    if bytes[0] == b'#' {
        let hash_end = bytes.iter().take_while(|&&b| b == b'#').count();
        if hash_end <= 6 && bytes.get(hash_end) == Some(&b' ') {
            spans.push(HighlightSpan {
                start: 0,
                end: hash_end + 1,
                color: theme.md_hr,
                weight: 400.0,
                style: FontStyle::Normal,
            });
            if hash_end + 1 < len {
                spans.push(HighlightSpan {
                    start: hash_end + 1,
                    end: len,
                    color: theme.md_heading,
                    weight: 700.0,
                    style: FontStyle::Normal,
                });
            }
            return spans;
        }
    }

    // --- Blockquote: line starts with `> ` ---
    if bytes[0] == b'>' {
        let marker_end = if bytes.get(1) == Some(&b' ') { 2 } else { 1 };
        spans.push(HighlightSpan {
            start: 0,
            end: marker_end,
            color: theme.md_blockquote_border,
            weight: 400.0,
            style: FontStyle::Normal,
        });
        if marker_end < len {
            spans.push(HighlightSpan {
                start: marker_end,
                end: len,
                color: theme.md_blockquote_fg,
                weight: 400.0,
                style: FontStyle::Italic,
            });
        }
        return spans;
    }

    // --- Thematic break: `---` (with optional spaces) ---
    let trimmed = line.trim();
    if trimmed.len() >= 3
        && trimmed.bytes().all(|b| b == b'-' || b == b' ')
        && trimmed.contains('-')
    {
        spans.push(HighlightSpan {
            start: 0,
            end: len,
            color: theme.md_hr,
            weight: 400.0,
            style: FontStyle::Normal,
        });
        return spans;
    }

    // --- Code fence: ``` at start of line ---
    if line.starts_with("```") {
        spans.push(HighlightSpan {
            start: 0,
            end: len,
            color: theme.md_code_fg,
            weight: 400.0,
            style: FontStyle::Normal,
        });
        return spans;
    }

    // --- Inline tokenization for mixed content ---
    highlight_inline(line, theme, &mut spans);

    // Fill gaps with default foreground.
    fill_gaps(&mut spans, len, theme.editor_fg);

    spans
}

/// Tokenize inline markdown: bold, italic, code, strikethrough, links.
fn highlight_inline(line: &str, theme: &Theme, spans: &mut Vec<HighlightSpan>) {
    let bytes = line.as_bytes();
    let len = bytes.len();

    // Detect leading list marker and highlight it.
    let content_start = highlight_list_marker(line, theme, spans);

    let mut i = content_start;
    while i < len {
        // Inline code: `...`
        if bytes[i] == b'`'
            && let Some(end) = find_closing_backtick(bytes, i + 1)
        {
            push_marker(spans, i, i + 1, theme);
            if i + 1 < end {
                spans.push(HighlightSpan {
                    start: i + 1,
                    end,
                    color: theme.md_code_fg,
                    weight: 400.0,
                    style: FontStyle::Normal,
                });
            }
            push_marker(spans, end, end + 1, theme);
            i = end + 1;
            continue;
        }

        // Bold: **...**
        if i + 1 < len
            && bytes[i] == b'*'
            && bytes[i + 1] == b'*'
            && let Some(end) = find_double_marker(bytes, i + 2, b'*')
        {
            push_marker(spans, i, i + 2, theme);
            if i + 2 < end {
                spans.push(HighlightSpan {
                    start: i + 2,
                    end,
                    color: theme.editor_fg,
                    weight: 700.0,
                    style: FontStyle::Normal,
                });
            }
            push_marker(spans, end, end + 2, theme);
            i = end + 2;
            continue;
        }

        // Strikethrough: ~~...~~
        if i + 1 < len
            && bytes[i] == b'~'
            && bytes[i + 1] == b'~'
            && let Some(end) = find_double_marker(bytes, i + 2, b'~')
        {
            push_marker(spans, i, i + 2, theme);
            if i + 2 < end {
                spans.push(HighlightSpan {
                    start: i + 2,
                    end,
                    color: theme.md_strikethrough,
                    weight: 400.0,
                    style: FontStyle::Normal,
                });
            }
            push_marker(spans, end, end + 2, theme);
            i = end + 2;
            continue;
        }

        // Italic: *...*  (single asterisk, only if not followed by another *)
        if bytes[i] == b'*'
            && (i + 1 >= len || bytes[i + 1] != b'*')
            && let Some(end) = find_single_marker(bytes, i + 1, b'*')
        {
            push_marker(spans, i, i + 1, theme);
            if i + 1 < end {
                spans.push(HighlightSpan {
                    start: i + 1,
                    end,
                    color: theme.editor_fg,
                    weight: 400.0,
                    style: FontStyle::Italic,
                });
            }
            push_marker(spans, end, end + 1, theme);
            i = end + 1;
            continue;
        }

        // Link: [text](url)
        if bytes[i] == b'['
            && let Some((text_end, url_end)) = find_link(bytes, i)
        {
            push_marker(spans, i, i + 1, theme);
            if i + 1 < text_end {
                spans.push(HighlightSpan {
                    start: i + 1,
                    end: text_end,
                    color: theme.md_link,
                    weight: 400.0,
                    style: FontStyle::Normal,
                });
            }
            push_marker(spans, text_end, url_end, theme);
            i = url_end;
            continue;
        }

        i += 1;
    }
}

/// Push a dimmed marker span (for syntax characters like `**`, `` ` ``, `#`).
fn push_marker(spans: &mut Vec<HighlightSpan>, start: usize, end: usize, theme: &Theme) {
    spans.push(HighlightSpan {
        start,
        end,
        color: theme.md_hr,
        weight: 400.0,
        style: FontStyle::Normal,
    });
}

/// Detect and highlight a leading list marker (- , * , 1. , - [ ] , - [x]).
/// Returns the byte offset where content starts (after the marker).
fn highlight_list_marker(line: &str, theme: &Theme, spans: &mut Vec<HighlightSpan>) -> usize {
    let bytes = line.as_bytes();
    let len = bytes.len();

    let indent: usize = bytes.iter().take_while(|&&b| b == b' ').count();
    let rest = &bytes[indent..];

    // Unordered: `- ` or `* `
    if rest.len() >= 2 && (rest[0] == b'-' || rest[0] == b'*') && rest[1] == b' ' {
        let marker_end = indent + 2;
        // Task list: `- [ ] ` or `- [x] `
        if rest.len() >= 6
            && rest[2] == b'['
            && (rest[3] == b' ' || rest[3] == b'x')
            && rest[4] == b']'
            && rest[5] == b' '
        {
            spans.push(HighlightSpan {
                start: 0,
                end: indent + 6,
                color: theme.md_checkbox,
                weight: 400.0,
                style: FontStyle::Normal,
            });
            return indent + 6;
        }
        spans.push(HighlightSpan {
            start: 0,
            end: marker_end,
            color: theme.md_bullet,
            weight: 400.0,
            style: FontStyle::Normal,
        });
        return marker_end;
    }

    // Ordered: `1. `, `12. `, etc.
    let digit_end = indent + rest.iter().take_while(|&&b| b.is_ascii_digit()).count();
    if digit_end > indent && digit_end < len && bytes[digit_end] == b'.' {
        let after_dot = digit_end + 1;
        if after_dot < len && bytes[after_dot] == b' ' {
            let marker_end = after_dot + 1;
            spans.push(HighlightSpan {
                start: 0,
                end: marker_end,
                color: theme.md_bullet,
                weight: 400.0,
                style: FontStyle::Normal,
            });
            return marker_end;
        }
    }

    0
}

// --- Marker search helpers ---

fn find_closing_backtick(bytes: &[u8], from: usize) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|&b| b == b'`')
        .map(|p| from + p)
}

fn find_single_marker(bytes: &[u8], from: usize, marker: u8) -> Option<usize> {
    (from..bytes.len()).find(|&i| bytes[i] == marker)
}

fn find_double_marker(bytes: &[u8], from: usize, marker: u8) -> Option<usize> {
    (from..bytes.len().saturating_sub(1)).find(|&i| bytes[i] == marker && bytes[i + 1] == marker)
}

/// Find a markdown link `[text](url)` starting at `[` position.
/// Returns `(text_end, url_end)` — the byte offset of `]` and `)` + 1.
fn find_link(bytes: &[u8], open: usize) -> Option<(usize, usize)> {
    let text_end = bytes[open + 1..]
        .iter()
        .position(|&b| b == b']')
        .map(|p| open + 1 + p)?;
    if bytes.get(text_end + 1) != Some(&b'(') {
        return None;
    }
    let url_end = bytes[text_end + 2..]
        .iter()
        .position(|&b| b == b')')
        .map(|p| text_end + 2 + p + 1)?;
    Some((text_end, url_end))
}

/// Fill gaps between spans with the default foreground color.
fn fill_gaps(spans: &mut Vec<HighlightSpan>, line_len: usize, default_color: ColorLinPremul) {
    if spans.is_empty() {
        spans.push(HighlightSpan {
            start: 0,
            end: line_len,
            color: default_color,
            weight: 400.0,
            style: FontStyle::Normal,
        });
        return;
    }

    spans.sort_by_key(|s| s.start);

    let mut filled = Vec::with_capacity(spans.len() * 2);
    let mut pos = 0;

    for span in spans.drain(..) {
        if span.start > pos {
            filled.push(HighlightSpan {
                start: pos,
                end: span.start,
                color: default_color,
                weight: 400.0,
                style: FontStyle::Normal,
            });
        }
        pos = span.end;
        filled.push(span);
    }

    if pos < line_len {
        filled.push(HighlightSpan {
            start: pos,
            end: line_len,
            color: default_color,
            weight: 400.0,
            style: FontStyle::Normal,
        });
    }

    *spans = filled;
}
