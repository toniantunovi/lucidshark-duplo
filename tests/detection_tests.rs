//! Duplicate detection accuracy integration tests

use std::path::PathBuf;
use std::process::Command;

/// Get the path to the test fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Get the path to the built binary
fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/lucidshark-duplo")
}

/// Create a temporary file list with the given files
fn create_file_list(files: &[&str]) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    for f in files {
        let path = fixtures_dir().join(f);
        writeln!(file, "{}", path.display()).expect("Failed to write to temp file");
    }
    file
}

/// Build the binary before running tests
fn ensure_binary_built() {
    let status = Command::new("cargo")
        .args(["build"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Failed to build binary");
    assert!(status.success(), "Failed to build binary");
}

/// Run the binary with JSON output and parse the result
fn run_with_json(file_list_path: &std::path::Path) -> serde_json::Value {
    let output = Command::new(binary_path())
        .args(["--json"])
        .arg(file_list_path)
        .output()
        .expect("Failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("Failed to parse JSON output")
}

mod detection_accuracy {
    use super::*;

    #[test]
    fn test_detects_identical_files() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            !duplicates.is_empty(),
            "Should detect duplicates in identical files"
        );

        // Should detect the full file as duplicate (5 lines)
        let first_dup = &duplicates[0];
        assert!(first_dup["line_count"].as_u64().unwrap() >= 4);
    }

    #[test]
    fn test_detects_partial_duplicates() {
        ensure_binary_built();
        let file_list = create_file_list(&["partial_a.c", "partial_b.c"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(!duplicates.is_empty(), "Should detect partial duplicates");

        // The shared_function has 6 lines of duplicate code
        let total_dup_lines: u64 = duplicates
            .iter()
            .map(|d| d["line_count"].as_u64().unwrap_or(0))
            .sum();
        assert!(
            total_dup_lines >= 4,
            "Should detect at least 4 duplicate lines"
        );
    }

    #[test]
    fn test_no_false_positives_on_unique_files() {
        ensure_binary_built();
        let file_list = create_file_list(&["unique_a.c", "unique_b.c"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            duplicates.is_empty(),
            "Should not detect duplicates in unique files"
        );
    }

    #[test]
    fn test_min_block_size_filtering() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        // With min-lines=10, should not detect the 5-line duplicate
        let output = Command::new(binary_path())
            .args(["--json", "-m", "10"])
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            duplicates.is_empty(),
            "Should not detect duplicates below min-lines threshold"
        );
    }

    #[test]
    fn test_comment_stripping_detects_equivalent_code() {
        ensure_binary_built();
        let file_list = create_file_list(&["with_comments.c", "without_comments.c"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        // After stripping comments, these files should have duplicate content
        assert!(
            !duplicates.is_empty(),
            "Should detect duplicates after comment stripping"
        );
    }
}

mod summary_stats {
    use super::*;

    #[test]
    fn test_summary_contains_correct_file_count() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c", "unique_a.c"]);
        let json = run_with_json(file_list.path());

        let files_analyzed = json["summary"]["files_analyzed"].as_u64().unwrap();
        assert_eq!(
            files_analyzed, 3,
            "Should report correct number of files analyzed"
        );
    }

    #[test]
    fn test_summary_contains_duplicate_stats() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);
        let json = run_with_json(file_list.path());

        assert!(json["summary"]["duplicate_blocks"].as_u64().unwrap() > 0);
        assert!(json["summary"]["duplicate_lines"].as_u64().unwrap() > 0);
        assert!(json["summary"]["total_lines"].as_u64().unwrap() > 0);
    }
}

mod language_specific {
    use super::*;

    #[test]
    fn test_python_comment_stripping() {
        ensure_binary_built();
        let file_list = create_file_list(&["python_with_comments.py", "python_no_comments.py"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            !duplicates.is_empty(),
            "Should detect duplicates in Python files after stripping # comments and docstrings"
        );
    }

    #[test]
    fn test_javascript_comment_stripping() {
        ensure_binary_built();
        let file_list = create_file_list(&["js_with_comments.js", "js_no_comments.js"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            !duplicates.is_empty(),
            "Should detect duplicates in JavaScript files after stripping // and /* */ comments"
        );
    }

    #[test]
    fn test_rust_comment_stripping() {
        ensure_binary_built();
        let file_list = create_file_list(&["rust_with_comments.rs", "rust_no_comments.rs"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            !duplicates.is_empty(),
            "Should detect duplicates in Rust files after stripping comments (including nested)"
        );
    }

    #[test]
    fn test_html_comment_stripping() {
        ensure_binary_built();
        let file_list = create_file_list(&["html_with_comments.html", "html_no_comments.html"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            !duplicates.is_empty(),
            "Should detect duplicates in HTML files after stripping <!-- --> comments"
        );
    }

    #[test]
    fn test_css_comment_stripping() {
        ensure_binary_built();
        let file_list = create_file_list(&["css_with_comments.css", "css_no_comments.css"]);
        let json = run_with_json(file_list.path());

        let duplicates = json["duplicates"]
            .as_array()
            .expect("duplicates should be array");
        assert!(
            !duplicates.is_empty(),
            "Should detect duplicates in CSS files after stripping /* */ comments"
        );
    }
}
