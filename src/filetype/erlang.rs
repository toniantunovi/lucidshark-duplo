//! Erlang file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// Erlang file type processor
pub struct ErlangFileType {
    ignore_preprocessor: bool,
    min_chars: u32,
}

impl ErlangFileType {
    pub fn new(ignore_preprocessor: bool, min_chars: u32) -> Self {
        Self {
            ignore_preprocessor,
            min_chars,
        }
    }

    /// Check if a line is an Erlang preprocessor directive
    fn is_preprocessor_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("-module")
            || trimmed.starts_with("-export")
            || trimmed.starts_with("-import")
            || trimmed.starts_with("-include")
            || trimmed.starts_with("-define")
    }

    /// Remove Erlang single-line comments (% style)
    fn remove_comment(line: &str) -> &str {
        if let Some(idx) = line.find('%') {
            &line[..idx]
        } else {
            line
        }
    }
}

impl FileType for ErlangFileType {
    fn name(&self) -> &'static str {
        "Erlang"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let without_comment = Self::remove_comment(line);
            let cleaned = clean_whitespace(without_comment);

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
    fn test_basic_erlang() {
        let ft = ErlangFileType::new(false, 3);
        let lines = vec![
            "hello() -> world.".to_string(),
            "foo(X) -> X + 1.".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_comment_removal() {
        let ft = ErlangFileType::new(false, 3);
        let lines = vec![
            "hello() -> world. % comment".to_string(),
            "% full line comment".to_string(),
            "foo(X) -> X + 1.".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_module_filtering() {
        let ft = ErlangFileType::new(true, 3);
        let lines = vec![
            "-module(test).".to_string(),
            "-export([hello/0]).".to_string(),
            "hello() -> world.".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
    }
}
