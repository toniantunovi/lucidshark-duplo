//! C/C++ file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// C/C++ file type processor
pub struct CFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl CFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is a preprocessor directive
    fn is_preprocessor_directive(line: &str) -> bool {
        line.trim_start().starts_with('#')
    }
}

impl FileType for CFileType {
    fn name(&self) -> &'static str {
        "C/C++"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;

        for (line_num, line) in lines.iter().enumerate() {
            let mut cleaned = String::new();
            let mut chars = line.chars().peekable();

            while let Some(c) = chars.next() {
                if in_block_comment {
                    // Look for end of block comment
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next(); // consume '/'
                        in_block_comment = false;
                    }
                } else {
                    // Check for start of block comment
                    if c == '/' && chars.peek() == Some(&'*') {
                        chars.next(); // consume '*'
                        in_block_comment = true;
                    }
                    // Check for single-line comment
                    else if c == '/' && chars.peek() == Some(&'/') {
                        // Skip rest of line
                        break;
                    } else {
                        cleaned.push(c);
                    }
                }
            }

            // Skip empty lines after comment removal
            let cleaned = clean_whitespace(&cleaned);
            if cleaned.is_empty() {
                continue;
            }

            // Skip preprocessor directives if configured
            if self.ignore_preprocessor && Self::is_preprocessor_directive(&cleaned) {
                continue;
            }

            // Validate and add line
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
        let ft = CFileType::new(false, 3);
        let lines = vec!["int x = 5;".to_string(), "int y = 10;".to_string()];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "int x = 5;");
        assert_eq!(result[0].line_number(), 1);
    }

    #[test]
    fn test_single_line_comment_removal() {
        let ft = CFileType::new(false, 3);
        let lines = vec![
            "int x = 5; // this is a comment".to_string(),
            "// full line comment".to_string(),
            "int y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "int x = 5;");
    }

    #[test]
    fn test_block_comment_removal() {
        let ft = CFileType::new(false, 3);
        let lines = vec![
            "int x /* comment */ = 5;".to_string(),
            "/* start".to_string(),
            "middle".to_string(),
            "end */ int y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "int x  = 5;");
        assert_eq!(result[1].line(), "int y = 10;");
    }

    #[test]
    fn test_preprocessor_filtering() {
        let ft = CFileType::new(true, 3);
        let lines = vec![
            "#include <stdio.h>".to_string(),
            "#define MAX 100".to_string(),
            "int x = 5;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "int x = 5;");
    }

    #[test]
    fn test_preprocessor_kept() {
        let ft = CFileType::new(false, 3);
        let lines = vec!["#include <stdio.h>".to_string(), "int x = 5;".to_string()];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_min_chars_filtering() {
        let ft = CFileType::new(false, 5);
        let lines = vec![
            "int x = 5;".to_string(),
            "x++".to_string(), // too short after filtering
            "int y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }
}
