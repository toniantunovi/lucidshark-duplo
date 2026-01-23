//! Duplicate block representation

/// Represents a detected duplicate code block between two files
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Block {
    /// Index of the first source file in the file list
    pub source1_idx: usize,
    /// Index of the second source file in the file list
    pub source2_idx: usize,
    /// Starting line index in source1 (0-indexed into cleaned lines)
    pub line1: usize,
    /// Starting line index in source2 (0-indexed into cleaned lines)
    pub line2: usize,
    /// Number of consecutive matching lines
    pub count: usize,
}

#[allow(dead_code)]
impl Block {
    /// Create a new Block
    pub fn new(
        source1_idx: usize,
        source2_idx: usize,
        line1: usize,
        line2: usize,
        count: usize,
    ) -> Self {
        Self {
            source1_idx,
            source2_idx,
            line1,
            line2,
            count,
        }
    }

    /// Check if this is a self-duplicate (within the same file)
    pub fn is_self_duplicate(&self) -> bool {
        self.source1_idx == self.source2_idx
    }

    /// Get the ending line index in source1 (exclusive)
    pub fn end1(&self) -> usize {
        self.line1 + self.count
    }

    /// Get the ending line index in source2 (exclusive)
    pub fn end2(&self) -> usize {
        self.line2 + self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new(0, 1, 10, 20, 5);
        assert_eq!(block.source1_idx, 0);
        assert_eq!(block.source2_idx, 1);
        assert_eq!(block.line1, 10);
        assert_eq!(block.line2, 20);
        assert_eq!(block.count, 5);
    }

    #[test]
    fn test_self_duplicate() {
        let self_dup = Block::new(0, 0, 10, 50, 5);
        let cross_dup = Block::new(0, 1, 10, 20, 5);

        assert!(self_dup.is_self_duplicate());
        assert!(!cross_dup.is_self_duplicate());
    }

    #[test]
    fn test_end_indices() {
        let block = Block::new(0, 1, 10, 20, 5);
        assert_eq!(block.end1(), 15);
        assert_eq!(block.end2(), 25);
    }
}
