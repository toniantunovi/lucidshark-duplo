//! CSS file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// CSS file type processor
pub struct CssFileType {
    min_chars: u32,
}

impl CssFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a CSS "preprocessor" directive (@import)
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("@import ")
            || trimmed.starts_with("@charset ")
            || trimmed.starts_with("@use ")
            || trimmed.starts_with("@forward ")
    }
}

impl FileType for CssFileType {
    fn name(&self) -> &'static str {
        "CSS"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;

        for (line_num, line) in lines.iter().enumerate() {
            let mut cleaned = String::new();
            let mut chars = line.chars().peekable();

            while let Some(c) = chars.next() {
                if in_block_comment {
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        in_block_comment = false;
                    }
                } else if c == '/' && chars.peek() == Some(&'*') {
                    chars.next();
                    in_block_comment = true;
                } else {
                    cleaned.push(c);
                }
            }

            let cleaned = clean_whitespace(&cleaned);
            if cleaned.is_empty() {
                continue;
            }

            if Self::is_preprocessor_directive(&cleaned) {
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
    fn test_basic_css() {
        let ft = CssFileType::new(3);
        let lines = vec![
            ".container {".to_string(),
            "    width: 100%;".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_comment_removal() {
        let ft = CssFileType::new(3);
        let lines = vec![
            ".class { /* inline comment */ color: red; }".to_string(),
            "/* full line comment */".to_string(),
            ".other { color: blue; }".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_import_filtering() {
        let ft = CssFileType::new(3);
        let lines = vec![
            "@import 'reset.css';".to_string(),
            "@charset \"UTF-8\";".to_string(),
            ".container { width: 100%; }".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }
}
