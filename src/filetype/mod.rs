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
mod html;
mod java;
mod javascript;
mod python;
mod rust_lang;
mod unknown;
mod vb;

use crate::core::SourceLine;

pub use c::CFileType;
pub use csharp::CSharpFileType;
pub use css::CssFileType;
pub use erlang::ErlangFileType;
pub use html::HtmlFileType;
pub use java::JavaFileType;
pub use javascript::JavaScriptFileType;
pub use python::PythonFileType;
pub use rust_lang::RustFileType;
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
/// * `ignore_preprocessor` - Whether to filter preprocessor directives
/// * `min_chars` - Minimum characters required for a line to be included
///
/// # Returns
/// A boxed FileType implementation appropriate for the file extension
pub fn create_file_type(
    filename: &str,
    ignore_preprocessor: bool,
    min_chars: u32,
) -> Box<dyn FileType> {
    let extension = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        // C/C++
        "c" | "cpp" | "cxx" | "cc" | "h" | "hpp" | "hxx" | "hh" => {
            Box::new(CFileType::new(ignore_preprocessor, min_chars))
        }
        // Java
        "java" => Box::new(JavaFileType::new(ignore_preprocessor, min_chars)),
        // C#
        "cs" => Box::new(CSharpFileType::new(ignore_preprocessor, min_chars)),
        // VB.NET
        "vb" => Box::new(VbFileType::new(ignore_preprocessor, min_chars)),
        // Erlang
        "erl" | "hrl" => Box::new(ErlangFileType::new(ignore_preprocessor, min_chars)),
        // Python
        "py" | "pyw" | "pyi" => Box::new(PythonFileType::new(ignore_preprocessor, min_chars)),
        // Rust
        "rs" => Box::new(RustFileType::new(ignore_preprocessor, min_chars)),
        // JavaScript/TypeScript
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => {
            Box::new(JavaScriptFileType::new(ignore_preprocessor, min_chars))
        }
        // HTML
        "html" | "htm" | "xhtml" => Box::new(HtmlFileType::new(min_chars)),
        // CSS
        "css" | "scss" | "less" => Box::new(CssFileType::new(ignore_preprocessor, min_chars)),
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
        let ft = create_file_type("test.cpp", false, 3);
        assert_eq!(ft.name(), "C/C++");
    }

    #[test]
    fn test_create_file_type_java() {
        let ft = create_file_type("Test.java", false, 3);
        assert_eq!(ft.name(), "Java");
    }

    #[test]
    fn test_create_file_type_unknown() {
        let ft = create_file_type("test.xyz", false, 3);
        assert_eq!(ft.name(), "Unknown");
    }

    #[test]
    fn test_create_file_type_case_insensitive() {
        let ft1 = create_file_type("test.CPP", false, 3);
        let ft2 = create_file_type("test.Cpp", false, 3);
        assert_eq!(ft1.name(), "C/C++");
        assert_eq!(ft2.name(), "C/C++");
    }
}
