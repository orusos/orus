use std::ops::Range;

/// A parsed markdown document: a sequence of blocks with source mapping.
#[derive(Debug, Clone)]
pub struct Document {
    pub blocks: Vec<Block>,
}

/// A top-level or nested block element.
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    /// Byte range in the original source text.
    pub source_range: Range<usize>,
}

#[derive(Debug, Clone)]
pub enum BlockKind {
    Paragraph {
        spans: Vec<InlineSpan>,
    },
    Heading {
        level: u8,
        spans: Vec<InlineSpan>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    BlockQuote {
        children: Vec<Block>,
    },
    OrderedList {
        start: u64,
        items: Vec<ListItem>,
    },
    UnorderedList {
        items: Vec<ListItem>,
    },
    Table {
        headers: Vec<TableCell>,
        rows: Vec<Vec<TableCell>>,
        alignments: Vec<Alignment>,
    },
    ThematicBreak,
}

/// An item in an ordered or unordered list.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub children: Vec<Block>,
}

/// A single table cell containing inline content.
pub type TableCell = Vec<InlineSpan>;

/// Table column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

/// Inline formatted content within a block.
#[derive(Debug, Clone)]
pub enum InlineSpan {
    Text(String),
    Bold(Vec<InlineSpan>),
    Italic(Vec<InlineSpan>),
    Strikethrough(Vec<InlineSpan>),
    Code(String),
    Link {
        text: Vec<InlineSpan>,
        #[allow(dead_code)] // Used by tests; Cmd+Click navigation deferred.
        url: String,
    },
    Image {
        alt: String,
        url: String,
    },
    SoftBreak,
    HardBreak,
}
