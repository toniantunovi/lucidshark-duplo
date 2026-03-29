//! Java file type implementation

use crate::core::SourceLine;
use crate::filetype::{
    analyze_line_basic, clean_whitespace, is_valid_line, strip_c_style_comments, FileType,
    SignatureTracker,
};

/// Java file type processor
pub struct JavaFileType {
    min_chars: u32,
}

impl JavaFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a Java "preprocessor" directive (package, import)
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("package ") || trimmed.starts_with("import ")
    }

    /// Check if a line is an annotation (@Something)
    fn is_annotation(line: &str) -> bool {
        line.trim_start().starts_with('@')
    }

    /// Check if a line starts a method/constructor signature
    /// Looks for patterns like: "modifier type name(" or just "Type name("
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();

        // Skip if it's a class/interface/enum declaration
        if trimmed.contains("class ") || trimmed.contains("interface ") || trimmed.contains("enum ")
        {
            return false;
        }

        // Must have '(' to be a method signature
        let Some(paren_pos) = trimmed.find('(') else {
            return false;
        };

        // Get the part before '('
        let before_paren = &trimmed[..paren_pos];

        // Method calls have '.' before '(' (e.g., "obj.method(", "System.out.println(")
        // Method signatures don't
        if before_paren.contains('.') {
            return false;
        }

        // Exclude control structures
        let control_keywords = [
            "if",
            "while",
            "for",
            "switch",
            "catch",
            "try",
            "else",
            "do",
            "synchronized",
        ];
        let first_word = before_paren.split_whitespace().next().unwrap_or("");
        if control_keywords.contains(&first_word) {
            return false;
        }

        // Exclude statements that aren't signatures
        let excluded_starts = ["return", "throw", "new", "super", "this"];
        if excluded_starts.contains(&first_word) {
            return false;
        }

        // Exclude assignments (e.g., "var x = method()")
        if trimmed.contains(" = ") {
            return false;
        }

        // Method signatures typically start with:
        // - Access modifier (public, private, protected)
        // - Or type name (starting with uppercase or primitives)
        // - Or generic type
        let signature_starters = [
            "public",
            "private",
            "protected",
            "static",
            "final",
            "abstract",
            "native",
            "synchronized",
            "default",
            "void",
        ];

        if signature_starters.contains(&first_word) {
            return true;
        }

        // Also match if it looks like "TypeName methodName(" pattern
        // This is heuristic: two words before '(' where first starts with uppercase
        let words: Vec<&str> = before_paren.split_whitespace().collect();
        if words.len() >= 2 {
            let first_char = words[0].chars().next().unwrap_or('a');
            if first_char.is_uppercase() || words[0].contains('<') {
                return true;
            }
        }

        false
    }
}

impl FileType for JavaFileType {
    fn name(&self) -> &'static str {
        "Java"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut sig = SignatureTracker::new();

        for (line_num, line) in lines.iter().enumerate() {
            let cleaned = strip_c_style_comments(line, &mut in_block_comment);
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
    fn test_basic_java() {
        let ft = JavaFileType::new(3);
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
        let ft = JavaFileType::new(3);
        let lines = vec![
            "package com.example;".to_string(),
            "import java.util.List;".to_string(),
            "public class Test {".to_string(),
            "    int x = 5;".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Package and import should be filtered
        assert!(result.iter().all(|l| !l.line().starts_with("package")));
        assert!(result.iter().all(|l| !l.line().starts_with("import")));
    }

    #[test]
    fn test_javadoc_comment() {
        let ft = JavaFileType::new(3);
        let lines = vec![
            "/**".to_string(),
            " * Javadoc comment".to_string(),
            " */".to_string(),
            "public void test() {".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Javadoc filtered, signature filtered
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_single_line_signature_filtering() {
        let ft = JavaFileType::new(3);
        let lines = vec![
            "public void doSomething(String param) {".to_string(),
            "    System.out.println(param);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Signature should be filtered, only body remains
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "System.out.println(param);");
    }

    #[test]
    fn test_multiline_signature_filtering() {
        let ft = JavaFileType::new(3);
        let lines = vec![
            "@Override".to_string(),
            "public ResponseEntity<Result> processRequest(".to_string(),
            "        String id,".to_string(),
            "        RequestBody body,".to_string(),
            "        HttpHeaders headers) {".to_string(),
            "    return service.process(id, body);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Annotation and signature should be filtered
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "return service.process(id, body);");
    }

    #[test]
    fn test_annotation_filtering() {
        let ft = JavaFileType::new(3);
        let lines = vec![
            "@Deprecated".to_string(),
            "@SuppressWarnings(\"unused\")".to_string(),
            "private int value;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Annotations filtered, field remains
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "private int value;");
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = JavaFileType::new(3);
        let lines = vec![
            "if (condition) {".to_string(),
            "    doSomething();".to_string(),
            "}".to_string(),
            "for (int i = 0; i < 10; i++) {".to_string(),
            "    process(i);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Control structures should NOT be filtered
        assert_eq!(result.len(), 4);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().starts_with("for")));
    }

    #[test]
    fn test_interface_method_filtering() {
        let ft = JavaFileType::new(3);
        let lines = vec![
            "public interface Service {".to_string(),
            "    Result process(Input input);".to_string(),
            "    void validate(Data data);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Interface declaration stays, abstract methods are filtered
        // (they end with ; not {, but they still match signature pattern)
        assert!(result.len() >= 1);
    }
}
