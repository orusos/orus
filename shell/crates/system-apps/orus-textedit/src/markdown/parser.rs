use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::model::{Alignment, Block, BlockKind, Document, InlineSpan, ListItem, TableCell};

/// Parse a markdown string into a structured `Document`.
pub fn parse(source: &str) -> Document {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(source, options);
    let mut builder = Builder::new();
    for (event, range) in parser.into_offset_iter() {
        builder.process(event, range);
    }
    builder.finish()
}

// ---------------------------------------------------------------------------
// Stack-based builder
// ---------------------------------------------------------------------------

/// Each frame on the builder stack represents an open container.
enum Frame {
    // Block containers (accept child blocks).
    Root(Vec<Block>),
    BlockQuote {
        children: Vec<Block>,
        start: usize,
    },
    ListItem {
        checked: Option<bool>,
        children: Vec<Block>,
        /// Inline spans that arrive without a Paragraph wrapper (tight lists).
        inline_spans: Vec<InlineSpan>,
    },
    OrderedList {
        start_num: u64,
        items: Vec<ListItem>,
        start: usize,
    },
    UnorderedList {
        items: Vec<ListItem>,
        start: usize,
    },

    // Block-level elements with inline content.
    Paragraph {
        spans: Vec<InlineSpan>,
        start: usize,
    },
    Heading {
        level: u8,
        spans: Vec<InlineSpan>,
        start: usize,
    },
    CodeBlock {
        lang: Option<String>,
        code: String,
        start: usize,
    },

    // Table structure.
    Table {
        aligns: Vec<Alignment>,
        headers: Vec<TableCell>,
        rows: Vec<Vec<TableCell>>,
        start: usize,
    },
    TableHead {
        cells: Vec<TableCell>,
    },
    TableRow {
        cells: Vec<TableCell>,
    },
    TableCell {
        spans: Vec<InlineSpan>,
    },

    // Inline containers (accept child spans).
    Emphasis(Vec<InlineSpan>),
    Strong(Vec<InlineSpan>),
    Strikethrough(Vec<InlineSpan>),
    Link {
        spans: Vec<InlineSpan>,
        url: String,
    },
    Image {
        alt_spans: Vec<InlineSpan>,
        url: String,
    },
}

struct Builder {
    stack: Vec<Frame>,
}

impl Builder {
    fn new() -> Self {
        Self {
            stack: vec![Frame::Root(Vec::new())],
        }
    }

    /// Add an inline span to the innermost inline-accepting frame.
    fn push_span(&mut self, span: InlineSpan) {
        let Some(top) = self.stack.last_mut() else {
            return;
        };
        match top {
            Frame::Paragraph { spans, .. }
            | Frame::Heading { spans, .. }
            | Frame::TableCell { spans }
            | Frame::Emphasis(spans)
            | Frame::Strong(spans)
            | Frame::Strikethrough(spans)
            | Frame::Link { spans, .. }
            | Frame::Image {
                alt_spans: spans, ..
            } => spans.push(span),
            // Tight list items: pulldown-cmark may emit inline content
            // directly inside Item without a Paragraph wrapper.
            Frame::ListItem { inline_spans, .. } => inline_spans.push(span),
            _ => {}
        }
    }

    /// Add a completed block to the innermost block-accepting frame.
    fn push_block(&mut self, block: Block) {
        let Some(top) = self.stack.last_mut() else {
            return;
        };
        match top {
            Frame::Root(blocks)
            | Frame::BlockQuote {
                children: blocks, ..
            } => blocks.push(block),
            Frame::ListItem { children, .. } => children.push(block),
            _ => {}
        }
    }

