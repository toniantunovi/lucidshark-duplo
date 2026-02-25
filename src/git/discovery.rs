//! Git file discovery functionality
//!
//! Provides functions to discover source files from git repositories,
//! including all tracked files or only changed files vs a base branch.

use crate::config::Config;
use crate::error::{DuploError, Result};
use std::path::PathBuf;
use std::process::Command;

/// Check if the current directory is inside a git repository
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get the root directory of the git repository
pub fn get_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| DuploError::GitError(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(DuploError::NotGitRepo);
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

/// Get all tracked files in the repository
pub fn get_tracked_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .map_err(|e| DuploError::GitError(format!("Failed to run git ls-files: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DuploError::GitError(format!(
            "git ls-files failed: {}",
            stderr
        )));
    }

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(files)
}

/// Detect the default base branch (tries main, master, develop in order)
pub fn detect_base_branch() -> Result<String> {
    // Try common default branches in order of preference
    for branch in &["main", "master", "develop"] {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", &format!("refs/heads/{}", branch)])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if let Ok(status) = output {
            if status.success() {
                return Ok(branch.to_string());
            }
        }
    }

    // Fallback: try to get from remote origin HEAD
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let remote = String::from_utf8_lossy(&o.stdout).trim().to_string();
            // remote is like "origin/main", extract "main"
            if let Some(branch) = remote.split('/').last() {
                return Ok(branch.to_string());
            }
        }
    }

    Err(DuploError::GitError(
        "Could not detect base branch. Use --base-branch to specify.".to_string(),
    ))
}

/// Get files changed compared to a base branch
pub fn get_changed_files(base_branch: &str) -> Result<Vec<String>> {
    // Get merge base commit
    let merge_base_output = Command::new("git")
        .args(["merge-base", "HEAD", base_branch])
        .output()
        .map_err(|e| DuploError::GitError(format!("Failed to run git merge-base: {}", e)))?;

    if !merge_base_output.status.success() {
        let stderr = String::from_utf8_lossy(&merge_base_output.stderr);
        return Err(DuploError::GitError(format!(
            "Failed to find merge base with '{}': {}. Is it a valid branch?",
            base_branch, stderr
        )));
    }

    let base_commit = String::from_utf8_lossy(&merge_base_output.stdout)
        .trim()
        .to_string();

    // Get changed files between merge base and HEAD
    let output = Command::new("git")
        .args(["diff", "--name-only", &base_commit, "HEAD"])
        .output()
        .map_err(|e| DuploError::GitError(format!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DuploError::GitError(format!(
            "git diff --name-only failed: {}",
            stderr
        )));
    }

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(files)
}

/// Check if a file has a supported source code extension
fn is_supported_file(path: &str) -> bool {
    let supported_extensions = [
        // C/C++
        ".c", ".cpp", ".cxx", ".cc", ".h", ".hpp", ".hxx", ".hh", // Java
        ".java", // C#
        ".cs", // Python
        ".py", // Rust
        ".rs", // JavaScript/TypeScript
        ".js", ".ts", ".jsx", ".tsx", // HTML/CSS
        ".html", ".htm", ".css", // Visual Basic
        ".vb", // Erlang
        ".erl",
    ];

    let path_lower = path.to_lowercase();
    supported_extensions
        .iter()
        .any(|ext| path_lower.ends_with(ext))
}

/// Result of git file discovery for --changed-only mode
pub struct GitDiscoveryResult {
    /// All files to analyze
    pub files: Vec<String>,
    /// Files that are changed (subset of files, only populated when changed_only is true)
    pub changed_files: Option<std::collections::HashSet<String>>,
}

/// Main entry point for git file discovery
///
/// When `changed_only` is true:
/// - Returns ALL tracked files (for comparison)
/// - Also returns the set of changed files (for filtering results)
///
/// Otherwise, returns all tracked files with no changed set.
///
/// All returned paths are absolute paths.
#[allow(dead_code)]
pub fn discover_files(config: &Config, progress: &impl Fn(&str)) -> Result<Vec<String>> {
    let result = discover_files_with_changed_set(config, progress)?;
    Ok(result.files)
}

