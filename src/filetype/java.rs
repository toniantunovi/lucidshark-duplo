//! Java file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// Java file type processor
pub struct JavaFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl JavaFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is a Java "preprocessor" directive
    /// (package, import, and access modifiers)
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("package ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("private ")
            || trimmed.starts_with("protected ")
            || trimmed.starts_with("public ")
    }
}

impl FileType for JavaFileType {
    fn name(&self) -> &'static str {
        "Java"
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
    fn test_basic_java() {
        let ft = JavaFileType::new(false, 3);
        let lines = vec![
            "public class Test {".to_string(),
            "    int x = 5;".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2); // } is too short
    }

    #[test]
    fn test_import_filtering() {
        let ft = JavaFileType::new(true, 3);
        let lines = vec![
            "package com.example;".to_string(),
            "import java.util.List;".to_string(),
            "public class Test {".to_string(),
            "    int x = 5;".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // With preprocessor filtering: public class should be filtered too
        assert!(result.iter().all(|l| !l.line().starts_with("package")));
        assert!(result.iter().all(|l| !l.line().starts_with("import")));
    }

    #[test]
    fn test_javadoc_comment() {
        let ft = JavaFileType::new(false, 3);
        let lines = vec![
            "/**".to_string(),
            " * Javadoc comment".to_string(),
            " */".to_string(),
            "public void test() {".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "public void test() {");
    }
}
