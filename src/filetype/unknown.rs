//! Unknown/fallback file type implementation
//!
//! This file type is used for files with unrecognized extensions.
//! It performs minimal processing - only filtering by line length
//! and alphabetic character presence.

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// Unknown file type processor (fallback)
pub struct UnknownFileType {
    min_chars: u32,
}

impl UnknownFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }
}

impl FileType for UnknownFileType {
    fn name(&self) -> &'static str {
        "Unknown"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let cleaned = clean_whitespace(line);

            if cleaned.is_empty() {
                continue;
            }

            if is_valid_line(&cleaned, self.min_chars) {
                result.push(SourceLine::new(cleaned, line_num + 1));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lines() {
        let ft = UnknownFileType::new(3);
        let lines = vec![
            "some text here".to_string(),
            "more text".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_empty_lines_filtered() {
        let ft = UnknownFileType::new(3);
        let lines = vec![
            "some text".to_string(),
            "".to_string(),
            "   ".to_string(),
            "more text".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_min_chars_filtering() {
        let ft = UnknownFileType::new(10);
        let lines = vec![
            "short".to_string(),
            "this is a longer line".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }
}
