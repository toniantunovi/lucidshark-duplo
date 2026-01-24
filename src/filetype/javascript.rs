//! JavaScript/TypeScript file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// JavaScript/TypeScript file type processor
pub struct JavaScriptFileType {
    min_chars: u32,
}

impl JavaScriptFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a JS/TS "preprocessor" directive (import/export)
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("import ")
            || trimmed.starts_with("export ")
            || trimmed.starts_with("require(")
            || trimmed.starts_with("const ") && trimmed.contains("require(")
    }

    /// Check if a line is a TypeScript decorator (@something)
    fn is_decorator(line: &str) -> bool {
        line.trim_start().starts_with('@')
    }

    /// Check if a line starts a function/method signature
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();

        // Skip class/interface declarations
        if trimmed.starts_with("class ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("enum ")
        {
            return false;
        }

        // Function declarations: "function name(" or "async function name("
        if trimmed.starts_with("function ") || trimmed.starts_with("async function ") {
            return true;
        }

        // Must have '(' to be a method signature
        let Some(paren_pos) = trimmed.find('(') else {
            return false;
        };

        let before_paren = &trimmed[..paren_pos];

        // Method calls have '.' before '(' - exclude them
        if before_paren.contains('.') {
            return false;
        }

        // Exclude control structures
        let control_keywords = ["if", "while", "for", "switch", "catch", "with"];
        let first_word = before_paren.split_whitespace().next().unwrap_or("");
        if control_keywords.contains(&first_word) {
            return false;
        }

        // Exclude statements and expressions
        let excluded = [
            "return", "throw", "new", "await", "yield", "typeof", "delete",
        ];
        if excluded.contains(&first_word) {
            return false;
        }

        // Exclude variable declarations with function calls
        if trimmed.starts_with("const ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("var ")
        {
            return false;
        }

        // Arrow functions assigned to variables are excluded above
        // But class methods look like: "methodName(" or "async methodName("
        // or with modifiers: "public methodName(", "private async methodName("

        // TypeScript modifiers that indicate method signatures
        let ts_modifiers = [
            "public",
            "private",
            "protected",
            "static",
            "readonly",
            "abstract",
            "override",
            "async",
        ];

        if ts_modifiers.contains(&first_word) {
            return true;
        }

        // Check if it looks like a method: "identifier(" with optional "async" prefix
        // But not a standalone function call
        let words: Vec<&str> = before_paren.split_whitespace().collect();

        // "async methodName" -> 2 words, second is method name
        // "methodName" -> 1 word, it's the method name (but could be a call)
        // For single word, check if line ends with '{' or has type annotation
        if words.len() == 1 || (words.len() == 2 && words[0] == "async") {
            // If line contains '{' or '):' (type annotation), likely a method definition
            if trimmed.contains('{') || trimmed.contains("):") || trimmed.contains("): ") {
                return true;
            }
            // Multi-line signature: ends with '(' or has unclosed parens
            if trimmed.ends_with('(') {
                return true;
            }
        }

        // TypeScript: "methodName(params): ReturnType" pattern
        if !words.is_empty() && (trimmed.contains("): ") || trimmed.contains("):")) {
            return true;
        }

        false
    }

    /// Count parentheses and braces, returns (paren_balance, has_open_brace, has_arrow)
    fn analyze_line(line: &str) -> (i32, bool, bool) {
        let mut paren_balance = 0;
        let mut has_open_brace = false;
        let mut has_arrow = false;
        let mut in_string = false;
        let mut string_char = ' ';
        let mut in_template = false;

        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_template {
                if c == '`' {
                    in_template = false;
                }
            } else if in_string {
                if c == string_char {
                    in_string = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else {
                match c {
                    '"' | '\'' => {
                        in_string = true;
                        string_char = c;
                    }
                    '`' => in_template = true,
                    '(' => paren_balance += 1,
                    ')' => paren_balance -= 1,
                    '{' => has_open_brace = true,
                    '=' if chars.peek() == Some(&'>') => {
                        chars.next();
                        has_arrow = true;
                    }
                    '/' if chars.peek() == Some(&'/') => break,
                    _ => {}
                }
            }
        }

        (paren_balance, has_open_brace, has_arrow)
    }
}

impl FileType for JavaScriptFileType {
    fn name(&self) -> &'static str {
        "JavaScript/TypeScript"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut in_signature = false;
        let mut paren_depth: i32 = 0;

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

            // Handle being inside a multi-line signature
            if in_signature {
                let (balance, has_brace, has_arrow) = Self::analyze_line(&cleaned);
                paren_depth += balance;

                // Signature ends when parens balanced and we see '{' or '=>'
                if paren_depth <= 0 && (has_brace || has_arrow) {
                    in_signature = false;
                    paren_depth = 0;
                }
                continue;
            }

            // Skip decorators
            if Self::is_decorator(&cleaned) {
                continue;
            }

            // Check for function/method signature start
            if Self::starts_signature(&cleaned) {
                let (balance, has_brace, has_arrow) = Self::analyze_line(&cleaned);
                paren_depth = balance;

                if paren_depth <= 0 && (has_brace || has_arrow) {
                    // Single-line signature
                    paren_depth = 0;
                } else {
                    // Multi-line signature
                    in_signature = true;
                }
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
    fn test_basic_javascript() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "function hello() {".to_string(),
            "    return 'world';".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Signature filtered, only body remains
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "return 'world';");
    }

    #[test]
    fn test_comment_removal() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "const x = 5; // comment".to_string(),
            "// full line comment".to_string(),
            "const y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_import_filtering() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "import React from 'react';".to_string(),
            "export const foo = 1;".to_string(),
            "function hello() {".to_string(),
            "    return 'world';".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Imports, exports, and function signature filtered; only body remains
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "return 'world';");
    }

    #[test]
    fn test_jsdoc_filtering() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "/**".to_string(),
            " * JSDoc comment".to_string(),
            " */".to_string(),
            "function hello() {".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        // Block comment and signature filtered
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_function_signature_filtering() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "function processData(input) {".to_string(),
            "    return input.map(x => x * 2);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "return input.map(x => x * 2);");
    }

    #[test]
    fn test_async_function_signature_filtering() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "async function fetchData(url) {".to_string(),
            "    const response = await fetch(url);".to_string(),
            "    return response.json();".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("fetch(url)")));
    }

    #[test]
    fn test_multiline_typescript_signature_filtering() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "@Decorator()".to_string(),
            "async processRequest(".to_string(),
            "    id: string,".to_string(),
            "    body: RequestBody,".to_string(),
            "): Promise<Result> {".to_string(),
            "    return await this.service.process(id, body);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].line(),
            "return await this.service.process(id, body);"
        );
    }

    #[test]
    fn test_class_method_signature_filtering() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "public getValue(): number {".to_string(),
            "    return this._value;".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "return this._value;");
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "if (condition) {".to_string(),
            "    doSomething();".to_string(),
            "}".to_string(),
            "for (const item of items) {".to_string(),
            "    process(item);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().starts_with("for")));
    }

    #[test]
    fn test_decorator_filtering() {
        let ft = JavaScriptFileType::new(3);
        let lines = vec![
            "@Injectable()".to_string(),
            "@Autowired".to_string(),
            "private service: Service;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "private service: Service;");
    }
}
