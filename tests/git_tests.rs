//! Git integration tests
//!
//! Tests for the --git, --changed-only, and --base-branch CLI options.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lucidshark-duplo"))
}

/// Set up a git repository with initial configuration
fn setup_git_repo() -> TempDir {
    let temp = TempDir::new().unwrap();

    // Initialize git repo with 'main' as the default branch
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to init git repo");

    // Configure git user (required for commits)
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to configure git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to configure git name");

    temp
}

/// Create a source file with given content
fn create_source_file(dir: &std::path::Path, name: &str, content: &str) {
    fs::write(dir.join(name), content).expect("Failed to write file");
}

/// Git add files
fn git_add(dir: &std::path::Path, files: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("add").current_dir(dir);
    for file in files {
        cmd.arg(file);
    }
    cmd.output().expect("Failed to git add");
}

/// Git commit
fn git_commit(dir: &std::path::Path, message: &str) {
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .expect("Failed to git commit");
}

/// Create a branch
fn git_branch(dir: &std::path::Path, branch: &str) {
    Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(dir)
        .output()
        .expect("Failed to create branch");
}

mod git_discovery {
    use super::*;

    #[test]
    fn test_git_flag_discovers_tracked_files() {
        let temp = setup_git_repo();

        // Create source files with duplicates
        let code = r#"
int duplicate_function() {
    int x = 1;
    int y = 2;
    int z = 3;
    return x + y + z;
}
"#;
        create_source_file(temp.path(), "a.c", code);
        create_source_file(temp.path(), "b.c", code);

        // Add and commit
        git_add(temp.path(), &["a.c", "b.c"]);
        git_commit(temp.path(), "initial commit");

        // Run duplo with --git
        let output = Command::new(binary_path())
            .args(["--git", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Should succeed (exit 1 = duplicates found)
        assert_eq!(
            output.status.code(),
            Some(1),
            "Expected exit code 1, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        // Should have analyzed 2 files
        assert_eq!(
            json["summary"]["files_analyzed"].as_u64().unwrap(),
            2,
            "Should analyze 2 files"
        );

        // Should find duplicates
        assert!(
            json["summary"]["duplicate_blocks"].as_u64().unwrap() > 0,
            "Should find duplicates"
        );
    }

    #[test]
    fn test_git_flag_ignores_untracked_files() {
        let temp = setup_git_repo();

        // Create tracked file
        let code = r#"
int tracked() {
    return 42;
}
"#;
        create_source_file(temp.path(), "tracked.c", code);
        git_add(temp.path(), &["tracked.c"]);
        git_commit(temp.path(), "initial commit");

        // Create untracked file with same content (would be duplicate if analyzed)
        create_source_file(temp.path(), "untracked.c", code);

        // Run duplo with --git
        let output = Command::new(binary_path())
            .args(["--git", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        // Should only analyze 1 file (the tracked one)
        assert_eq!(
            json["summary"]["files_analyzed"].as_u64().unwrap(),
            1,
            "Should only analyze tracked files"
        );
    }

    #[test]
    fn test_git_flag_filters_unsupported_extensions() {
        let temp = setup_git_repo();

        // Create various files
        create_source_file(temp.path(), "code.c", "int main() { return 0; }");
        create_source_file(temp.path(), "README.md", "# Project");
        create_source_file(temp.path(), "config.toml", "[package]\nname = \"test\"");
        create_source_file(temp.path(), "data.json", "{}");

        git_add(
            temp.path(),
            &["code.c", "README.md", "config.toml", "data.json"],
        );
        git_commit(temp.path(), "initial commit");

        // Run duplo with --git
        let output = Command::new(binary_path())
            .args(["--git", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        // Should only analyze 1 file (code.c)
        assert_eq!(
            json["summary"]["files_analyzed"].as_u64().unwrap(),
            1,
            "Should only analyze supported file types"
        );
    }

    #[test]
    fn test_git_flag_fails_outside_repo() {
        let temp = TempDir::new().unwrap(); // Not a git repo

        let output = Command::new(binary_path())
            .args(["--git"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        assert_eq!(
            output.status.code(),
            Some(2),
            "Should fail with exit code 2"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("git repository") || stderr.contains("Not a git"),
            "Error should mention git repository"
        );
    }
}

mod changed_only {
    use super::*;

    #[test]
    fn test_changed_only_requires_git_flag() {
        let output = Command::new(binary_path())
            .args(["--changed-only", "files.txt"])
            .output()
            .expect("Failed to run binary");

        // Clap should reject this combination
        assert!(
            !output.status.success(),
            "Should fail when --changed-only used without --git"
        );
    }

    #[test]
    fn test_changed_only_analyzes_changed_files() {
        let temp = setup_git_repo();

        // Create initial file on main branch
        let original_code = r#"
int original() {
    int a = 1;
    int b = 2;
    int c = 3;
    return a + b + c;
}
"#;
        create_source_file(temp.path(), "original.c", original_code);
        git_add(temp.path(), &["original.c"]);
        git_commit(temp.path(), "initial commit");

        // Create a feature branch
        git_branch(temp.path(), "feature");

        // Add a new file with duplicate code on feature branch
        create_source_file(temp.path(), "new_file.c", original_code);
        git_add(temp.path(), &["new_file.c"]);
        git_commit(temp.path(), "add duplicate");

        // Run with --changed-only
        let output = Command::new(binary_path())
            .args(["--git", "--changed-only", "--base-branch", "main", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Should find the duplicate between original.c and new_file.c
        // Even though only new_file.c is "changed", it should be compared against all files
        assert_eq!(
            output.status.code(),
            Some(1),
            "Expected duplicates to be found, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        assert!(
            json["summary"]["duplicate_blocks"].as_u64().unwrap() > 0,
            "Should find duplicate between original and new file"
        );
    }

    #[test]
    fn test_changed_only_no_changes_no_analysis() {
        let temp = setup_git_repo();

        // Create file on main
        create_source_file(temp.path(), "file.c", "int main() { return 0; }");
        git_add(temp.path(), &["file.c"]);
        git_commit(temp.path(), "initial commit");

        // Create feature branch but don't change anything
        git_branch(temp.path(), "feature");

        // Run with --changed-only (no changes)
        let output = Command::new(binary_path())
            .args(["--git", "--changed-only", "--base-branch", "main", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Should succeed with no duplicates (no files to analyze)
        assert_eq!(
            output.status.code(),
            Some(0),
            "Expected exit 0 when no changed files, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_base_branch_auto_detection() {
        let temp = setup_git_repo();

        // Create a file on main branch
        create_source_file(temp.path(), "file.c", "int main() { return 0; }");
        git_add(temp.path(), &["file.c"]);
        git_commit(temp.path(), "initial commit");

        // Create feature branch
        git_branch(temp.path(), "feature");

        // Add a new file
        create_source_file(temp.path(), "new.c", "int new() { return 1; }");
        git_add(temp.path(), &["new.c"]);
        git_commit(temp.path(), "add new file");

        // Run without specifying base branch (should auto-detect 'main')
        let output = Command::new(binary_path())
            .args(["--git", "--changed-only", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Should succeed (auto-detect main branch)
        assert!(
            output.status.code() == Some(0) || output.status.code() == Some(1),
            "Should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_invalid_base_branch_error() {
        let temp = setup_git_repo();

        create_source_file(temp.path(), "file.c", "int main() { return 0; }");
        git_add(temp.path(), &["file.c"]);
        git_commit(temp.path(), "initial commit");

        // Run with non-existent base branch
        let output = Command::new(binary_path())
            .args([
                "--git",
                "--changed-only",
                "--base-branch",
                "nonexistent-branch",
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        assert_eq!(
            output.status.code(),
            Some(2),
            "Should fail with invalid base branch"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("merge base") || stderr.contains("nonexistent"),
            "Error should mention the issue with the branch"
        );
    }
}

mod git_with_file_list {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_git_flag_takes_precedence_over_file_list() {
        let temp = setup_git_repo();

        // Create tracked file
        let code = "int tracked() { return 1; }";
        create_source_file(temp.path(), "tracked.c", code);
        git_add(temp.path(), &["tracked.c"]);
        git_commit(temp.path(), "initial commit");

        // Create untracked file
        create_source_file(temp.path(), "untracked.c", code);

        // Create file list pointing to untracked file
        let file_list_path = temp.path().join("files.txt");
        let mut file_list = fs::File::create(&file_list_path).unwrap();
        writeln!(file_list, "{}", temp.path().join("untracked.c").display()).unwrap();

        // Run with both --git and file list (--git should win)
        let output = Command::new(binary_path())
            .args(["--git", "--json"])
            .arg(&file_list_path)
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        // Should analyze tracked files, not the file list
        assert_eq!(
            json["summary"]["files_analyzed"].as_u64().unwrap(),
            1,
            "Should analyze git-tracked files, not file list"
        );
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_repo_no_files() {
        let temp = setup_git_repo();

        // Empty repo, no files
        let output = Command::new(binary_path())
            .args(["--git", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Should succeed with 0 files analyzed
        assert_eq!(
            output.status.code(),
            Some(0),
            "Should succeed with empty repo"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        assert_eq!(
            json["summary"]["files_analyzed"].as_u64().unwrap(),
            0,
            "Should analyze 0 files in empty repo"
        );
    }

    #[test]
    fn test_repo_with_only_unsupported_files() {
        let temp = setup_git_repo();

        // Create only unsupported files
        create_source_file(temp.path(), "README.md", "# Test");
        create_source_file(temp.path(), "Makefile", "all: build");
        git_add(temp.path(), &["README.md", "Makefile"]);
        git_commit(temp.path(), "add files");

        let output = Command::new(binary_path())
            .args(["--git", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        assert_eq!(
            output.status.code(),
            Some(0),
            "Should succeed with 0 supported files"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        assert_eq!(
            json["summary"]["files_analyzed"].as_u64().unwrap(),
            0,
            "Should analyze 0 files when all files are unsupported"
        );
    }

    #[test]
    fn test_subdirectories() {
        let temp = setup_git_repo();

        // Create files in subdirectories
        fs::create_dir_all(temp.path().join("src/module")).unwrap();

        let code = r#"
int deep_function() {
    int x = 1;
    int y = 2;
    return x + y;
}
"#;
        create_source_file(temp.path(), "src/main.c", code);
        create_source_file(temp.path(), "src/module/helper.c", code);

        git_add(temp.path(), &["src/main.c", "src/module/helper.c"]);
        git_commit(temp.path(), "add nested files");

        let output = Command::new(binary_path())
            .args(["--git", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        assert_eq!(
            output.status.code(),
            Some(1),
            "Should find duplicates in subdirectories"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Failed to parse JSON");

        assert_eq!(
            json["summary"]["files_analyzed"].as_u64().unwrap(),
            2,
            "Should analyze files in subdirectories"
        );
    }
}
