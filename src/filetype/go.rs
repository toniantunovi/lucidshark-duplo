//! Go file type implementation

use crate::core::SourceLine;
use crate::filetype::{
    clean_whitespace, is_valid_line, strip_c_style_comments, FileType, SignatureTracker,
};

/// Go file type processor
pub struct GoFileType {
    min_chars: u32,
}

impl GoFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a Go import or package declaration
    fn is_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("package ")
            || trimmed.starts_with("import ")
            || trimmed == "import ("
            || trimmed == ")"
    }

    /// Check if a line starts a function/method signature
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();

        // Go functions start with "func "
        if !trimmed.starts_with("func ") {
            return false;
        }

        // Exclude control structures that won't appear after func
        // but include method receivers like "func (r *Receiver) Method("
        true
    }

    /// Count parentheses and check for opening brace
    fn analyze_line(line: &str) -> (i32, bool) {
        let mut paren_balance = 0;
        let mut has_open_brace = false;
        let mut in_string = false;
        let mut in_rune = false;
        let mut in_raw_string = false;

        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_raw_string {
                if c == '`' {
                    in_raw_string = false;
                }
            } else if in_string {
                if c == '"' {
                    in_string = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else if in_rune {
                if c == '\'' {
                    in_rune = false;
                } else if c == '\\' {
                    chars.next();
                }
            } else {
                match c {
                    '`' => in_raw_string = true,
                    '"' => in_string = true,
                    '\'' => in_rune = true,
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

impl FileType for GoFileType {
    fn name(&self) -> &'static str {
        "Go"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut in_import_block = false;
        let mut sig = SignatureTracker::new();

        for (line_num, line) in lines.iter().enumerate() {
            let cleaned = strip_c_style_comments(line, &mut in_block_comment);
            let cleaned = clean_whitespace(&cleaned);
            if cleaned.is_empty() {
                continue;
            }

            // Track import blocks
            if cleaned == "import (" {
                in_import_block = true;
                continue;
            }
            if in_import_block {
                if cleaned == ")" {
                    in_import_block = false;
                }
                continue;
            }

            if sig.in_signature {
                let (balance, has_brace) = Self::analyze_line(&cleaned);
                sig.update(balance, has_brace);
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
    fn test_basic_go() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "func main() {".to_string(),
            "    fmt.Println(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "fmt.Println(\"Hello\")");
    }

    #[test]
    fn test_comment_removal() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "x := 5 // comment".to_string(),
            "// full line comment".to_string(),
            "y := 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "x := 5");
    }

    #[test]
    fn test_block_comment_removal() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "x := 5".to_string(),
            "/* block".to_string(),
            "comment */".to_string(),
            "y := 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_import_block_filtering() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "package main".to_string(),
            "import (".to_string(),
            "    \"fmt\"".to_string(),
            "    \"os\"".to_string(),
            ")".to_string(),
            "func main() {".to_string(),
            "    fmt.Println(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "fmt.Println(\"Hello\")");
    }

    #[test]
    fn test_single_import_filtering() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "package main".to_string(),
            "import \"fmt\"".to_string(),
            "func main() {".to_string(),
            "    fmt.Println(\"Hello\")".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_method_receiver_signature_filtering() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "func (s *Server) HandleRequest(w http.ResponseWriter, r *http.Request) {".to_string(),
            "    data := s.process(r)".to_string(),
            "    w.Write(data)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("s.process")));
    }

    #[test]
    fn test_multiline_signature_filtering() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "func complexFunction(".to_string(),
            "    param1 string,".to_string(),
            "    param2 int,".to_string(),
            ") error {".to_string(),
            "    result := doWork(param1)".to_string(),
            "    return result".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|l| l.line().contains("doWork")));
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = GoFileType::new(3);
        let lines = vec![
            "if err != nil {".to_string(),
            "    return err".to_string(),
            "}".to_string(),
            "for _, item := range items {".to_string(),
            "    process(item)".to_string(),
            "}".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().contains("range")));
    }
}
