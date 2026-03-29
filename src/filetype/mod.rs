//! File type system for language-specific source processing
//!
//! This module provides a trait-based system for handling different programming
//! languages. Each language has specific rules for:
//! - Block comment removal (e.g., /* */ for C)
//! - Single-line comment removal (e.g., // for C)
//! - Preprocessor directive handling (e.g., #include for C)

mod c;
mod csharp;
mod css;
mod erlang;
mod go;
mod html;
mod java;
mod javascript;
mod kotlin;
mod php;
mod python;
mod ruby;
mod rust_lang;
mod scala;
mod swift;
mod unknown;
mod vb;

use crate::core::SourceLine;

pub use c::CFileType;
pub use csharp::CSharpFileType;
pub use css::CssFileType;
pub use erlang::ErlangFileType;
pub use go::GoFileType;
pub use html::HtmlFileType;
pub use java::JavaFileType;
pub use javascript::JavaScriptFileType;
pub use kotlin::KotlinFileType;
pub use php::PhpFileType;
pub use python::PythonFileType;
pub use ruby::RubyFileType;
pub use rust_lang::RustFileType;
pub use scala::ScalaFileType;
pub use swift::SwiftFileType;
pub use unknown::UnknownFileType;
pub use vb::VbFileType;

/// Trait for language-specific source file processing
///
/// Implementations handle comment removal, preprocessor filtering,
/// and line validation specific to each programming language.
pub trait FileType: Send + Sync {
    /// Get the name of this file type (used in tests and for debugging)
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Process raw file lines and return cleaned source lines
    ///
    /// This method:
    /// 1. Removes block comments (statefully tracking comment state)
    /// 2. Removes single-line comments
    /// 3. Optionally removes preprocessor directives
    /// 4. Filters out lines that are too short or have no alphabetic chars
    /// 5. Creates SourceLine objects with hashes for remaining lines
    fn get_cleaned_source_lines(&self, lines: &[String]) -> Vec<SourceLine>;
}

/// Create a FileType implementation based on file extension
///
/// # Arguments
/// * `filename` - The filename to determine type from
/// * `min_chars` - Minimum characters required for a line to be included
///
/// # Returns
/// A boxed FileType implementation appropriate for the file extension
pub fn create_file_type(filename: &str, min_chars: u32) -> Box<dyn FileType> {
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    match extension.as_str() {
        // C/C++
        "c" | "cpp" | "cxx" | "cc" | "h" | "hpp" | "hxx" | "hh" => {
            Box::new(CFileType::new(min_chars))
        }
        // Java
        "java" => Box::new(JavaFileType::new(min_chars)),
        // C#
        "cs" => Box::new(CSharpFileType::new(min_chars)),
        // VB.NET
        "vb" => Box::new(VbFileType::new(min_chars)),
        // Erlang
        "erl" | "hrl" => Box::new(ErlangFileType::new(min_chars)),
        // Python
        "py" | "pyw" | "pyi" => Box::new(PythonFileType::new(min_chars)),
        // Rust
        "rs" => Box::new(RustFileType::new(min_chars)),
        // JavaScript/TypeScript
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => Box::new(JavaScriptFileType::new(min_chars)),
        // Go
        "go" => Box::new(GoFileType::new(min_chars)),
        // Kotlin
        "kt" | "kts" => Box::new(KotlinFileType::new(min_chars)),
        // Ruby
        "rb" | "rake" | "gemspec" => Box::new(RubyFileType::new(min_chars)),
        // PHP
        "php" | "phtml" | "php3" | "php4" | "php5" | "phps" => {
            Box::new(PhpFileType::new(min_chars))
        }
        // Swift
        "swift" => Box::new(SwiftFileType::new(min_chars)),
        // Scala
        "scala" | "sc" => Box::new(ScalaFileType::new(min_chars)),
        // HTML
        "html" | "htm" | "xhtml" => Box::new(HtmlFileType::new(min_chars)),
        // CSS
        "css" | "scss" | "less" => Box::new(CssFileType::new(min_chars)),
        // Unknown/fallback
        _ => Box::new(UnknownFileType::new(min_chars)),
    }
}

/// Common line validation logic shared by all file types
pub(crate) fn is_valid_line(line: &str, min_chars: u32) -> bool {
    let trimmed = line.trim();

    // Must have at least min_chars non-whitespace characters
    if trimmed.len() < min_chars as usize {
        return false;
    }

    // Must contain at least one alphabetic character
    trimmed.chars().any(|c| c.is_alphabetic())
}

/// Remove leading and trailing whitespace while preserving the line
pub(crate) fn clean_whitespace(line: &str) -> String {
    line.trim().to_string()
}

