//! Incremental caching module for processed source files
//!
//! This module provides caching of processed source lines to speed up
//! repeated runs on the same codebase. Files are cached based on their
//! content hash and the cleaning configuration.

mod storage;

pub use storage::{clear_cache, FileCache};
