//! Shared test helpers for integration tests

use std::path::PathBuf;

pub fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lucidshark-duplo"))
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Create a temp file list referencing fixture files
pub fn create_fixture_file_list(files: &[&str]) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    for f in files {
        let path = fixtures_dir().join(f);
        writeln!(file, "{}", path.display()).expect("Failed to write to temp file");
    }
    file
}

/// Create a file list in a directory referencing files within that directory
pub fn create_file_list_in_dir(dir: &std::path::Path, files: &[&str]) -> PathBuf {
    use std::io::Write;
    let file_list_path = dir.join("files.txt");
    let mut file_list = std::fs::File::create(&file_list_path).unwrap();
    for file in files {
        writeln!(file_list, "{}", dir.join(file).display()).unwrap();
    }
    file_list_path
}

/// Create a source file with given content
pub fn create_source_file(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).expect("Failed to write file");
}
