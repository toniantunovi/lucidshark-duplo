//! Ruby file type implementation

use crate::core::SourceLine;
use crate::filetype::{clean_whitespace, is_valid_line, FileType};

/// Ruby file type processor
pub struct RubyFileType {
    min_chars: u32,
}

impl RubyFileType {
    pub fn new(min_chars: u32) -> Self {
        Self { min_chars }
    }

    /// Check if a line is a Ruby directive (require, require_relative, load, include)
    fn is_directive(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("require ")
            || trimmed.starts_with("require_relative ")
            || trimmed.starts_with("load ")
            || trimmed.starts_with("include ")
            || trimmed.starts_with("extend ")
            || trimmed.starts_with("prepend ")
    }

    /// Check if a line starts a method signature
    fn starts_signature(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("def ") || trimmed.starts_with("def self.")
    }

    /// Remove Ruby single-line comments (# style)
    fn remove_comment(line: &str) -> &str {
        let mut in_single_string = false;
        let mut in_double_string = false;
        let bytes = line.as_bytes();

        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            if in_double_string {
                if c == b'"' {
                    in_double_string = false;
                } else if c == b'\\' {
                    i += 1; // skip escaped char
                }
            } else if in_single_string {
                if c == b'\'' {
                    in_single_string = false;
                } else if c == b'\\' {
                    i += 1;
                }
            } else if c == b'"' {
                in_double_string = true;
            } else if c == b'\'' {
                in_single_string = true;
            } else if c == b'#' {
                return &line[..i];
            }
            i += 1;
        }

        line
    }
}

impl FileType for RubyFileType {
    fn name(&self) -> &'static str {
        "Ruby"
    }

    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut in_heredoc = false;
        let mut heredoc_delimiter: Option<String> = None;

        for (line_num, line) in lines.iter().enumerate() {
            // Handle =begin/=end block comments
            let trimmed = line.trim();
            if trimmed == "=begin" {
                in_block_comment = true;
                continue;
            }
            if in_block_comment {
                if trimmed == "=end" {
                    in_block_comment = false;
                }
                continue;
            }

            // Handle heredoc
            if in_heredoc {
                if let Some(ref delim) = heredoc_delimiter {
                    if trimmed == delim.as_str() {
                        in_heredoc = false;
                        heredoc_delimiter = None;
                    }
                }
                continue;
            }

            // Check for heredoc start (<<~IDENTIFIER or <<-IDENTIFIER or <<IDENTIFIER)
            if let Some(pos) = line.find("<<") {
                let after = &line[pos + 2..];
                let after = after.trim_start_matches(['~', '-']);
                let delim: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !delim.is_empty() && after.starts_with(delim.as_str()) {
                    in_heredoc = true;
                    heredoc_delimiter = Some(delim);
                    // Process the part before the heredoc marker
                    let before = &line[..pos];
                    let without_comment = Self::remove_comment(before);
                    let cleaned = clean_whitespace(without_comment);
                    if !cleaned.is_empty()
                        && !Self::is_directive(&cleaned)
                        && !Self::starts_signature(&cleaned)
                        && is_valid_line(&cleaned, self.min_chars)
                    {
                        result.push(SourceLine::new(cleaned, line_num + 1));
                    }
                    continue;
                }
            }

            let without_comment = Self::remove_comment(line);
            let cleaned = clean_whitespace(without_comment);

            if cleaned.is_empty() {
                continue;
            }

            // Skip method signatures
            if Self::starts_signature(&cleaned) {
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
    fn test_basic_ruby() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "def hello".to_string(),
            "    puts 'world'".to_string(),
            "end".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "puts 'world'");
        assert_eq!(result[1].line(), "end");
    }

    #[test]
    fn test_comment_removal() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "x = 5 # comment".to_string(),
            "# full line comment".to_string(),
            "y = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "x = 5");
    }

    #[test]
    fn test_block_comment_removal() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "x = 5".to_string(),
            "=begin".to_string(),
            "This is a block comment".to_string(),
            "spanning multiple lines".to_string(),
            "=end".to_string(),
            "y = 10".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].line(), "x = 5");
        assert_eq!(result[1].line(), "y = 10");
    }

    #[test]
    fn test_require_filtering() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "require 'json'".to_string(),
            "require_relative 'helper'".to_string(),
            "load 'config.rb'".to_string(),
            "puts 'hello'".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "puts 'hello'");
    }

    #[test]
    fn test_method_signature_filtering() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "def process_data(input)".to_string(),
            "    result = parse(input)".to_string(),
            "    result.to_s".to_string(),
            "end".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|l| !l.line().starts_with("def ")));
    }

    #[test]
    fn test_class_method_signature_filtering() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "def self.create(attrs)".to_string(),
            "    new(attrs).tap(&:save)".to_string(),
            "end".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().all(|l| !l.line().starts_with("def ")));
    }

    #[test]
    fn test_include_extend_filtering() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "include Comparable".to_string(),
            "extend ClassMethods".to_string(),
            "prepend Validation".to_string(),
            "attr_accessor :name".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line(), "attr_accessor :name");
    }

    #[test]
    fn test_control_structures_not_filtered() {
        let ft = RubyFileType::new(3);
        let lines = vec![
            "if condition".to_string(),
            "    do_something".to_string(),
            "end".to_string(),
            "items.each do |item|".to_string(),
            "    process(item)".to_string(),
            "end".to_string(),
        ];
        let result = ft.get_cleaned_source_lines(&lines);
        assert!(result.iter().any(|l| l.line().starts_with("if")));
        assert!(result.iter().any(|l| l.line().contains("each")));
    }
}
