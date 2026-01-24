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
            if single_quotes.is_multiple_of(2) && double_quotes.is_multiple_of(2) {
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
            // Handle being inside a multiline string/docstring
            if in_multiline_string {
                if let Some(delim) = multiline_delimiter {
                    if line.contains(delim) {
                        in_multiline_string = false;
                        multiline_delimiter = None;
                    }
                }
                // Skip all lines inside multiline strings
                continue;
            }

            // Check for triple-quoted strings anywhere in the line (not just at start)
            // This handles cases like: def foo(): """docstring starts here
            let triple_double = line.find("\"\"\"");
            let triple_single = line.find("'''");

            let docstring_start: Option<(&str, usize)> = match (triple_double, triple_single) {
                (Some(d), Some(s)) => Some(if d < s { ("\"\"\"", d) } else { ("'''", s) }),
                (Some(d), None) => Some(("\"\"\"", d)),
                (None, Some(s)) => Some(("'''", s)),
                (None, None) => None,
            };

            if let Some((delim, start_idx)) = docstring_start {
                let after_delim = &line[start_idx + 3..];
                if after_delim.contains(delim) {
                    // Single-line docstring (e.g., """short docstring""")
                    // Process the code before the docstring, skip the docstring itself
                    let before_docstring = &line[..start_idx];
                    let without_comment = Self::remove_comment(before_docstring);
                    let cleaned = clean_whitespace(without_comment);

                    if !cleaned.is_empty()
                        && is_valid_line(&cleaned, self.min_chars)
                        && !(self.ignore_preprocessor && Self::is_preprocessor_directive(&cleaned))
                    {
                        result.push(SourceLine::new(cleaned, line_num + 1));
                    }
                    continue;
                } else {
                    // Start of multiline docstring
                    in_multiline_string = true;
                    multiline_delimiter = Some(delim);

                    // Process code BEFORE the docstring (e.g., "def foo():" in "def foo(): """doc")
                    let before_docstring = &line[..start_idx];
                    let without_comment = Self::remove_comment(before_docstring);
                    let cleaned = clean_whitespace(without_comment);

                    if !cleaned.is_empty()
                        && is_valid_line(&cleaned, self.min_chars)
                        && !(self.ignore_preprocessor && Self::is_preprocessor_directive(&cleaned))
                    {
                        result.push(SourceLine::new(cleaned, line_num + 1));
                    }
                    continue;
                }
            }

            // No docstring on this line - process normally
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

    #[test]
    fn test_multiline_docstring_with_content_on_first_line() {
        // This is the common pattern: """Docstring starts here.
        //     More content.
        //     Args:
        //         param: description
        //     """
        let ft = PythonFileType::new(false, 3);
        let lines = vec![
            "def run_scan(self, context):".to_string(),
            "    \"\"\"Run duplication detection on the entire project.".to_string(),
            "".to_string(),
            "    Note: Always scans the entire project.".to_string(),
            "".to_string(),
            "    Args:".to_string(),
            "        context: Scan context with project root.".to_string(),
            "        threshold: Maximum allowed duplication percentage.".to_string(),
            "        min_lines: Minimum lines for a duplicate block.".to_string(),
            "".to_string(),
            "    Returns:".to_string(),
            "        DuplicationResult with statistics and issues.".to_string(),
            "    \"\"\"".to_string(),
            "    return self.scan(context)".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Should only have the def line and return line, all docstring content filtered
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "def run_scan(self, context):");
        assert_eq!(result[1].line(), "return self.scan(context)");
    }

    #[test]
    fn test_docstring_on_same_line_as_def() {
        // Pattern: def foo(): """docstring starts here
        let ft = PythonFileType::new(false, 3);
        let lines = vec![
            "def foo(): \"\"\"This is a docstring.".to_string(),
            "    More docstring content.".to_string(),
            "    \"\"\"".to_string(),
            "    return 42".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Should have "def foo():" and "return 42"
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "def foo():");
        assert_eq!(result[1].line(), "return 42");
    }

    #[test]
    fn test_single_quote_docstring() {
        let ft = PythonFileType::new(false, 3);
        let lines = vec![
            "def hello():".to_string(),
            "    '''Single quote docstring.".to_string(),
            "    With multiple lines.".to_string(),
            "    '''".to_string(),
            "    return 'world'".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "def hello():");
        assert_eq!(result[1].line(), "return 'world'");
    }
}
