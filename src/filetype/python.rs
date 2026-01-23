//! Python file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// Python file type processor
pub struct PythonFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl PythonFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is a Python "preprocessor" directive (import/from)
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("import ") || trimmed.starts_with("from ")
    }

    /// Remove Python single-line comments (# style)
    fn remove_comment(line: &str) -> &str {
        // Simple approach - find # not inside a string
        // This is simplified and may not handle all edge cases
        if let Some(idx) = line.find('#') {
            let before = &line[..idx];
            // Count quotes to check if # is inside a string (simplified)
            let single_quotes = before.matches('\'').count();
            let double_quotes = before.matches('"').count();
            if single_quotes % 2 == 0 && double_quotes % 2 == 0 {
                return &line[..idx];
            }
        }
        line
    }
}

impl FileType for PythonFileType {
    fn name(&self) -> &'static str {
        "Python"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_multiline_string = false;
        let mut multiline_delimiter: Option<&str> = None;

        for (line_num, line) in lines.iter().enumerate() {
            // Handle triple-quoted strings (simplified docstring handling)
            if in_multiline_string {
                if let Some(delim) = multiline_delimiter {
                    if line.contains(delim) {
                        in_multiline_string = false;
                        multiline_delimiter = None;
                        // Skip this line as it's part of a docstring
                        continue;
                    }
                }
                continue;
            }

            // Check for start of triple-quoted string
            let trimmed = line.trim_start();
            if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                let delim = if trimmed.starts_with("\"\"\"") {
                    "\"\"\""
                } else {
                    "'''"
                };
                // Check if it ends on the same line
                if trimmed[3..].contains(delim) {
                    // Single-line docstring - skip
                    continue;
                } else {
                    in_multiline_string = true;
                    multiline_delimiter = Some(delim);
                    continue;
                }
            }

            // Remove single-line comments
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
    fn test_basic_python() {
        let ft = PythonFileType::new(false, 3);
        let lines = vec!["def hello():".to_string(), "    return 'world'".to_string()];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_comment_removal() {
        let ft = PythonFileType::new(false, 3);
        let lines = vec![
            "x = 5  # this is a comment".to_string(),
            "# full line comment".to_string(),
            "y = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "x = 5");
    }

    #[test]
    fn test_import_filtering() {
        let ft = PythonFileType::new(true, 3);
        let lines = vec![
            "import os".to_string(),
            "from typing import List".to_string(),
            "def hello():".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "def hello():");
    }

    #[test]
    fn test_docstring_filtering() {
        let ft = PythonFileType::new(false, 3);
        let lines = vec![
            "def hello():".to_string(),
            "    \"\"\"This is a docstring.\"\"\"".to_string(),
            "    return 'world'".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }
}
