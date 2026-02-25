//! Baseline comparison module
//!
//! This module provides functionality to save duplicate detection results
//! as a baseline and compare subsequent runs against it to identify NEW duplicates.

mod storage;

pub use storage::{load_baseline, save_baseline, Baseline};
