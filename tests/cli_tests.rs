//! CLI integration tests for lucidshark-duplo

use std::path::PathBuf;
use std::process::Command;

/// Get the path to the test fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Get the path to the built binary
fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/lucidshark-duplo")
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

mod cli_behavior {
    use super::*;

    #[test]
    fn test_help_flag() {
        ensure_binary_built();
        let output = Command::new(binary_path())
            .arg("--help")
            .output()
            .expect("Failed to run binary");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Detect code duplication"));
        assert!(stdout.contains("--json"));
        assert!(stdout.contains("--xml"));
    }

    #[test]
    fn test_version_flag() {
        ensure_binary_built();
        let output = Command::new(binary_path())
            .arg("--version")
            .output()
            .expect("Failed to run binary");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("lucidshark-duplo"));
    }

    #[test]
    fn test_missing_file_list_argument() {
        ensure_binary_built();
        let output = Command::new(binary_path())
            .output()
            .expect("Failed to run binary");

        // Should fail with missing required argument
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("FILE_LIST") || stderr.contains("required"));
    }

    #[test]
    fn test_conflicting_output_formats() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .args(["--json", "--xml"])
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        // Should fail with conflicting formats
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("conflict") || stderr.contains("Output format"));
    }

    #[test]
    fn test_nonexistent_file_list() {
        ensure_binary_built();
        let output = Command::new(binary_path())
            .arg("/nonexistent/file/list.txt")
            .output()
            .expect("Failed to run binary");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Cannot open") || stderr.contains("Error"));
    }
}

mod exit_codes {
    use super::*;

    #[test]
    fn test_exit_code_1_when_duplicates_found() {
        ensure_binary_built();
        let file_list = create_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        // Exit code 1 means duplicates were found
        assert_eq!(output.status.code(), Some(1));
    }

    #[test]
    fn test_exit_code_0_when_no_duplicates() {
        ensure_binary_built();
        let file_list = create_file_list(&["unique_a.c", "unique_b.c"]);

        let output = Command::new(binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        // Exit code 0 means no duplicates
        assert_eq!(output.status.code(), Some(0));
    }
}
