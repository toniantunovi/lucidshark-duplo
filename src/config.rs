//! Configuration types for lucidshark-duplo

use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// Output format for duplicate detection results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable console output
    #[default]
    Console,
    /// JSON output with structured data
    Json,
    /// XML output for tool integration
    Xml,
}

/// Configuration options for Duplo
#[derive(Debug, Clone)]
pub struct Config {
    /// Minimum number of characters in a line to be considered (default: 3)
    /// Lines with fewer characters are ignored
    pub min_chars: u32,

    /// Minimum block size in lines to report (default: 4)
    /// Duplicate blocks smaller than this are ignored
    pub min_block_size: u32,

    /// Block percentage threshold (default: 100)
    /// When set below 100, also considers blocks that represent
    /// at least this percentage of the smaller file
    pub block_percent_threshold: u8,

    /// Maximum number of files to analyze (0 = all files)
    pub files_to_check: usize,

    /// Number of threads for parallel processing (default: num_cpus)
    pub num_threads: usize,

    /// Output format (console, json, or xml)
    pub output_format: OutputFormat,

    /// Ignore file pairs with the same filename (different paths)
    pub ignore_same_filename: bool,

    /// Path to input file list (or "-" for stdin). None when using --git mode.
    pub list_filename: Option<String>,

    /// Path to output file (or "-" for stdout)
    pub output_filename: String,

    // === Git Integration ===
    /// Use git to discover files
    pub git_mode: bool,

    /// Only analyze files changed vs base branch (requires git_mode)
    pub changed_only: bool,

    /// Base branch for --changed-only comparison (auto-detected if None)
    pub base_branch: Option<String>,

    // === Incremental Cache ===
    /// Enable incremental caching
    pub cache_enabled: bool,

    /// Cache directory (default: .duplo-cache in repo root)
    pub cache_dir: Option<PathBuf>,

    /// Clear the cache before running
    pub clear_cache: bool,

    // === Baseline Mode ===
    /// Path to baseline file to compare against
    pub baseline_path: Option<PathBuf>,

    /// Path to save current results as baseline
    pub save_baseline_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_chars: 3,
            min_block_size: 4,
            block_percent_threshold: 100,
            files_to_check: 0,
            num_threads: num_cpus::get(),
            output_format: OutputFormat::Console,
            ignore_same_filename: false,
            list_filename: None,
            output_filename: String::from("-"),
            // Git integration
            git_mode: false,
            changed_only: false,
            base_branch: None,
            // Caching
            cache_enabled: false,
            cache_dir: None,
            clear_cache: false,
            // Baseline
            baseline_path: None,
            save_baseline_path: None,
        }
    }
}

impl Config {
    /// Returns the effective number of files to check
    /// If files_to_check is 0, returns usize::MAX (all files)
    pub fn effective_files_to_check(&self) -> usize {
        if self.files_to_check == 0 {
            usize::MAX
        } else {
            self.files_to_check
        }
    }

    /// Compute a hash of config options that affect source line cleaning.
    /// Used for cache invalidation - if this changes, cached lines are invalid.
    pub fn cleaning_config_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Only min_chars affects the cleaning process
        self.min_chars.hash(&mut hasher);
        hasher.finish()
    }

    /// Compute a hash of config options that affect duplicate detection.
    /// Used for baseline comparison - warns if detection parameters differ.
    pub fn detection_config_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.min_chars.hash(&mut hasher);
        self.min_block_size.hash(&mut hasher);
        self.block_percent_threshold.hash(&mut hasher);
        self.ignore_same_filename.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleaning_config_hash_deterministic() {
        let config1 = Config::default();
        let config2 = Config::default();
        assert_eq!(
            config1.cleaning_config_hash(),
            config2.cleaning_config_hash()
        );
    }

    #[test]
    fn test_cleaning_config_hash_changes_with_min_chars() {
        let mut config1 = Config::default();
        config1.min_chars = 3;

        let mut config2 = Config::default();
        config2.min_chars = 5;

        assert_ne!(
            config1.cleaning_config_hash(),
            config2.cleaning_config_hash()
        );
    }

    #[test]
    fn test_cleaning_config_hash_unchanged_by_min_block_size() {
        let mut config1 = Config::default();
        config1.min_block_size = 4;

        let mut config2 = Config::default();
        config2.min_block_size = 10;

        // min_block_size doesn't affect cleaning, only detection
        assert_eq!(
            config1.cleaning_config_hash(),
            config2.cleaning_config_hash()
        );
    }

    #[test]
    fn test_detection_config_hash_deterministic() {
        let config1 = Config::default();
        let config2 = Config::default();
        assert_eq!(
            config1.detection_config_hash(),
            config2.detection_config_hash()
        );
    }

    #[test]
    fn test_detection_config_hash_changes_with_min_block_size() {
        let mut config1 = Config::default();
        config1.min_block_size = 4;

        let mut config2 = Config::default();
        config2.min_block_size = 10;

        assert_ne!(
            config1.detection_config_hash(),
            config2.detection_config_hash()
        );
    }

    #[test]
    fn test_detection_config_hash_changes_with_threshold() {
        let mut config1 = Config::default();
        config1.block_percent_threshold = 100;

        let mut config2 = Config::default();
        config2.block_percent_threshold = 50;

        assert_ne!(
            config1.detection_config_hash(),
            config2.detection_config_hash()
        );
    }
}
