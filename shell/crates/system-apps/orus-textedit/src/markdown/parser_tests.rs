use super::*;
use crate::markdown::model::*;

/// Helper: return the first block's kind from parsed markdown.
fn first_block(md: &str) -> BlockKind {
    let doc = parse(md);
    assert!(!doc.blocks.is_empty(), "expected at least one block");
    doc.blocks.into_iter().next().unwrap().kind
}

/// Helper: extract text from a flat span list (no nesting).
fn span_text(spans: &[InlineSpan]) -> String {
    extract_plain_text(spans)
}

#[test]
fn paragraph() {
    let BlockKind::Paragraph { spans } = first_block("Hello world") else {
        panic!("expected Paragraph");
    };
    assert_eq!(span_text(&spans), "Hello world");
}

#[test]
fn heading_levels() {
    for level in 1..=6u8 {
        let md = format!("{} Title", "#".repeat(level as usize));
        let BlockKind::Heading { level: l, spans } = first_block(&md) else {
            panic!("expected Heading");
        };
        assert_eq!(l, level);
        assert_eq!(span_text(&spans), "Title");
    }
}

#[test]
fn bold_and_italic() {
    let BlockKind::Paragraph { spans } = first_block("**bold** and *italic*") else {
        panic!("expected Paragraph");
    };
    assert!(matches!(&spans[0], InlineSpan::Bold(_)));
    assert!(matches!(&spans[2], InlineSpan::Italic(_)));
}

#[test]
fn inline_code() {
    let BlockKind::Paragraph { spans } = first_block("use `println!`") else {
        panic!("expected Paragraph");
    };
    assert!(matches!(&spans[1], InlineSpan::Code(c) if c == "println!"));
}

#[test]
fn code_block() {
    let md = "```rust\nfn main() {}\n```";
    let BlockKind::CodeBlock { language, code } = first_block(md) else {
        panic!("expected CodeBlock");
    };
    assert_eq!(language.as_deref(), Some("rust"));
    assert_eq!(code.trim(), "fn main() {}");
}

#[test]
fn unordered_list() {
    let md = "- one\n- two\n- three";
    let BlockKind::UnorderedList { items } = first_block(md) else {
        panic!("expected UnorderedList");
    };
    assert_eq!(items.len(), 3);
    assert!(items[0].checked.is_none());
}

#[test]
fn ordered_list() {
    let md = "1. first\n2. second";
    let BlockKind::OrderedList { start, items } = first_block(md) else {
        panic!("expected OrderedList");
    };
    assert_eq!(start, 1);
    assert_eq!(items.len(), 2);
}

#[test]
fn task_list() {
    let md = "- [ ] todo\n- [x] done";
    let BlockKind::UnorderedList { items } = first_block(md) else {
        panic!("expected UnorderedList");
    };
    assert_eq!(items[0].checked, Some(false));
    assert_eq!(items[1].checked, Some(true));
}

#[test]
fn blockquote() {
    let md = "> quoted text";
    let BlockKind::BlockQuote { children } = first_block(md) else {
        panic!("expected BlockQuote");
    };
    assert!(!children.is_empty());
}

#[test]
fn thematic_break() {
    let doc = parse("above\n\n---\n\nbelow");
    assert!(
        doc.blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::ThematicBreak))
    );
}

#[test]
fn link() {
    let BlockKind::Paragraph { spans } = first_block("[click](https://x.com)") else {
        panic!("expected Paragraph");
    };
    assert!(
        matches!(&spans[0], InlineSpan::Link { url, text } if url == "https://x.com" && !text.is_empty())
    );
}

#[test]
fn image() {
    let BlockKind::Paragraph { spans } = first_block("![alt](img.png)") else {
        panic!("expected Paragraph");
    };
    assert!(
        matches!(&spans[0], InlineSpan::Image { alt, url } if alt == "alt" && url == "img.png")
    );
}

#[test]
fn strikethrough() {
    let BlockKind::Paragraph { spans } = first_block("~~deleted~~") else {
        panic!("expected Paragraph");
    };
    assert!(matches!(&spans[0], InlineSpan::Strikethrough(_)));
}