/// Git file discovery that also returns the changed file set
pub fn discover_files_with_changed_set(
    config: &Config,
    progress: &impl Fn(&str),
) -> Result<GitDiscoveryResult> {
    if !is_git_repo() {
        return Err(DuploError::NotGitRepo);
    }

    let repo_root = get_repo_root()?;

    // Always get all tracked files
    progress("Finding git-tracked files...");
    let all_files = get_tracked_files()?;

    // Convert to absolute paths and filter by supported extensions
    let absolute_files: Vec<String> = all_files
        .into_iter()
        .filter(|f| is_supported_file(f))
        .map(|f| repo_root.join(&f).to_string_lossy().to_string())
        .filter(|f| std::path::Path::new(f).exists())
        .collect();

    // If changed_only, also get the changed file set
    let changed_files = if config.changed_only {
        let base_branch = config
            .base_branch
            .clone()
            .map(Ok)
            .unwrap_or_else(detect_base_branch)?;

        progress(&format!(
            "Finding files changed vs '{}' branch...",
            base_branch
        ));
        let changed = get_changed_files(&base_branch)?;

        // Convert to absolute paths and create set
        let changed_set: std::collections::HashSet<String> = changed
            .into_iter()
            .filter(|f| is_supported_file(f))
            .map(|f| repo_root.join(&f).to_string_lossy().to_string())
            .collect();

        progress(&format!("Found {} changed files", changed_set.len()));
        Some(changed_set)
    } else {
        None
    };

    progress(&format!("Found {} source files", absolute_files.len()));
    Ok(GitDiscoveryResult {
        files: absolute_files,
        changed_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_file_rust() {
        assert!(is_supported_file("main.rs"));
        assert!(is_supported_file("src/lib.rs"));
        assert!(is_supported_file("/path/to/file.rs"));
    }

    #[test]
    fn test_is_supported_file_c_cpp() {
        assert!(is_supported_file("main.c"));
        assert!(is_supported_file("main.cpp"));
        assert!(is_supported_file("header.h"));
        assert!(is_supported_file("header.hpp"));
        assert!(is_supported_file("file.cc"));
        assert!(is_supported_file("file.cxx"));
    }

    #[test]
    fn test_is_supported_file_javascript() {
        assert!(is_supported_file("app.js"));
        assert!(is_supported_file("app.ts"));
        assert!(is_supported_file("Component.jsx"));
        assert!(is_supported_file("Component.tsx"));
    }

    #[test]
    fn test_is_supported_file_python() {
        assert!(is_supported_file("script.py"));
        assert!(is_supported_file("/path/to/module.py"));
    }

    #[test]
    fn test_is_supported_file_java() {
        assert!(is_supported_file("Main.java"));
        assert!(is_supported_file("com/example/Class.java"));
    }

    #[test]
    fn test_is_supported_file_unsupported() {
        assert!(!is_supported_file("README.md"));
        assert!(!is_supported_file("Cargo.toml"));
        assert!(!is_supported_file("package.json"));
        assert!(!is_supported_file("image.png"));
        assert!(!is_supported_file(".gitignore"));
        assert!(!is_supported_file("Makefile"));
    }

    #[test]
    fn test_is_supported_file_case_insensitive() {
        assert!(is_supported_file("FILE.RS"));
        assert!(is_supported_file("Main.JAVA"));
        assert!(is_supported_file("script.PY"));
    }

    #[test]
    fn test_is_supported_file_web() {
        assert!(is_supported_file("index.html"));
        assert!(is_supported_file("page.htm"));
        assert!(is_supported_file("styles.css"));
    }

    #[test]
    fn test_is_supported_file_other_languages() {
        assert!(is_supported_file("Program.cs"));
        assert!(is_supported_file("Module.vb"));
        assert!(is_supported_file("server.erl"));
    }
}
