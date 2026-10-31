use std::ops::Range;

use super::model::Document;

/// Maps between block indices and source byte ranges.
///
/// Built from a parsed `Document`, this allows:
/// - Finding which block contains a given byte offset (for click-to-focus).
/// - Getting the source range for a block (for raw-mode editing).
/// - Knowing which blocks need re-parsing after an edit at a byte offset.
pub struct SourceMap {
    /// One entry per block, in document order.
    entries: Vec<SourceEntry>,
}

struct SourceEntry {
    block_index: usize,
    range: Range<usize>,
}

impl SourceMap {
    /// Build a source map from a parsed document.
    pub fn from_document(doc: &Document) -> Self {
        let entries = doc
            .blocks
            .iter()
            .enumerate()
            .map(|(i, block)| SourceEntry {
                block_index: i,
                range: block.source_range.clone(),
            })
            .collect();
        Self { entries }
    }

    /// Find the block index that contains the given byte offset.
    #[allow(dead_code)]
    pub fn block_at_offset(&self, offset: usize) -> Option<usize> {
        self.entries
            .iter()
            .find(|e| e.range.contains(&offset))
            .map(|e| e.block_index)
    }

    /// Get the source byte range for a block by index.
    pub fn range_of(&self, block_index: usize) -> Option<Range<usize>> {
        self.entries
            .iter()
            .find(|e| e.block_index == block_index)
            .map(|e| e.range.clone())
    }

    /// Find all block indices that overlap with the given byte range.
    /// Used after an edit to determine which blocks need re-parsing.
    #[allow(dead_code)]
    pub fn blocks_in_range(&self, range: &Range<usize>) -> Vec<usize> {
        self.entries
            .iter()
            .filter(|e| e.range.start < range.end && e.range.end > range.start)
            .map(|e| e.block_index)
            .collect()
    }

    /// Number of mapped blocks.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
