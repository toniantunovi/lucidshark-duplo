//! Configuration types for lucidshark-duplo

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

    /// Whether to ignore preprocessor directives (default: false)
    /// When true, lines starting with # (C/C++) or equivalent are skipped
    pub ignore_preprocessor: bool,

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

    /// Path to input file list (or "-" for stdin)
    pub list_filename: String,

    /// Path to output file (or "-" for stdout)
    pub output_filename: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_chars: 3,
            ignore_preprocessor: false,
            min_block_size: 4,
            block_percent_threshold: 100,
            files_to_check: 0,
            num_threads: num_cpus::get(),
            output_format: OutputFormat::Console,
            ignore_same_filename: false,
            list_filename: String::new(),
            output_filename: String::from("-"),
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
}