/// Strip C-style block comments (/* */) and line comments (//).
/// Updates `in_block_comment` state across lines. Returns cleaned content.
pub(crate) fn strip_c_style_comments(line: &str, in_block_comment: &mut bool) -> String {
    let mut cleaned = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if *in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *in_block_comment = false;
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            *in_block_comment = true;
        } else if c == '/' && chars.peek() == Some(&'/') {
            break;
        } else {
            cleaned.push(c);
        }
    }

    cleaned
}

/// Strip nested block comments (/* /* */ */) and line comments (//).
/// Used by languages that support nested comments (Rust, Kotlin, Scala, Swift).
pub(crate) fn strip_nested_comments(
    line: &str,
    in_block_comment: &mut bool,
    comment_depth: &mut i32,
) -> String {
    let mut cleaned = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if *in_block_comment {
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                *comment_depth += 1;
            } else if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *comment_depth -= 1;
                if *comment_depth == 0 {
                    *in_block_comment = false;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            *in_block_comment = true;
            *comment_depth = 1;
        } else if c == '/' && chars.peek() == Some(&'/') {
            break;
        } else {
            cleaned.push(c);
        }
    }

    cleaned
}

/// Analyze a line for parenthesis balance and opening brace.
/// Correctly handles string ("...") and char ('...') literals.
pub(crate) fn analyze_line_basic(line: &str) -> (i32, bool) {
    let mut paren_balance = 0;
    let mut has_open_brace = false;
    let mut in_string = false;
    let mut in_char = false;

    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
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

/// Tracks multi-line function/method signature state across lines.
pub(crate) struct SignatureTracker {
    pub in_signature: bool,
    pub paren_depth: i32,
}

impl SignatureTracker {
    pub fn new() -> Self {
        Self {
            in_signature: false,
            paren_depth: 0,
        }
    }

    /// Update state while inside a multi-line signature.
    pub fn update(&mut self, balance: i32, has_terminator: bool) {
        self.paren_depth += balance;
        if self.paren_depth <= 0 && has_terminator {
            self.in_signature = false;
            self.paren_depth = 0;
        }
    }

    /// Start tracking a new signature. The signature line is always consumed.
    pub fn start(&mut self, balance: i32, has_terminator: bool) {
        self.paren_depth = balance;
        if self.paren_depth <= 0 && has_terminator {
            self.paren_depth = 0;
        } else {
            self.in_signature = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_line() {
        assert!(is_valid_line("int x = 5;", 3));
        assert!(is_valid_line("abc", 3));
        assert!(!is_valid_line("ab", 3)); // too short
        assert!(!is_valid_line("123", 3)); // no alphabetic
        assert!(!is_valid_line("   ", 3)); // only whitespace
    }

    #[test]
    fn test_create_file_type_c() {
        let ft = create_file_type("test.cpp", 3);
        assert_eq!(ft.name(), "C/C++");
    }

    #[test]
    fn test_create_file_type_java() {
        let ft = create_file_type("Test.java", 3);
        assert_eq!(ft.name(), "Java");
    }

    #[test]
    fn test_create_file_type_unknown() {
        let ft = create_file_type("test.xyz", 3);
        assert_eq!(ft.name(), "Unknown");
    }

    #[test]
    fn test_create_file_type_go() {
        let ft = create_file_type("main.go", 3);
        assert_eq!(ft.name(), "Go");
    }

    #[test]
    fn test_create_file_type_kotlin() {
        let ft = create_file_type("Main.kt", 3);
        assert_eq!(ft.name(), "Kotlin");
        let ft2 = create_file_type("build.kts", 3);
        assert_eq!(ft2.name(), "Kotlin");
    }

    #[test]
    fn test_create_file_type_ruby() {
        let ft = create_file_type("app.rb", 3);
        assert_eq!(ft.name(), "Ruby");
        let ft2 = create_file_type("Rakefile.rake", 3);
        assert_eq!(ft2.name(), "Ruby");
    }

    #[test]
    fn test_create_file_type_php() {
        let ft = create_file_type("index.php", 3);
        assert_eq!(ft.name(), "PHP");
    }

    #[test]
    fn test_create_file_type_swift() {
        let ft = create_file_type("ViewController.swift", 3);
        assert_eq!(ft.name(), "Swift");
    }

    #[test]
    fn test_create_file_type_scala() {
        let ft = create_file_type("Main.scala", 3);
        assert_eq!(ft.name(), "Scala");
        let ft2 = create_file_type("script.sc", 3);
        assert_eq!(ft2.name(), "Scala");
    }

    #[test]
    fn test_create_file_type_case_insensitive() {
        let ft1 = create_file_type("test.CPP", 3);
        let ft2 = create_file_type("test.Cpp", 3);
        assert_eq!(ft1.name(), "C/C++");
        assert_eq!(ft2.name(), "C/C++");
    }
}
