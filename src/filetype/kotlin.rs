//! Kotlin file type implementation

use crate::core::SourceLine;
use crate::filetype::{
    analyze_line_basic, clean_whitespace, is_valid_line, strip_nested_comments, FileType,
    SignatureTracker,
};

/// Kotlin file type processor
pub struct KotlinFileType {
    min_chars: u32,
}

impl KotlinFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a Kotlin directive (package, import)
    fn is_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("package ") || trimmed.starts_with("import ")
    }

    /// Check if a line is an annotation (@Something)
    fn is_annotation(line: &str) -> bool {
        line.trim_start().starts_with('@')
    }

    /// Check if a line starts a function/method signature
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();

        // Skip class/interface/object declarations
        if trimmed.contains("class ")
            || trimmed.contains("interface ")
            || trimmed.contains("object ")
            || trimmed.contains("enum ")
        {
            return false;
        }

        // Must have '(' for a function signature
        let Some(paren_pos) = trimmed.find('(') else {
            return false;
        };

        let before_paren = &trimmed[..paren_pos];

        // Method calls have '.' before '('
        if before_paren.contains('.') {
            return false;
        }

        // Exclude control structures
        let control_keywords = ["if", "while", "for", "when", "catch", "try", "else"];
        let first_word = before_paren.split_whitespace().next().unwrap_or("");
        if control_keywords.contains(&first_word) {
            return false;
        }

        // Exclude statements
        let excluded = ["return", "throw", "val", "var"];
        if excluded.contains(&first_word) {
            return false;
        }

        // Exclude assignments
        if trimmed.contains(" = ") {
            return false;
        }

        // Function signatures start with fun or modifiers
        let signature_starters = [
            "fun",
            "suspend",
            "override",
            "open",
            "abstract",
            "private",
            "protected",
            "internal",
            "public",
            "inline",
            "tailrec",
            "operator",
            "infix",
        ];

        if signature_starters.contains(&first_word) {
            return true;
        }

        // Check for "fun" anywhere in the words before paren
        let words: Vec<&str> = before_paren.split_whitespace().collect();
        words.contains(&"fun")
    }
}

impl FileType for KotlinFileType {
    fn name(&self) -> &'static str {
        "Kotlin"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut comment_depth = 0;
        let mut sig = SignatureTracker::new();

        for (line_num, line) in lines.iter().enumerate() {
            let cleaned = strip_nested_comments(line, &mut in_block_comment, &mut comment_depth);
            let cleaned = clean_whitespace(&cleaned);
            if cleaned.is_empty() {
                continue;
            }

            if sig.in_signature {
                let (balance, has_brace) = analyze_line_basic(&cleaned);
                sig.update(balance, has_brace);
                continue;
            }

            if Self::is_annotation(&cleaned) {
                continue;
            }

            if Self::starts_signature(&cleaned) {
                let (balance, has_brace) = analyze_line_basic(&cleaned);
                sig.start(balance, has_brace);
                continue;
            }

            if Self::is_directive(&cleaned) {
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
    fn test_basic_kotlin() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "fun main() {".to_string(),
            "    println(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "println(\"Hello\")");
    }

    #[test]
    fn test_comment_removal() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "val x = 5 // comment".to_string(),
            "// full line comment".to_string(),
            "val y = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "val x = 5");
    }

    #[test]
    fn test_nested_block_comment() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "val x = 5".to_string(),
            "/* outer /* nested */ still comment */".to_string(),
            "val y = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_import_filtering() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "package com.example".to_string(),
            "import kotlin.collections.List".to_string(),
            "fun main() {".to_string(),
            "    println(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "println(\"Hello\")");
    }

    #[test]
    fn test_annotation_filtering() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "@JvmStatic".to_string(),
            "@Deprecated(\"use newMethod\")".to_string(),
            "private val value = 42".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "private val value = 42");
    }

    #[test]
    fn test_suspend_function_filtering() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "suspend fun fetchData(url: String): Response {".to_string(),
            "    val result = client.get(url)".to_string(),
            "    return result".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("client.get")));
    }

    #[test]
    fn test_multiline_signature_filtering() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "@Override".to_string(),
            "fun processRequest(".to_string(),
            "    id: String,".to_string(),
            "    body: RequestBody,".to_string(),
            "): Result {".to_string(),
            "    return service.process(id, body)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "return service.process(id, body)");
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "if (condition) {".to_string(),
            "    doSomething()".to_string(),
            "}".to_string(),
            "for (item in items) {".to_string(),
            "    process(item)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().starts_with("for")));
    }

    #[test]
    fn test_kdoc_comment() {
        let ft = KotlinFileType::new(3);
        let lines = vec![
            "/**".to_string(),
            " * KDoc comment".to_string(),
            " */".to_string(),
            "fun test() {".to_string(),
            "    println(\"test\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "println(\"test\")");
    }
}
