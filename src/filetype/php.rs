//! PHP file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// PHP file type processor
pub struct PhpFileType {
    min_chars: u32,
}

impl PhpFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a PHP directive (use, namespace, require, include)
    fn is_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("use ")
            || trimmed.starts_with("namespace ")
            || trimmed.starts_with("require ")
            || trimmed.starts_with("require_once ")
            || trimmed.starts_with("include ")
            || trimmed.starts_with("include_once ")
    }

    /// Check if a line is a PHP tag
    fn is_php_tag(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed == "<?php" || trimmed == "?>"
    }

    /// Check if a line is an annotation/attribute (#[...] or @Something in docblock)
    fn is_attribute(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("#[")
    }

    /// Check if a line starts a function/method signature
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();

        // Skip class/interface/trait declarations
        if trimmed.contains("class ")
            || trimmed.contains("interface ")
            || trimmed.contains("trait ")
            || trimmed.contains("enum ")
        {
            // But allow "class" in anonymous class expressions inside functions
            if !trimmed.contains("function ") {
                return false;
            }
        }

        // Must have '(' for a function signature
        let Some(paren_pos) = trimmed.find('(') else {
            return false;
        };

        let before_paren = &trimmed[..paren_pos];

        // Method calls have -> or :: before (
        if before_paren.contains("->") || before_paren.contains("::") {
            return false;
        }

        // Exclude control structures
        let control_keywords = [
            "if", "while", "for", "foreach", "switch", "catch", "elseif", "match",
        ];
        let first_word = before_paren.split_whitespace().next().unwrap_or("");
        if control_keywords.contains(&first_word) {
            return false;
        }

        // Exclude statements
        let excluded = ["return", "throw", "echo", "print", "new", "array"];
        if excluded.contains(&first_word) {
            return false;
        }

        // Exclude assignments
        if trimmed.contains(" = ") && trimmed.find(" = ").unwrap_or(usize::MAX) < paren_pos {
            return false;
        }

        // Function signatures contain "function"
        let signature_starters = [
            "function",
            "public",
            "private",
            "protected",
            "static",
            "abstract",
            "final",
        ];

        if signature_starters.contains(&first_word) {
            return true;
        }

        // Check for "function" anywhere before paren
        before_paren.contains("function ")
    }

    /// Count parentheses and check for opening brace
    fn analyze_line(line: &str) -> (i32, bool) {
        let mut paren_balance = 0;
        let mut has_open_brace = false;
        let mut in_string = false;
        let mut in_single_string = false;
        let mut string_char = '"';

        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_string {
                if c == string_char {
                    in_string = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else if in_single_string {
                if c == '\'' {
                    in_single_string = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else {
                match c {
                    '"' => {
                        in_string = true;
                        string_char = '"';
                    }
                    '\'' => in_single_string = true,
                    '(' => paren_balance += 1,
                    ')' => paren_balance -= 1,
                    '{' => has_open_brace = true,
                    '/' if chars.peek() == Some(&'/') => break,
                    '#' => break, // PHP also supports # comments
                    _ => {}
                }
            }
        }

        (paren_balance, has_open_brace)
    }
}

impl FileType for PhpFileType {
    fn name(&self) -> &'static str {
        "PHP"
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
                } else if c == '#' && chars.peek() != Some(&'[') {
                    // # comment (but not #[attribute])
                    break;
                } else {
                    cleaned.push(c);
                }
            }

            let cleaned = clean_whitespace(&cleaned);
            if cleaned.is_empty() {
                continue;
            }

            // Skip PHP tags
            if Self::is_php_tag(&cleaned) {
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

            // Skip attributes
            if Self::is_attribute(&cleaned) {
                continue;
            }

            // Check for function signature start
            if Self::starts_signature(&cleaned) {
                let (balance, has_brace) = Self::analyze_line(&cleaned);
                paren_depth = balance;

                if paren_depth <= 0 && has_brace {
                    paren_depth = 0;
                } else if balance > 0 || !has_brace {
                    in_signature = true;
                }
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
    fn test_basic_php() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "<?php".to_string(),
            "function hello() {".to_string(),
            "    echo 'world';".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "echo 'world';");
    }

    #[test]
    fn test_comment_removal() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "$x = 5; // comment".to_string(),
            "// full line comment".to_string(),
            "# hash comment".to_string(),
            "$y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "$x = 5;");
    }

    #[test]
    fn test_block_comment_removal() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "$x = 5;".to_string(),
            "/* block".to_string(),
            "comment */".to_string(),
            "$y = 10;".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_phpdoc_comment() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "/**".to_string(),
            " * PHPDoc comment".to_string(),
            " * @param string $name".to_string(),
            " */".to_string(),
            "public function greet($name) {".to_string(),
            "    return \"Hello $name\";".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "return \"Hello $name\";");
    }

    #[test]
    fn test_use_namespace_filtering() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "<?php".to_string(),
            "namespace App\\Controllers;".to_string(),
            "use App\\Models\\User;".to_string(),
            "require_once 'config.php';".to_string(),
            "include 'helpers.php';".to_string(),
            "$data = fetchAll();".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "$data = fetchAll();");
    }

    #[test]
    fn test_method_signature_filtering() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "public function processData(string $input): array {".to_string(),
            "    $result = $this->parse($input);".to_string(),
            "    return $result;".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("parse")));
    }

    #[test]
    fn test_multiline_signature_filtering() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "public function handleRequest(".to_string(),
            "    string $id,".to_string(),
            "    array $body,".to_string(),
            "): Response {".to_string(),
            "    return $this->service->process($id, $body);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert!(result[0].line().contains("process"));
    }

    #[test]
    fn test_attribute_filtering() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "#[Route('/api/users')]".to_string(),
            "public function listUsers() {".to_string(),
            "    return User::all();".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().all(|l| !l.line().starts_with("#[")));
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = PhpFileType::new(3);
        let lines = vec![
            "if ($condition) {".to_string(),
            "    doSomething();".to_string(),
            "}".to_string(),
            "foreach ($items as $item) {".to_string(),
            "    process($item);".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().starts_with("foreach")));
    }
}
