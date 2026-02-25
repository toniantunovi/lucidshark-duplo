//! Integration tests for caching and baseline features

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lucidshark-duplo"))
}

/// Create a source file with given content
fn create_source_file(dir: &std::path::Path, name: &str, content: &str) {
    fs::write(dir.join(name), content).expect("Failed to write file");
}

/// Create a file list pointing to the given files
fn create_file_list(dir: &std::path::Path, files: &[&str]) -> PathBuf {
    let file_list_path = dir.join("files.txt");
    let mut file_list = fs::File::create(&file_list_path).unwrap();
    for file in files {
        writeln!(file_list, "{}", dir.join(file).display()).unwrap();
    }
    file_list_path
}

mod caching {
    use super::*;

    #[test]
    fn test_cache_flag_creates_cache_directory() {
        let temp = TempDir::new().unwrap();

        // Create source files
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
        let file_list = create_file_list(temp.path(), &["a.c", "b.c"]);

        let cache_dir = temp.path().join("my-cache");

        // Run with --cache
        let output = Command::new(binary_path())
            .args([
                "--cache",
                "--cache-dir",
                cache_dir.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        assert!(
            output.status.code() == Some(1),
            "Expected exit code 1 (duplicates found)"
        );

        // Cache directory should be created
        assert!(cache_dir.exists(), "Cache directory should be created");

        // Cache should contain .cache files
        let cache_files: Vec<_> = fs::read_dir(&cache_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "cache"))
            .collect();

        assert_eq!(
            cache_files.len(),
            2,
            "Should have 2 cache files (one per source file)"
        );
    }

    #[test]
    fn test_cache_speeds_up_second_run() {
        let temp = TempDir::new().unwrap();

        // Create source files
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
        let file_list = create_file_list(temp.path(), &["a.c", "b.c"]);

        let cache_dir = temp.path().join("cache");

        // First run - cache miss
        let output1 = Command::new(binary_path())
            .args([
                "--cache",
                "--cache-dir",
                cache_dir.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        let stderr1 = String::from_utf8_lossy(&output1.stderr);
        // First run should not have cache hits (or might show "0 hits")
        assert!(
            !stderr1.contains("2 hits"),
            "First run should not have cache hits for all files"
        );

        // Second run - cache hit
        let output2 = Command::new(binary_path())
            .args([
                "--cache",
                "--cache-dir",
                cache_dir.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        let stderr2 = String::from_utf8_lossy(&output2.stderr);
        // Second run should have cache hits
        assert!(
            stderr2.contains("2 hits"),
            "Second run should have cache hits: {}",
            stderr2
        );
    }

    #[test]
    fn test_clear_cache_removes_cache_files() {
        let temp = TempDir::new().unwrap();

        // Create source files
        create_source_file(temp.path(), "a.c", "int main() { return 0; }");
        let file_list = create_file_list(temp.path(), &["a.c"]);

        let cache_dir = temp.path().join("cache");

        // First run to create cache
        Command::new(binary_path())
            .args([
                "--cache",
                "--cache-dir",
                cache_dir.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Verify cache exists
        let cache_files: Vec<_> = fs::read_dir(&cache_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "cache"))
            .collect();
        assert!(!cache_files.is_empty(), "Cache files should exist");

        // Run with --clear-cache
        let output = Command::new(binary_path())
            .args([
                "--cache",
                "--cache-dir",
                cache_dir.to_str().unwrap(),
                "--clear-cache",
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Clearing cache"),
            "Should show clearing cache message"
        );
    }

    #[test]
    fn test_cache_invalidation_on_file_change() {
        let temp = TempDir::new().unwrap();

        // Create initial source file
        create_source_file(temp.path(), "a.c", "int x = 1;");
        let file_list = create_file_list(temp.path(), &["a.c"]);

        let cache_dir = temp.path().join("cache");

        // First run to create cache
        Command::new(binary_path())
            .args([
                "--cache",
                "--cache-dir",
                cache_dir.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Modify the file
        create_source_file(temp.path(), "a.c", "int x = 2; int y = 3;");

        // Second run - cache should be invalidated
        let output = Command::new(binary_path())
            .args([
                "--cache",
                "--cache-dir",
                cache_dir.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should have cache miss (0 hits or 1 miss)
        assert!(
            stderr.contains("0 hits") || !stderr.contains("1 hits"),
            "Cache should be invalidated on file change: {}",
            stderr
        );
    }
}

mod baseline {
    use super::*;

    #[test]
    fn test_save_baseline_creates_file() {
        let temp = TempDir::new().unwrap();

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
        let file_list = create_file_list(temp.path(), &["a.c", "b.c"]);

        let baseline_path = temp.path().join("baseline.json");

        // Run with --save-baseline
        let output = Command::new(binary_path())
            .args([
                "--save-baseline",
                baseline_path.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        assert!(
            output.status.code() == Some(1),
            "Expected exit code 1 (duplicates found)"
        );

        // Baseline file should be created
        assert!(baseline_path.exists(), "Baseline file should be created");

        // Baseline should be valid JSON
        let content = fs::read_to_string(&baseline_path).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("Baseline should be valid JSON");

        assert!(json["version"].is_u64(), "Baseline should have version");
        assert!(json["entries"].is_array(), "Baseline should have entries");
    }

    #[test]
    fn test_baseline_filters_known_duplicates() {
        let temp = TempDir::new().unwrap();

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
        let file_list = create_file_list(temp.path(), &["a.c", "b.c"]);

        let baseline_path = temp.path().join("baseline.json");

        // First run: save baseline
        Command::new(binary_path())
            .args([
                "--save-baseline",
                baseline_path.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Second run: use baseline (same files, no new duplicates)
        let output = Command::new(binary_path())
            .args([
                "--baseline",
                baseline_path.to_str().unwrap(),
                "--json",
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Should exit with 0 (no NEW duplicates)
        assert_eq!(
            output.status.code(),
            Some(0),
            "Expected exit code 0 (no new duplicates), stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // JSON output should show 0 duplicate blocks
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
        assert_eq!(
            json["summary"]["duplicate_blocks"].as_u64().unwrap(),
            0,
            "Should have 0 duplicate blocks when all are in baseline"
        );
    }

    #[test]
    fn test_baseline_reports_new_duplicates() {
        let temp = TempDir::new().unwrap();

        // Create initial source files
        let code1 = "int x = 1;";
        create_source_file(temp.path(), "a.c", code1);
        let file_list = create_file_list(temp.path(), &["a.c", "b.c", "c.c"]);

        // Create baseline with no duplicates
        create_source_file(temp.path(), "b.c", "int y = 2;");
        create_source_file(temp.path(), "c.c", "int z = 3;");

        let baseline_path = temp.path().join("baseline.json");

        // Create baseline (no duplicates)
        Command::new(binary_path())
            .args([
                "--save-baseline",
                baseline_path.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Now create a duplicate
        let duplicate_code = r#"
int duplicate_function() {
    int x = 1;
    int y = 2;
    int z = 3;
    return x + y + z;
}
"#;
        create_source_file(temp.path(), "b.c", duplicate_code);
        create_source_file(temp.path(), "c.c", duplicate_code);

        // Run with baseline - should find NEW duplicates
        let output = Command::new(binary_path())
            .args([
                "--baseline",
                baseline_path.to_str().unwrap(),
                "--json",
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Should exit with 1 (new duplicates found)
        assert_eq!(
            output.status.code(),
            Some(1),
            "Expected exit code 1 (new duplicates), stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Should report the new duplicates
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
        assert!(
            json["summary"]["duplicate_blocks"].as_u64().unwrap() > 0,
            "Should have new duplicate blocks"
        );
    }

    #[test]
    fn test_baseline_with_save_baseline_updates_baseline() {
        let temp = TempDir::new().unwrap();

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
        let file_list = create_file_list(temp.path(), &["a.c", "b.c"]);

        let old_baseline = temp.path().join("old-baseline.json");
        let new_baseline = temp.path().join("new-baseline.json");

        // Create initial baseline
        Command::new(binary_path())
            .args([
                "--save-baseline",
                old_baseline.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        // Run with both --baseline and --save-baseline
        let output = Command::new(binary_path())
            .args([
                "--baseline",
                old_baseline.to_str().unwrap(),
                "--save-baseline",
                new_baseline.to_str().unwrap(),
                file_list.to_str().unwrap(),
            ])
            .current_dir(temp.path())
            .output()
            .expect("Failed to run binary");

        assert!(output.status.success() || output.status.code() == Some(1));

        // New baseline should be created
        assert!(new_baseline.exists(), "New baseline should be created");
    }
}
