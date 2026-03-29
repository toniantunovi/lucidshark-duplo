//! Scala file type implementation

use crate::core::SourceLine;
use crate::filetype::{
    analyze_line_basic, clean_whitespace, is_valid_line, strip_nested_comments, FileType,
    SignatureTracker,
};

/// Scala file type processor
pub struct ScalaFileType {
    min_chars: u32,
}

impl ScalaFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a Scala directive (package, import)
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

        // Skip class/trait/object declarations
        if (trimmed.contains("class ") || trimmed.contains("trait ") || trimmed.contains("object "))
            && !trimmed.contains("def ")
        {
            return false;
        }

        // Must have '(' for a function signature (or just "def name:" for parameterless)
        let Some(paren_pos) = trimmed.find('(') else {
            // Parameterless def like "def name: Type = {"
            if trimmed.starts_with("def ")
                || trimmed.starts_with("override def ")
                || trimmed.starts_with("private def ")
                || trimmed.starts_with("protected def ")
            {
                return trimmed.contains(" = {") || trimmed.contains(": ");
            }
            return false;
        };

        let before_paren = &trimmed[..paren_pos];

        // Method calls have '.' before '('
        if before_paren.contains('.') {
            return false;
        }

        // Exclude control structures
        let control_keywords = ["if", "while", "for", "match", "catch", "try", "else"];
        let first_word = before_paren.split_whitespace().next().unwrap_or("");
        if control_keywords.contains(&first_word) {
            return false;
        }

        // Exclude statements
        let excluded = ["return", "throw", "new", "val", "var"];
        if excluded.contains(&first_word) {
            return false;
        }

        // Exclude assignments
        if trimmed.contains(" = ") && trimmed.find(" = ").unwrap_or(usize::MAX) < paren_pos {
            return false;
        }

        // Function signatures use "def" keyword
        let signature_starters = [
            "def",
            "override",
            "private",
            "protected",
            "final",
            "abstract",
            "implicit",
            "lazy",
        ];

        if signature_starters.contains(&first_word) {
            return true;
        }

        // Check for "def" anywhere in the words before paren
        let words: Vec<&str> = before_paren.split_whitespace().collect();
        words.contains(&"def")
    }
}

impl FileType for ScalaFileType {
    fn name(&self) -> &'static str {
        "Scala"
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
    fn test_basic_scala() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "def greet(): Unit = {".to_string(),
            "    println(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "println(\"Hello\")");
    }

    #[test]
    fn test_comment_removal() {
        let ft = ScalaFileType::new(3);
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
        let ft = ScalaFileType::new(3);
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
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "package com.example".to_string(),
            "import scala.collection.mutable".to_string(),
            "def main(): Unit = {".to_string(),
            "    println(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "println(\"Hello\")");
    }

    #[test]
    fn test_annotation_filtering() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "@deprecated".to_string(),
            "@throws(classOf[Exception])".to_string(),
            "val value = 42".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "val value = 42");
    }

    #[test]
    fn test_method_signature_filtering() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "def processData(input: String): Result = {".to_string(),
            "    val result = parse(input)".to_string(),
            "    result".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("parse")));
    }

    #[test]
    fn test_multiline_signature_filtering() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "@tailrec".to_string(),
            "def handleRequest(".to_string(),
            "    id: String,".to_string(),
            "    body: RequestBody".to_string(),
            "): Response = {".to_string(),
            "    service.process(id, body)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert!(result[0].line().contains("process"));
    }

    #[test]
    fn test_override_def_filtering() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "override def toString(): String = {".to_string(),
            "    s\"MyClass($value)\"".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_scaladoc_comment() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "/**".to_string(),
            " * Scaladoc comment".to_string(),
            " * @param input the input string".to_string(),
            " */".to_string(),
            "def process(input: String): Unit = {".to_string(),
            "    println(input)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "println(input)");
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "if (condition) {".to_string(),
            "    doSomething()".to_string(),
            "}".to_string(),
            "for (item <- items) {".to_string(),
            "    process(item)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().starts_with("for")));
    }

    #[test]
    fn test_implicit_def_filtering() {
        let ft = ScalaFileType::new(3);
        let lines = vec![
            "implicit def intToString(value: Int): String = {".to_string(),
            "    value.toString".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert!(result[0].line().contains("toString"));
    }
}