#[test]
fn table() {
    let md = "| A | B |\n|---|---|\n| 1 | 2 |";
    let BlockKind::Table {
        headers,
        rows,
        alignments,
    } = first_block(md)
    else {
        panic!("expected Table");
    };
    assert_eq!(headers.len(), 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 2);
    assert_eq!(alignments.len(), 2);
}

#[test]
fn source_ranges_are_set() {
    let md = "# Hello\n\nWorld";
    let doc = parse(md);
    assert_eq!(doc.blocks.len(), 2);
    assert_eq!(doc.blocks[0].source_range.start, 0);
    assert!(doc.blocks[1].source_range.start > 0);
}

#[test]
fn nested_bold_italic() {
    let BlockKind::Paragraph { spans } = first_block("***bold italic***") else {
        panic!("expected Paragraph");
    };
    let text = span_text(&spans);
    assert_eq!(text, "bold italic");
}

#[test]
fn tight_list_items_have_text() {
    // Tight list (no blank lines between items) — pulldown-cmark may
    // skip Paragraph wrapping, so the builder must handle inline content
    // arriving directly inside a ListItem frame.
    let md = "- one\n- two\n- three";
    let BlockKind::UnorderedList { items } = first_block(md) else {
        panic!("expected UnorderedList");
    };
    assert_eq!(items.len(), 3);
    // Each item must have children with actual text content.
    for (i, item) in items.iter().enumerate() {
        assert!(
            !item.children.is_empty(),
            "item {i} should have children, got: {item:?}"
        );
        let BlockKind::Paragraph { spans } = &item.children[0].kind else {
            panic!(
                "item {i}: expected Paragraph child, got: {:?}",
                item.children[0].kind
            );
        };
        assert!(!spans.is_empty(), "item {i}: paragraph should have spans");
    }
}

#[test]
fn tight_list_with_bold() {
    let md = "- **bold** text\n- plain text";
    let BlockKind::UnorderedList { items } = first_block(md) else {
        panic!("expected UnorderedList");
    };
    assert_eq!(items.len(), 2);
    // Item 0: "**bold** text" — should have Bold + Text spans.
    let BlockKind::Paragraph { spans } = &items[0].children[0].kind else {
        panic!("expected Paragraph child in item 0");
    };
    assert!(
        matches!(&spans[0], InlineSpan::Bold(_)),
        "expected Bold span, got: {:?}",
        spans[0]
    );
    // Item 1: "plain text" — should have Text span.
    let BlockKind::Paragraph { spans } = &items[1].children[0].kind else {
        panic!("expected Paragraph child in item 1");
    };
    assert_eq!(span_text(spans), "plain text");
}

#[test]
fn nested_list_items_have_text() {
    let md = "- parent\n  - child one\n  - child two";
    let BlockKind::UnorderedList { items } = first_block(md) else {
        panic!("expected UnorderedList");
    };
    assert_eq!(items.len(), 1, "should be 1 top-level item");
    // The parent item should have: Paragraph("parent") + UnorderedList(children).
    assert!(
        items[0].children.len() >= 2,
        "parent item should have paragraph + nested list, got: {:?}",
        items[0].children
    );
    // Check the nested list has text.
    let nested = &items[0].children[1];
    let BlockKind::UnorderedList { items: sub_items } = &nested.kind else {
        panic!("expected nested UnorderedList, got: {:?}", nested.kind);
    };
    assert_eq!(sub_items.len(), 2);
    // Each sub-item should have text.
    for (i, sub) in sub_items.iter().enumerate() {
        assert!(
            !sub.children.is_empty(),
            "sub-item {i} should have children"
        );
    }
}

#[test]
fn source_map_lookup() {
    use crate::markdown::source_map::SourceMap;

    let md = "# Title\n\nParagraph text\n";
    let doc = parse(md);
    let map = SourceMap::from_document(&doc);

    assert_eq!(map.len(), 2);
    assert_eq!(map.block_at_offset(0), Some(0));
    let para_start = doc.blocks[1].source_range.start;
    assert_eq!(map.block_at_offset(para_start), Some(1));
}
