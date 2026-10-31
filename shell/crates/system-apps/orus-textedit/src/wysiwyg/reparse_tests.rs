use crate::markdown::parser::parse;

use super::WysiwygState;

/// Compare an incrementally-reparsed state against a fresh full parse.
fn assert_matches_full_reparse(state: &WysiwygState, source: &str) {
    let full_doc = parse(source);
    assert_eq!(
        state.md_doc.blocks.len(),
        full_doc.blocks.len(),
        "block count mismatch: incremental={}, full={}",
        state.md_doc.blocks.len(),
        full_doc.blocks.len(),
    );
    for (i, (actual, expected)) in state
        .md_doc
        .blocks
        .iter()
        .zip(full_doc.blocks.iter())
        .enumerate()
    {
        assert_eq!(
            actual.source_range, expected.source_range,
            "block {i} source_range mismatch: incremental={:?}, full={:?}",
            actual.source_range, expected.source_range,
        );
    }
}

/// Set up a focused block at `block_index`, then replace its text with `new_text`.
fn focus_and_edit(state: &mut WysiwygState, source: &str, block_index: usize, new_text: &str) {
    state.focus_block(block_index, source);
    let focused = state
        .focused
        .as_mut()
        .expect("focus_block should set focused");
    focused.editor.text = new_text.to_string();
    focused.editor.cursor_pos = new_text.len();
    focused.editor.selection = jag_text::layout::Selection::collapsed(new_text.len());
}

#[test]
fn edit_middle_block() {
    let source = "# Heading\n\nParagraph one.\n\nParagraph two.\n\nParagraph three.";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    // Edit block 2 ("Paragraph two.") → "Paragraph TWO EDITED."
    focus_and_edit(&mut state, &source, 2, "Paragraph TWO EDITED.");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}

#[test]
fn edit_first_block() {
    let source = "# Heading\n\nParagraph one.\n\nParagraph two.";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    // Edit block 0 ("# Heading") → "# New Heading"
    focus_and_edit(&mut state, &source, 0, "# New Heading");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}

#[test]
fn edit_last_block() {
    let source = "# Heading\n\nParagraph one.\n\nParagraph two.";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    // Edit block 2 ("Paragraph two.") → "Last block modified."
    focus_and_edit(&mut state, &source, 2, "Last block modified.");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}

#[test]
fn shrink_text() {
    let source =
        "# Heading\n\nThis is a very long paragraph with lots of text.\n\nAnother paragraph.";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    // Shrink block 1 to a short string.
    focus_and_edit(&mut state, &source, 1, "Short.");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}

#[test]
fn edit_changes_block_type() {
    let source = "# Heading\n\nParagraph one.\n\nParagraph two.";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    // Change block 1 from paragraph to heading.
    focus_and_edit(&mut state, &source, 1, "## Now a heading");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}

#[test]
fn single_block_document() {
    let source = "Just one paragraph.";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    focus_and_edit(&mut state, &source, 0, "Edited single paragraph.");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}

#[test]
fn two_block_document_edit_first() {
    let source = "# Title\n\nBody text here.";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    focus_and_edit(&mut state, &source, 0, "# Updated Title");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}

#[test]
fn many_blocks_edit_early() {
    let source = "# H1\n\nA\n\nB\n\nC\n\nD\n\nE\n\nF";
    let mut state = WysiwygState::new(source);
    let mut source = source.to_string();

    // Edit block 1 ("A") → "AAA"
    focus_and_edit(&mut state, &source, 1, "AAA");
    state.commit_focused(&mut source);

    assert_matches_full_reparse(&state, &source);
}
