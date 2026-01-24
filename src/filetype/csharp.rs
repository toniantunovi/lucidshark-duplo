//! C# file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// C# file type processor
pub struct CSharpFileType {
    min_chars: u32,
}

impl CSharpFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a C# preprocessor directive
    fn is_preprocessor_directive(line: &str) -> bool {
        line.trim_start().starts_with('#')
    }
}

impl FileType for CSharpFileType {
    fn name(&self) -> &'static str {
        "C#"
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
                } else if c == '/' && chars.peek() == Some(&'/') {
                    break;
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
    fn test_basic_csharp() {
        let ft = CSharpFileType::new(3);
        let lines = vec![
            "public class Test {".to_string(),
            "    int x = 5;".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_preprocessor_filtering() {
        let ft = CSharpFileType::new(3);
        let lines = vec![
            "#region MyRegion".to_string(),
            "int x = 5;".to_string(),
            "#endregion".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "int x = 5;");
    }
}
