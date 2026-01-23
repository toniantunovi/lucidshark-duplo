//! HTML file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// HTML file type processor
pub struct HtmlFileType {
    min_chars: u32,
}

impl HtmlFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }
}

impl FileType for HtmlFileType {
    fn name(&self) -> &'static str {
        "HTML"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_comment = false;

        for (line_num, line) in lines.iter().enumerate() {
            let mut cleaned = String::new();
            let mut i = 0;
            let line_bytes = line.as_bytes();

            while i < line.len() {
                if in_comment {
                    // Look for -->
                    if i + 2 < line.len()
                        && line_bytes[i] == b'-'
                        && line_bytes[i + 1] == b'-'
                        && line_bytes[i + 2] == b'>'
                    {
                        in_comment = false;
                        i += 3;
                        continue;
                    }
                    i += 1;
                } else {
                    // Look for <!--
                    if i + 3 < line.len()
                        && line_bytes[i] == b'<'
                        && line_bytes[i + 1] == b'!'
                        && line_bytes[i + 2] == b'-'
                        && line_bytes[i + 3] == b'-'
                    {
                        in_comment = true;
                        i += 4;
                        continue;
                    }
                    cleaned.push(line_bytes[i] as char);
                    i += 1;
                }
            }

            let cleaned = clean_whitespace(&cleaned);
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
    fn test_basic_html() {
        let ft = HtmlFileType::new(3);
        let lines = vec![
            "<html>".to_string(),
            "<body>".to_string(),
            "<p>Hello World</p>".to_string(),
            "</body>".to_string(),
            "</html>".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.len() >= 1);
    }

    #[test]
    fn test_comment_removal() {
        let ft = HtmlFileType::new(3);
        let lines = vec![
            "<div>content</div>".to_string(),
            "<!-- this is a comment -->".to_string(),
            "<p>more content</p>".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_multiline_comment() {
        let ft = HtmlFileType::new(3);
        let lines = vec![
            "<div>before</div>".to_string(),
            "<!-- start of".to_string(),
            "multiline comment".to_string(),
            "end --> <p>after</p>".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Should have "before" and "after" content
        assert_eq!(result.len(), 2);
    }
}
