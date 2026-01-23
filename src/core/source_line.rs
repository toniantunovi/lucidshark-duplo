//! Source line representation with hash

use super::hash::hash_line;

/// Represents a single processed source code line
#[derive(Debug, Clone)]
pub struct SourceLine {
    /// The cleaned line text (after comment/preprocessor removal)
    line: String,
    /// Original line number in the source file (1-indexed for display)
    line_number: usize,
    /// FNV-1a hash of the whitespace-normalized line
    hash: u32,
}

impl SourceLine {
    /// Create a new SourceLine with automatic hash computation
    ///
    /// # Arguments
    /// * `line` - The cleaned line text
    /// * `line_number` - The 1-indexed original line number
    pub fn new(line: String, line_number: usize) -> Self {
        let hash = hash_line(&line);
        Self {
            line,
            line_number,
            hash,
        }
    }

    /// Get the line text
    #[inline]
    pub fn line(&self) -> &str {
        &self.line
    }

    /// Get the original line number (1-indexed)
    #[inline]
    pub fn line_number(&self) -> usize {
        self.line_number
    }

    /// Get the hash value
    #[inline]
    pub fn hash(&self) -> u32 {
        self.hash
    }
}

impl PartialEq for SourceLine {
    /// Two source lines are equal if their hashes match
    /// This provides fast comparison for duplicate detection
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for SourceLine {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_line_creation() {
        let line = SourceLine::new("int x = 5;".to_string(), 10);
        assert_eq!(line.line(), "int x = 5;");
        assert_eq!(line.line_number(), 10);
        assert_ne!(line.hash(), 0);
    }

    #[test]
    fn test_source_line_equality() {
        let line1 = SourceLine::new("int x = 5;".to_string(), 10);
        let line2 = SourceLine::new("int x = 5;".to_string(), 20);
        let line3 = SourceLine::new("int y = 5;".to_string(), 10);

        // Same content, different line numbers should be equal (hash-based)
        assert_eq!(line1, line2);
        // Different content should not be equal
        assert_ne!(line1, line3);
    }

    #[test]
    fn test_source_line_whitespace_equality() {
        let line1 = SourceLine::new("int x = 5;".to_string(), 1);
        let line2 = SourceLine::new("int  x  =  5;".to_string(), 2);

        // Whitespace differences should be normalized in hash
        assert_eq!(line1, line2);
    }
}
