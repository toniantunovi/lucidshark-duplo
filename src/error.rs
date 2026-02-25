//! Error types for lucidshark-duplo

use thiserror::Error;

/// Result type alias for Duplo operations
pub type Result<T> = std::result::Result<T, DuploError>;

/// Error types for Duplo operations
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum DuploError {
    /// File could not be opened or read
    #[error("Cannot open file '{path}': {reason}")]
    FileNotFound { path: String, reason: String },

    /// File is too large for the configured thread count
    #[error(
        "File '{path}' has {lines} lines, which is too large.\n\
         Using {threads} thread(s), maximum supported is approximately {max_lines} lines per file."
    )]
    FileTooLarge {
        path: String,
        lines: usize,
        threads: usize,
        max_lines: usize,
    },

    /// Memory allocation failed
    #[error("Memory allocation failed: {0}")]
    AllocationFailed(String),

    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Conflicting output format options
    #[error("Output format conflict: specify only one of --json or --xml")]
    OutputFormatConflict,

    /// I/O error during file operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Git operation failed
    #[error("Git error: {0}")]
    GitError(String),

    /// Not inside a git repository
    #[error("Not a git repository. The --git flag requires running inside a git repository.")]
    NotGitRepo,

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Baseline file error
    #[error("Baseline error: {0}")]
    BaselineError(String),

    /// Baseline version mismatch
    #[error("Baseline version {found} is not supported (expected {expected})")]
    BaselineVersionMismatch { found: u32, expected: u32 },

    /// Generic error for other cases
    #[error("{0}")]
    Other(String),
}
