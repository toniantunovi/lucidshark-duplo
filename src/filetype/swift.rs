//! Swift file type implementation

use crate::core::SourceLine;
use crate::filetype::{
    clean_whitespace, is_valid_line, strip_nested_comments, FileType, SignatureTracker,
};

/// Swift file type processor
pub struct SwiftFileType {
    min_chars: u32,
}

impl SwiftFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a Swift directive (import)
    fn is_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("import ")
    }

    /// Check if a line is an attribute (@Something)
    fn is_attribute(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with('@')
    }

    /// Check if a line starts a function/method signature
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();

        // Skip class/struct/enum/protocol declarations
        if (trimmed.contains("class ")
            || trimmed.contains("struct ")
            || trimmed.contains("enum ")
            || trimmed.contains("protocol ")
            || trimmed.contains("extension "))
            && !trimmed.contains("func ")
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
        let control_keywords = ["if", "while", "for", "switch", "catch", "guard", "else"];
        let first_word = before_paren.split_whitespace().next().unwrap_or("");
        if control_keywords.contains(&first_word) {
            return false;
        }

        // Exclude statements
        let excluded = ["return", "throw", "try", "let", "var"];
        if excluded.contains(&first_word) {
            return false;
        }

        // Exclude assignments
        if trimmed.contains(" = ") && trimmed.find(" = ").unwrap_or(usize::MAX) < paren_pos {
            return false;
        }

        // Function signatures contain "func" or start with modifiers
        let signature_starters = [
            "func",
            "public",
            "private",
            "internal",
            "fileprivate",
            "open",
            "override",
            "static",
            "class",
            "mutating",
            "nonmutating",
            "init",
            "deinit",
            "subscript",
        ];

        if signature_starters.contains(&first_word) {
            return true;
        }

        // Check for "func" or "init" anywhere in the words before paren
        let words: Vec<&str> = before_paren.split_whitespace().collect();
        words.iter().any(|w| *w == "func" || *w == "init")
    }

    /// Count parentheses and check for opening brace
    fn analyze_line(line: &str) -> (i32, bool) {
        let mut paren_balance = 0;
        let mut has_open_brace = false;
        let mut in_string = false;

        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_string {
                if c == '"' {
                    in_string = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else {
                match c {
                    '"' => in_string = true,
                    '(' => paren_balance += 1,
                    ')' => paren_balance -= 1,
                    '{' => has_open_brace = true,
                    '/' if chars.peek() == Some(&'/') => break,
                    _ => {}
                }
            }
        }

        (paren_balance, has_open_brace)
    }
}

impl FileType for SwiftFileType {
    fn name(&self) -> &'static str {
        "Swift"
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
                let (balance, has_brace) = Self::analyze_line(&cleaned);
                sig.update(balance, has_brace);
                continue;
            }

            if Self::is_attribute(&cleaned) {
                continue;
            }

            if Self::starts_signature(&cleaned) {
                let (balance, has_brace) = Self::analyze_line(&cleaned);
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
    fn test_basic_swift() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "func greet() {".to_string(),
            "    print(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "print(\"Hello\")");
    }

    #[test]
    fn test_comment_removal() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "let x = 5 // comment".to_string(),
            "// full line comment".to_string(),
            "let y = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "let x = 5");
    }

    #[test]
    fn test_nested_block_comment() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "let x = 5".to_string(),
            "/* outer /* nested */ still comment */".to_string(),
            "let y = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_import_filtering() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "import Foundation".to_string(),
            "import UIKit".to_string(),
            "func main() {".to_string(),
            "    print(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "print(\"Hello\")");
    }

    #[test]
    fn test_attribute_filtering() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "@available(iOS 15, *)".to_string(),
            "@MainActor".to_string(),
            "let value = 42".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "let value = 42");
    }

    #[test]
    fn test_method_signature_filtering() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "public func processData(input: String) -> Result<Data, Error> {".to_string(),
            "    let result = parse(input)".to_string(),
            "    return result".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("parse")));
    }

    #[test]
    fn test_multiline_signature_filtering() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "@objc".to_string(),
            "func handleRequest(".to_string(),
            "    id: String,".to_string(),
            "    body: RequestBody".to_string(),
            ") -> Response {".to_string(),
            "    return service.process(id, body)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert!(result[0].line().contains("process"));
    }

    #[test]
    fn test_init_signature_filtering() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "init(value: Int) {".to_string(),
            "    self.value = value".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert!(result[0].line().contains("self.value"));
    }

    #[test]
    fn test_guard_not_filtered() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "guard let value = optional else {".to_string(),
            "    return nil".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("guard")));
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = SwiftFileType::new(3);
        let lines = vec![
            "if condition {".to_string(),
            "    doSomething()".to_string(),
            "}".to_string(),
            "for item in items {".to_string(),
            "    process(item)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().starts_with("for")));
    }
}
