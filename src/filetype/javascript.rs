//! JavaScript/TypeScript file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// JavaScript/TypeScript file type processor
pub struct JavaScriptFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl JavaScriptFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is a JS/TS "preprocessor" directive (import/export)
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("import ")
            || trimmed.starts_with("export ")
            || trimmed.starts_with("require(")
            || trimmed.starts_with("const ") && trimmed.contains("require(")
    }
}

impl FileType for JavaScriptFileType {
    fn name(&self) -> &'static str {
        "JavaScript/TypeScript"
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
                } else {
                    if c == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        in_block_comment = true;
                    } else if c == '/' && chars.peek() == Some(&'/') {
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
    fn test_basic_javascript() {
        let ft = JavaScriptFileType::new(false, 3);
        let lines = vec![
            "function hello() {".to_string(),
            "    return 'world';".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_comment_removal() {
        let ft = JavaScriptFileType::new(false, 3);
        let lines = vec![
            "const x = 5; // comment".to_string(),
            "// full line comment".to_string(),
            "const y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_import_filtering() {
        let ft = JavaScriptFileType::new(true, 3);
        let lines = vec![
            "import React from 'react';".to_string(),
            "export const foo = 1;".to_string(),
            "function hello() {".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_jsdoc_filtering() {
        let ft = JavaScriptFileType::new(false, 3);
        let lines = vec![
            "/**".to_string(),
            " * JSDoc comment".to_string(),
            " */".to_string(),
            "function hello() {".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }
}
