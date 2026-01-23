//! Source file representation

use crate::core::SourceLine;
use crate::error::{DuploError, Result};
use crate::filetype::create_file_type;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Represents a loaded and processed source file
#[derive(Debug)]
pub struct SourceFile {
    /// Full path to the file
    filename: String,
    /// Processed source lines (after cleaning/filtering)
    source_lines: Vec<SourceLine>,
}

impl SourceFile {
    /// Load and process a source file
    ///
    /// # Arguments
    /// * `path` - Path to the source file
    /// * `min_chars` - Minimum characters per line
    /// * `ignore_preprocessor` - Whether to filter preprocessor directives
    ///
    /// # Returns
    /// A processed SourceFile, or an error if the file cannot be read
    pub fn load(path: &str, min_chars: u32, ignore_preprocessor: bool) -> Result<Self> {
        let file = File::open(path).map_err(|e| DuploError::FileNotFound {
            path: path.to_string(),
            reason: e.to_string(),
        })?;

        let reader = BufReader::new(file);
        let raw_lines: Vec<String> = reader
            .lines()
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| DuploError::FileNotFound {
                path: path.to_string(),
                reason: e.to_string(),
            })?;

        let file_type = create_file_type(path, ignore_preprocessor, min_chars);
        let source_lines = file_type.get_cleaned_source_lines(&raw_lines);

        Ok(Self {
            filename: path.to_string(),
            source_lines,
        })
    }

    /// Create a SourceFile from already-processed lines (for testing)
    #[cfg(test)]
    pub fn from_lines(filename: String, source_lines: Vec<SourceLine>) -> Self {
        Self {
            filename,
            source_lines,
        }
    }

    /// Get the filename
    #[inline]
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Get just the file name without the directory path
    pub fn basename(&self) -> &str {
        Path::new(&self.filename)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.filename)
    }

    /// Get the number of cleaned source lines
    #[inline]
    pub fn num_lines(&self) -> usize {
        self.source_lines.len()
    }

    /// Get a specific line by index
    #[inline]
    pub fn get_line(&self, index: usize) -> &SourceLine {
        &self.source_lines[index]
    }

    /// Get a range of line texts
    pub fn get_lines(&self, start: usize, end: usize) -> Vec<&str> {
        self.source_lines[start..end]
            .iter()
            .map(|l| l.line())
            .collect()
    }

    /// Iterate over all source lines
    pub fn lines(&self) -> impl Iterator<Item = &SourceLine> {
        self.source_lines.iter()
    }

    /// Check if two files have the same basename (for -d flag)
    pub fn has_same_basename(&self, other: &SourceFile) -> bool {
        self.basename() == other.basename()
    }
}

impl PartialEq for SourceFile {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.filename == other.filename
    }
}

impl Eq for SourceFile {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_from_lines() {
        let lines = vec![
            SourceLine::new("int x = 5;".to_string(), 1),
            SourceLine::new("int y = 10;".to_string(), 2),
        ];
        let sf = SourceFile::from_lines("test.c".to_string(), lines);

        assert_eq!(sf.filename(), "test.c");
        assert_eq!(sf.num_lines(), 2);
        assert_eq!(sf.get_line(0).line(), "int x = 5;");
    }

    #[test]
    fn test_basename() {
        let sf = SourceFile::from_lines("/path/to/test.c".to_string(), vec![]);
        assert_eq!(sf.basename(), "test.c");
    }

    #[test]
    fn test_same_basename() {
        let sf1 = SourceFile::from_lines("/path/a/test.c".to_string(), vec![]);
        let sf2 = SourceFile::from_lines("/path/b/test.c".to_string(), vec![]);
        let sf3 = SourceFile::from_lines("/path/a/other.c".to_string(), vec![]);

        assert!(sf1.has_same_basename(&sf2));
        assert!(!sf1.has_same_basename(&sf3));
    }

    #[test]
    fn test_get_lines() {
        let lines = vec![
            SourceLine::new("line1".to_string(), 1),
            SourceLine::new("line2".to_string(), 2),
            SourceLine::new("line3".to_string(), 3),
        ];
        let sf = SourceFile::from_lines("test.c".to_string(), lines);

        let range = sf.get_lines(0, 2);
        assert_eq!(range, vec!["line1", "line2"]);
    }

    #[test]
    fn test_equality() {
        let sf1 = SourceFile::from_lines("test.c".to_string(), vec![]);
        let sf2 = SourceFile::from_lines("test.c".to_string(), vec![]);
        let sf3 = SourceFile::from_lines("other.c".to_string(), vec![]);

        assert_eq!(sf1, sf2);
        assert_ne!(sf1, sf3);
    }
}
