//! CLI integration tests for lucidshark-duplo

mod common;

use std::process::Command;

mod cli_behavior {
    use super::*;

    #[test]
    fn test_help_flag() {
        // binary is auto-built by cargo test
        let output = Command::new(common::binary_path())
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
        // binary is auto-built by cargo test
        let output = Command::new(common::binary_path())
            .arg("--version")
            .output()
            .expect("Failed to run binary");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("lucidshark-duplo"));
    }

    #[test]
    fn test_missing_file_list_argument() {
        // binary is auto-built by cargo test
        let output = Command::new(common::binary_path())
            .output()
            .expect("Failed to run binary");

        // Should fail with missing required argument
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("FILE_LIST") || stderr.contains("required"));
    }

    #[test]
    fn test_conflicting_output_formats() {
        // binary is auto-built by cargo test
        let file_list = common::create_fixture_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(common::binary_path())
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
        // binary is auto-built by cargo test
        let output = Command::new(common::binary_path())
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
        // binary is auto-built by cargo test
        let file_list = common::create_fixture_file_list(&["identical_a.c", "identical_b.c"]);

        let output = Command::new(common::binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        // Exit code 1 means duplicates were found
        assert_eq!(output.status.code(), Some(1));
    }

    #[test]
    fn test_exit_code_0_when_no_duplicates() {
        // binary is auto-built by cargo test
        let file_list = common::create_fixture_file_list(&["unique_a.c", "unique_b.c"]);

        let output = Command::new(common::binary_path())
            .arg(file_list.path())
            .output()
            .expect("Failed to run binary");

        // Exit code 0 means no duplicates
        assert_eq!(output.status.code(), Some(0));
    }
}
