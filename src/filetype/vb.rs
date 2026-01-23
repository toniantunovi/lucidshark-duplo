//! VB.NET file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// VB.NET file type processor
pub struct VbFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl VbFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is a VB preprocessor directive
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("Imports ") || trimmed.starts_with('#')
    }

    /// Remove VB single-line comments (' style)
    fn remove_comment(line: &str) -> &str {
        // VB uses ' for comments
        if let Some(idx) = line.find('\'') {
            &line[..idx]
        } else {
            line
        }
    }
}

impl FileType for VbFileType {
    fn name(&self) -> &'static str {
        "VB.NET"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            // Remove VB comments
            let without_comment = Self::remove_comment(line);
            let cleaned = clean_whitespace(without_comment);

            if cleaned.is_empty() {
                continue;
            }

            if self.ignore_preprocessor && Self::is_preprocessor_directive(&cleaned) {
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
    fn test_basic_vb() {
        let ft = VbFileType::new(false, 3);
        let lines = vec![
            "Dim x As Integer = 5".to_string(),
            "Dim y As Integer = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_comment_removal() {
        let ft = VbFileType::new(false, 3);
        let lines = vec![
            "Dim x As Integer = 5 ' this is a comment".to_string(),
            "' full line comment".to_string(),
            "Dim y As Integer = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "Dim x As Integer = 5");
    }

    #[test]
    fn test_imports_filtering() {
        let ft = VbFileType::new(true, 3);
        let lines = vec![
            "Imports System".to_string(),
            "Dim x As Integer = 5".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }
}