    fn process(&mut self, event: Event<'_>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.open(tag, range),
            Event::End(tag_end) => self.close(tag_end, range),

            // Text inside a code block appends to the code string.
            Event::Text(text) if matches!(self.stack.last(), Some(Frame::CodeBlock { .. })) => {
                if let Some(Frame::CodeBlock { code, .. }) = self.stack.last_mut() {
                    code.push_str(&text);
                }
            }
            Event::Text(text) => self.push_span(InlineSpan::Text(text.to_string())),
            Event::Code(code) => self.push_span(InlineSpan::Code(code.to_string())),
            Event::SoftBreak => self.push_span(InlineSpan::SoftBreak),
            Event::HardBreak => self.push_span(InlineSpan::HardBreak),
            Event::Rule => self.push_block(Block {
                kind: BlockKind::ThematicBreak,
                source_range: range,
            }),
            Event::TaskListMarker(checked) => {
                if let Some(Frame::ListItem { checked: c, .. }) = self.stack.last_mut() {
                    *c = Some(checked);
                }
            }
            // HTML, math, footnotes — skip for now.
            _ => {}
        }
    }

    fn open(&mut self, tag: Tag<'_>, range: Range<usize>) {
        let s = range.start;
        match tag {
            Tag::Paragraph => self.stack.push(Frame::Paragraph {
                spans: Vec::new(),
                start: s,
            }),
            Tag::Heading { level, .. } => self.stack.push(Frame::Heading {
                level: heading_to_u8(level),
                spans: Vec::new(),
                start: s,
            }),
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(l) if !l.is_empty() => Some(l.to_string()),
                    _ => None,
                };
                self.stack.push(Frame::CodeBlock {
                    lang,
                    code: String::new(),
                    start: s,
                });
            }
            Tag::BlockQuote(_) => self.stack.push(Frame::BlockQuote {
                children: Vec::new(),
                start: s,
            }),
            Tag::List(start_num) => {
                if let Some(n) = start_num {
                    self.stack.push(Frame::OrderedList {
                        start_num: n,
                        items: Vec::new(),
                        start: s,
                    });
                } else {
                    self.stack.push(Frame::UnorderedList {
                        items: Vec::new(),
                        start: s,
                    });
                }
            }
            Tag::Item => self.stack.push(Frame::ListItem {
                checked: None,
                children: Vec::new(),
                inline_spans: Vec::new(),
            }),
            Tag::Table(aligns) => {
                let aligns = aligns.into_iter().map(convert_alignment).collect();
                self.stack.push(Frame::Table {
                    aligns,
                    headers: Vec::new(),
                    rows: Vec::new(),
                    start: s,
                });
            }
            Tag::TableHead => self.stack.push(Frame::TableHead { cells: Vec::new() }),
            Tag::TableRow => self.stack.push(Frame::TableRow { cells: Vec::new() }),
            Tag::TableCell => self.stack.push(Frame::TableCell { spans: Vec::new() }),

            Tag::Emphasis => self.stack.push(Frame::Emphasis(Vec::new())),
            Tag::Strong => self.stack.push(Frame::Strong(Vec::new())),
            Tag::Strikethrough => self.stack.push(Frame::Strikethrough(Vec::new())),
            Tag::Link { dest_url, .. } => self.stack.push(Frame::Link {
                spans: Vec::new(),
                url: dest_url.to_string(),
            }),
            Tag::Image { dest_url, .. } => self.stack.push(Frame::Image {
                alt_spans: Vec::new(),
                url: dest_url.to_string(),
            }),
            _ => {} // HtmlBlock, FootnoteDefinition, MetadataBlock, DefinitionList
        }
    }

    fn close(&mut self, tag_end: TagEnd, range: Range<usize>) {
        let Some(frame) = self.stack.pop() else {
            return;
        };
        let end = range.end;

        match (tag_end, frame) {
            // -- Blocks with inline content --
            (TagEnd::Paragraph, Frame::Paragraph { spans, start }) => {
                self.push_block(Block {
                    kind: BlockKind::Paragraph { spans },
                    source_range: start..end,
                });
            }
            (
                TagEnd::Heading(_),
                Frame::Heading {
                    level,
                    spans,
                    start,
                },
            ) => {
                self.push_block(Block {
                    kind: BlockKind::Heading { level, spans },
                    source_range: start..end,
                });
            }
            (TagEnd::CodeBlock, Frame::CodeBlock { lang, code, start }) => {
                self.push_block(Block {
                    kind: BlockKind::CodeBlock {
                        language: lang,
                        code,
                    },
                    source_range: start..end,
                });
            }

            // -- Block containers --
            (TagEnd::BlockQuote(_), Frame::BlockQuote { children, start }) => {
                self.push_block(Block {
                    kind: BlockKind::BlockQuote { children },
                    source_range: start..end,
                });
            }
            (
                TagEnd::List(true),
                Frame::OrderedList {
                    start_num,
                    items,
                    start,
                },
            ) => {
                self.push_block(Block {
                    kind: BlockKind::OrderedList {
                        start: start_num,
                        items,
                    },
                    source_range: start..end,
                });
            }
            (TagEnd::List(false), Frame::UnorderedList { items, start }) => {
                self.push_block(Block {
                    kind: BlockKind::UnorderedList { items },
                    source_range: start..end,
                });
            }
            (
                TagEnd::Item,
                Frame::ListItem {
                    checked,
                    mut children,
                    inline_spans,
                },
            ) => {
                // Tight list items: pulldown-cmark omits the Paragraph wrapper,
                // so inline content collects in `inline_spans`. Synthesize a
                // Paragraph block so the renderer always sees uniform structure.
                if !inline_spans.is_empty() {
                    children.insert(
                        0,
                        Block {
                            kind: BlockKind::Paragraph {
                                spans: inline_spans,
                            },
                            source_range: range.clone(),
                        },
                    );
                }
                let item = ListItem { checked, children };
                match self.stack.last_mut() {
                    Some(Frame::OrderedList { items, .. })
                    | Some(Frame::UnorderedList { items, .. }) => items.push(item),
                    _ => {}
                }
            }

            // -- Table structure --
            (
                TagEnd::Table,
                Frame::Table {
                    aligns,
                    headers,
                    rows,
                    start,
                },
            ) => {
                self.push_block(Block {
                    kind: BlockKind::Table {
                        headers,
                        rows,
                        alignments: aligns,
                    },
                    source_range: start..end,
                });
            }
            (TagEnd::TableHead, Frame::TableHead { cells }) => {
                if let Some(Frame::Table { headers, .. }) = self.stack.last_mut() {
                    *headers = cells;
                }
            }
            (TagEnd::TableRow, Frame::TableRow { cells }) => {
                if let Some(Frame::Table { rows, .. }) = self.stack.last_mut() {
                    rows.push(cells);
                }
            }
            (TagEnd::TableCell, Frame::TableCell { spans }) => match self.stack.last_mut() {
                Some(Frame::TableHead { cells }) | Some(Frame::TableRow { cells }) => {
                    cells.push(spans);
                }
                _ => {}
            },

            // -- Inline containers → become spans --
            (TagEnd::Emphasis, Frame::Emphasis(spans)) => {
                self.push_span(InlineSpan::Italic(spans));
            }
            (TagEnd::Strong, Frame::Strong(spans)) => {
                self.push_span(InlineSpan::Bold(spans));
            }
            (TagEnd::Strikethrough, Frame::Strikethrough(spans)) => {
                self.push_span(InlineSpan::Strikethrough(spans));
            }
            (TagEnd::Link, Frame::Link { spans, url }) => {
                self.push_span(InlineSpan::Link { text: spans, url });
            }
            (TagEnd::Image, Frame::Image { alt_spans, url }) => {
                let alt = extract_plain_text(&alt_spans);
                self.push_span(InlineSpan::Image { alt, url });
            }

            // Unhandled / mismatched — silently drop.
            _ => {}
        }
    }

    fn finish(mut self) -> Document {
        match self.stack.pop() {
            Some(Frame::Root(blocks)) => Document { blocks },
            _ => Document { blocks: Vec::new() },
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn heading_to_u8(level: pulldown_cmark::HeadingLevel) -> u8 {
    use pulldown_cmark::HeadingLevel::*;
    match level {
        H1 => 1,
        H2 => 2,
        H3 => 3,
        H4 => 4,
        H5 => 5,
        H6 => 6,
    }
}

fn convert_alignment(a: pulldown_cmark::Alignment) -> Alignment {
    match a {
        pulldown_cmark::Alignment::None => Alignment::None,
        pulldown_cmark::Alignment::Left => Alignment::Left,
        pulldown_cmark::Alignment::Center => Alignment::Center,
        pulldown_cmark::Alignment::Right => Alignment::Right,
    }
}

/// Recursively extract plain text from a span tree (used for image alt text).
fn extract_plain_text(spans: &[InlineSpan]) -> String {
    let mut out = String::new();
    for span in spans {
        match span {
            InlineSpan::Text(t) => out.push_str(t),
            InlineSpan::Code(c) => out.push_str(c),
            InlineSpan::Bold(children)
            | InlineSpan::Italic(children)
            | InlineSpan::Strikethrough(children) => {
                out.push_str(&extract_plain_text(children));
            }
            InlineSpan::Link { text, .. } => out.push_str(&extract_plain_text(text)),
            InlineSpan::Image { alt, .. } => out.push_str(alt),
            InlineSpan::SoftBreak | InlineSpan::HardBreak => out.push(' '),
        }
    }
    out
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
