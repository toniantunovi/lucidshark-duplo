//! Rust file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// Rust file type processor
pub struct RustFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl RustFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is a Rust "preprocessor" directive
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("use ")
            || trimmed.starts_with("mod ")
            || trimmed.starts_with("extern ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("pub mod ")
    }
}

impl FileType for RustFileType {
    fn name(&self) -> &'static str {
        "Rust"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut comment_depth = 0; // Rust supports nested block comments

        for (line_num, line) in lines.iter().enumerate() {
            let mut cleaned = String::new();
            let mut chars = line.chars().peekable();

            while let Some(c) = chars.next() {
                if in_block_comment {
                    // Check for nested comment start
                    if c == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        comment_depth += 1;
                    }
                    // Check for comment end
                    else if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        comment_depth -= 1;
                        if comment_depth == 0 {
                            in_block_comment = false;
                        }
                    }
                } else {
                    // Check for block comment start
                    if c == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        in_block_comment = true;
                        comment_depth = 1;
                    }
                    // Check for line comment
                    else if c == '/' && chars.peek() == Some(&'/') {
                        break;
                    } else {
                        cleaned.push(c);
                    }
                }
            }

            let cleaned = clean_whitespace(&cleaned);
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
    fn test_basic_rust() {
        let ft = RustFileType::new(false, 3);
        let lines = vec![
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2); // } is too short
    }

    #[test]
    fn test_comment_removal() {
        let ft = RustFileType::new(false, 3);
        let lines = vec![
            "let x = 5; // comment".to_string(),
            "// full line comment".to_string(),
            "let y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "let x = 5;");
    }

    #[test]
    fn test_nested_block_comment() {
        let ft = RustFileType::new(false, 3);
        let lines = vec![
            "let x = 5;".to_string(),
            "/* outer /* nested */ still comment */".to_string(),
            "let y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_use_filtering() {
        let ft = RustFileType::new(true, 3);
        let lines = vec![
            "use std::io;".to_string(),
            "mod tests;".to_string(),
            "fn main() {".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "fn main() {");
    }
}
