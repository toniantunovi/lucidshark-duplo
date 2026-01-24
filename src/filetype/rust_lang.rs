//! Rust file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// Rust file type processor
pub struct RustFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl RustFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is a Rust "preprocessor" directive
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("use ")
            || trimmed.starts_with("mod ")
            || trimmed.starts_with("extern ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("pub mod ")
    }

    /// Check if a line is an attribute (#[...])
    fn is_attribute(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("#[") || trimmed.starts_with("#![")
    }

    /// Check if a line starts a function/method signature
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();

        // Skip struct/enum/trait/impl declarations (but not their methods)
        if trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("static ")
        {
            return false;
        }

        // Also skip pub variants of above
        if trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("pub type ")
            || trimmed.starts_with("pub const ")
            || trimmed.starts_with("pub static ")
            || trimmed.starts_with("pub(")
        {
            // Check if it's pub(crate) fn etc.
            if !trimmed.contains(" fn ") {
                return false;
            }
        }

        // Function signatures contain "fn "
        if trimmed.contains("fn ") {
            return true;
        }

        false
    }

    /// Count parentheses and check for opening brace
    fn analyze_line(line: &str) -> (i32, bool) {
        let mut paren_balance = 0;
        let mut has_open_brace = false;
        let mut in_string = false;
        let mut in_char = false;
        let mut in_raw_string = false;

        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_raw_string {
                if c == '"' {
                    in_raw_string = false;
                }
            } else if in_string {
                if c == '"' {
                    in_string = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else if in_char {
                if c == '\'' {
                    in_char = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else {
                match c {
                    'r' if chars.peek() == Some(&'"') => {
                        chars.next();
                        in_raw_string = true;
                    }
                    '"' => in_string = true,
                    '\'' => in_char = true,
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

impl FileType for RustFileType {
    fn name(&self) -> &'static str {
        "Rust"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut comment_depth = 0; // Rust supports nested block comments
        let mut in_signature = false;
        let mut paren_depth: i32 = 0;

        for (line_num, line) in lines.iter().enumerate() {
            let mut cleaned = String::new();
            let mut chars = line.chars().peekable();

            while let Some(c) = chars.next() {
                if in_block_comment {
                    // Check for nested comment start
                    if c == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        comment_depth += 1;
                    }
                    // Check for comment end
                    else if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        comment_depth -= 1;
                        if comment_depth == 0 {
                            in_block_comment = false;
                        }
                    }
                } else {
                    // Check for block comment start
                    if c == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        in_block_comment = true;
                        comment_depth = 1;
                    }
                    // Check for line comment
                    else if c == '/' && chars.peek() == Some(&'/') {
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

            // Handle being inside a multi-line signature
            if in_signature {
                let (balance, has_brace) = Self::analyze_line(&cleaned);
                paren_depth += balance;

                if paren_depth <= 0 && has_brace {
                    in_signature = false;
                    paren_depth = 0;
                }
                continue;
            }

            // Skip attributes when ignore_preprocessor is enabled
            if self.ignore_preprocessor && Self::is_attribute(&cleaned) {
                continue;
            }

            // Check for function signature start
            if self.ignore_preprocessor && Self::starts_signature(&cleaned) {
                let (balance, has_brace) = Self::analyze_line(&cleaned);
                paren_depth = balance;

                if paren_depth <= 0 && has_brace {
                    // Single-line signature
                    paren_depth = 0;
                } else {
                    // Multi-line signature
                    in_signature = true;
                }
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
    fn test_basic_rust() {
        let ft = RustFileType::new(false, 3);
        let lines = vec![
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2); // } is too short
    }

    #[test]
    fn test_comment_removal() {
        let ft = RustFileType::new(false, 3);
        let lines = vec![
            "let x = 5; // comment".to_string(),
            "// full line comment".to_string(),
            "let y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "let x = 5;");
    }

    #[test]
    fn test_nested_block_comment() {
        let ft = RustFileType::new(false, 3);
        let lines = vec![
            "let x = 5;".to_string(),
            "/* outer /* nested */ still comment */".to_string(),
            "let y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_use_filtering() {
        let ft = RustFileType::new(true, 3);
        let lines = vec![
            "use std::io;".to_string(),
            "mod tests;".to_string(),
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // use, mod, and fn signature filtered; only body remains
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "println!(\"Hello\");");
    }

    #[test]
    fn test_function_signature_filtering() {
        let ft = RustFileType::new(true, 3);
        let lines = vec![
            "pub fn process_data(input: &str) -> Result<(), Error> {".to_string(),
            "    let result = parse(input)?;".to_string(),
            "    Ok(())".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("parse(input)")));
    }

    #[test]
    fn test_multiline_signature_filtering() {
        let ft = RustFileType::new(true, 3);
        let lines = vec![
            "#[derive(Debug)]".to_string(),
            "pub fn complex_function(".to_string(),
            "    param1: String,".to_string(),
            "    param2: i32,".to_string(),
            "    param3: Option<bool>,".to_string(),
            ") -> Result<Output, Error> {".to_string(),
            "    let x = do_something(param1);".to_string(),
            "    Ok(x)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Attribute and signature should be filtered
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("do_something")));
    }

    #[test]
    fn test_attribute_filtering() {
        let ft = RustFileType::new(true, 3);
        let lines = vec![
            "#[cfg(test)]".to_string(),
            "#[derive(Clone, Debug)]".to_string(),
            "struct MyStruct {".to_string(),
            "    field: i32,".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Attributes filtered, struct declaration and body remain
        assert!(result.iter().all(|l| !l.line().starts_with("#[")));
    }

    #[test]
    fn test_impl_method_filtering() {
        let ft = RustFileType::new(true, 3);
        let lines = vec![
            "impl MyStruct {".to_string(),
            "    pub fn new(value: i32) -> Self {".to_string(),
            "        Self { field: value }".to_string(),
            "    }".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // impl block stays, method signature filtered
        assert!(result.iter().any(|l| l.line().starts_with("impl")));
        assert!(result.iter().any(|l| l.line().contains("Self { field:")));
    }

    #[test]
    fn test_signature_not_filtered_when_disabled() {
        let ft = RustFileType::new(false, 3);
        let lines = vec![
            "fn hello() {".to_string(),
            "    println!(\"world\");".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "fn hello() {");
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = RustFileType::new(true, 3);
        let lines = vec![
            "if condition {".to_string(),
            "    do_something();".to_string(),
            "}".to_string(),
            "for item in items {".to_string(),
            "    process(item);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().starts_with("for")));
    }
}
