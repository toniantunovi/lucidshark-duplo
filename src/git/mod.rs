//! Git integration module for file discovery
//!
//! This module provides functionality to discover source files using git,
//! including tracking files and detecting changed files for PR workflows.

mod discovery;

// Keep all discovery functions in public API even if not all are used in main
#[allow(unused_imports)]
pub use discovery::{
    detect_base_branch, discover_files, discover_files_with_changed_set, get_changed_files,
    get_repo_root, get_tracked_files, is_git_repo, GitDiscoveryResult,
};
