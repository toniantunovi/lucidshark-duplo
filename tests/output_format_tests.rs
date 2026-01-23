//! Output format validation integration tests

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

mod json_output {
    use super::*;

    #[test]
    fn test_json_is_valid() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .args(["--json"])
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");

        // Verify structure
        assert!(
            json.get("duplicates").is_some(),
            "Should have 'duplicates' field"
        );
        assert!(json.get("summary").is_some(), "Should have 'summary' field");
    }

    #[test]
    fn test_json_duplicate_structure() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .args(["--json"])
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        let duplicates = json["duplicates"].as_array().unwrap();
        assert!(!duplicates.is_empty());

        let dup = &duplicates[0];
        assert!(dup.get("line_count").is_some());
        assert!(dup.get("file1").is_some());
        assert!(dup.get("file2").is_some());
        assert!(dup.get("lines").is_some());

        // Check file structure
        let file1 = &dup["file1"];
        assert!(file1.get("path").is_some());
        assert!(file1.get("start_line").is_some());
        assert!(file1.get("end_line").is_some());
    }

    #[test]
    fn test_json_summary_structure() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .args(["--json"])
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        let summary = &json["summary"];
        assert!(summary.get("files_analyzed").is_some());
        assert!(summary.get("total_lines").is_some());
        assert!(summary.get("duplicate_blocks").is_some());
        assert!(summary.get("duplicate_lines").is_some());
        assert!(summary.get("duplication_percent").is_some());
    }
}

mod xml_output {
    use super::*;

    #[test]
    fn test_xml_has_correct_structure() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .args(["--xml"])
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check for XML structure
        assert!(
            stdout.contains("<?xml version"),
            "Should have XML declaration"
        );
        assert!(stdout.contains("<duplo>"), "Should have duplo root element");
        assert!(
            stdout.contains("</duplo>"),
            "Should have closing duplo element"
        );
        assert!(
            stdout.contains("<set"),
            "Should have set elements for duplicates"
        );
        assert!(stdout.contains("<block"), "Should have block elements");
        assert!(stdout.contains("<summary"), "Should have summary element");
    }

    #[test]
    fn test_xml_contains_line_count() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .args(["--xml"])
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("LineCount="),
            "Should include LineCount attribute"
        );
    }
}

mod console_output {
    use super::*;

    #[test]
    fn test_console_shows_file_paths() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("identical_a.c"),
            "Should show first file path"
        );
        assert!(
            stdout.contains("identical_b.c"),
            "Should show second file path"
        );
    }

    #[test]
    fn test_console_shows_line_numbers() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should show line ranges like (1-5)
        assert!(
            stdout.contains("(") && stdout.contains(")"),
            "Should show line number ranges"
        );
        assert!(stdout.contains("<->"), "Should show <-> between file pairs");
    }

    #[test]
    fn test_console_shows_summary() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Summary:"), "Should have Summary section");
        assert!(
            stdout.contains("Files analyzed:"),
            "Should show files analyzed"
        );
        assert!(
            stdout.contains("Duplicate blocks:"),
            "Should show duplicate blocks"
        );
    }

    #[test]
    fn test_console_shows_configuration() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Configuration:"),
            "Should have Configuration section"
        );
        assert!(
            stdout.contains("Minimum block size:"),
            "Should show min block size"
        );
    }
}
