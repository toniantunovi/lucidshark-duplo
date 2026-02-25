//! Baseline storage implementation

use crate::core::{Block, DuploResult, SourceFile};
use crate::error::{DuploError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Current baseline format version
const BASELINE_VERSION: u32 = 1;

/// A single baseline entry representing a known duplicate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BaselineEntry {
    /// Path to the first file
    pub file1: String,
    /// Path to the second file
    pub file2: String,
    /// Hash of the duplicate content (for fuzzy matching)
    pub content_hash: u64,
    /// Number of duplicate lines
    pub line_count: usize,
}

impl BaselineEntry {
    /// Create a normalized baseline entry (files sorted for consistent comparison)
    pub fn new(file1: String, file2: String, content_hash: u64, line_count: usize) -> Self {
        // Sort files for consistent ordering
        let (f1, f2) = if file1 <= file2 {
            (file1, file2)
        } else {
            (file2, file1)
        };
        Self {
            file1: f1,
            file2: f2,
            content_hash,
            line_count,
        }
    }

    /// Create a matching key (file pair only, ignoring content for broad matching)
    #[allow(dead_code)]
    fn file_pair_key(&self) -> (&str, &str) {
        (&self.file1, &self.file2)
    }
}

/// Baseline data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Baseline {
    /// Format version
    pub version: u32,
    /// Hash of detection configuration (for warning about config changes)
    pub config_hash: u64,
    /// All baseline entries
    pub entries: Vec<BaselineEntry>,
}

impl Baseline {
    /// Create a new baseline from detection results
    pub fn from_results(
        result: &DuploResult,
        source_files: &[SourceFile],
        config_hash: u64,
    ) -> Self {
        let entries: Vec<BaselineEntry> = result
            .blocks
            .iter()
            .map(|block| {
                let file1 = source_files[block.source1_idx].filename().to_string();
                let file2 = source_files[block.source2_idx].filename().to_string();
                let content_hash = compute_block_hash(block, source_files);
                BaselineEntry::new(file1, file2, content_hash, block.count)
            })
            .collect();

        Self {
            version: BASELINE_VERSION,
            config_hash,
            entries,
        }
    }

    /// Get the set of baseline entries for fast lookup
    #[allow(dead_code)]
    pub fn entry_set(&self) -> HashSet<BaselineEntry> {
        self.entries.iter().cloned().collect()
    }

    /// Check if a block matches any baseline entry
    pub fn contains(&self, block: &Block, source_files: &[SourceFile]) -> bool {
        let file1 = source_files[block.source1_idx].filename();
        let file2 = source_files[block.source2_idx].filename();
        let content_hash = compute_block_hash(block, source_files);

        // Normalize file order
        let (f1, f2) = if file1 <= file2 {
            (file1, file2)
        } else {
            (file2, file1)
        };

        self.entries.iter().any(|entry| {
            // Match by file pair and content hash
            entry.file1 == f1 && entry.file2 == f2 && entry.content_hash == content_hash
        })
    }

    /// Filter results to only NEW duplicates (not in baseline)
    pub fn filter_new_duplicates(
        &self,
        result: DuploResult,
        source_files: &[SourceFile],
    ) -> DuploResult {
        let new_blocks: Vec<Block> = result
            .blocks
            .into_iter()
            .filter(|block| !self.contains(block, source_files))
            .collect();

        let duplicate_lines: usize = new_blocks.iter().map(|b| b.count).sum();
        let duplicate_blocks = new_blocks.len();

        DuploResult {
            blocks: new_blocks,
            files_analyzed: result.files_analyzed,
            total_lines: result.total_lines,
            duplicate_lines,
            duplicate_blocks,
        }
    }
}

/// Compute a hash of the block's content for fuzzy matching
fn compute_block_hash(block: &Block, source_files: &[SourceFile]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let source = &source_files[block.source1_idx];
    let mut hasher = DefaultHasher::new();

    // Hash all line hashes in the block
    for i in 0..block.count {
        source.get_line(block.line1 + i).hash().hash(&mut hasher);
    }

    hasher.finish()
}

/// Save baseline to a file
pub fn save_baseline(baseline: &Baseline, path: &Path) -> Result<()> {
    let file = File::create(path).map_err(|e| {
        DuploError::BaselineError(format!(
            "Failed to create baseline file '{}': {}",
            path.display(),
            e
        ))
    })?;

    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, baseline).map_err(|e| {
        DuploError::BaselineError(format!("Failed to write baseline: {}", e))
    })?;

    Ok(())
}

/// Load baseline from a file
pub fn load_baseline(path: &Path) -> Result<Baseline> {
    let file = File::open(path).map_err(|e| {
        DuploError::BaselineError(format!(
            "Failed to open baseline file '{}': {}",
            path.display(),
            e
        ))
    })?;

    let reader = BufReader::new(file);
    let baseline: Baseline = serde_json::from_reader(reader).map_err(|e| {
        DuploError::BaselineError(format!("Failed to parse baseline file: {}", e))
    })?;

    // Validate version
    if baseline.version != BASELINE_VERSION {
        return Err(DuploError::BaselineVersionMismatch {
            found: baseline.version,
            expected: BASELINE_VERSION,
        });
    }

    Ok(baseline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::SourceLine;
    use tempfile::TempDir;

    fn create_test_source_files() -> Vec<SourceFile> {
        let lines1 = vec![
            SourceLine::new("int x = 5;".to_string(), 1),
            SourceLine::new("int y = 10;".to_string(), 2),
            SourceLine::new("return x + y;".to_string(), 3),
        ];
        let lines2 = vec![
            SourceLine::new("int x = 5;".to_string(), 1),
            SourceLine::new("int y = 10;".to_string(), 2),
            SourceLine::new("return x + y;".to_string(), 3),
        ];

        vec![
            SourceFile::from_lines("a.c".to_string(), lines1),
            SourceFile::from_lines("b.c".to_string(), lines2),
        ]
    }

    #[test]
    fn test_baseline_entry_normalization() {
        let entry1 = BaselineEntry::new("b.c".to_string(), "a.c".to_string(), 123, 5);
        let entry2 = BaselineEntry::new("a.c".to_string(), "b.c".to_string(), 123, 5);

        // Both should have the same normalized order
        assert_eq!(entry1.file1, "a.c");
        assert_eq!(entry1.file2, "b.c");
        assert_eq!(entry1, entry2);
    }

    #[test]
    fn test_baseline_from_results() {
        let source_files = create_test_source_files();
        let result = DuploResult {
            blocks: vec![Block::new(0, 1, 0, 0, 3)],
            files_analyzed: 2,
            total_lines: 6,
            duplicate_lines: 3,
            duplicate_blocks: 1,
        };

        let baseline = Baseline::from_results(&result, &source_files, 12345);

        assert_eq!(baseline.version, BASELINE_VERSION);
        assert_eq!(baseline.config_hash, 12345);
        assert_eq!(baseline.entries.len(), 1);
    }

    #[test]
    fn test_baseline_save_load_roundtrip() {
        let temp = TempDir::new().unwrap();
        let baseline_path = temp.path().join("baseline.json");

        let source_files = create_test_source_files();
        let result = DuploResult {
            blocks: vec![Block::new(0, 1, 0, 0, 3)],
            files_analyzed: 2,
            total_lines: 6,
            duplicate_lines: 3,
            duplicate_blocks: 1,
        };

        let baseline = Baseline::from_results(&result, &source_files, 12345);
        save_baseline(&baseline, &baseline_path).unwrap();

        let loaded = load_baseline(&baseline_path).unwrap();
        assert_eq!(loaded.version, baseline.version);
        assert_eq!(loaded.config_hash, baseline.config_hash);
        assert_eq!(loaded.entries.len(), baseline.entries.len());
    }

    #[test]
    fn test_baseline_contains() {
        let source_files = create_test_source_files();
        let block = Block::new(0, 1, 0, 0, 3);

        let result = DuploResult {
            blocks: vec![block.clone()],
            files_analyzed: 2,
            total_lines: 6,
            duplicate_lines: 3,
            duplicate_blocks: 1,
        };

        let baseline = Baseline::from_results(&result, &source_files, 12345);

        // Same block should be found in baseline
        assert!(baseline.contains(&block, &source_files));

        // Different block should not be found
        let different_block = Block::new(0, 1, 1, 1, 2);
        assert!(!baseline.contains(&different_block, &source_files));
    }

    #[test]
    fn test_filter_new_duplicates() {
        let source_files = create_test_source_files();

        // Create baseline with one block
        let baseline_result = DuploResult {
            blocks: vec![Block::new(0, 1, 0, 0, 2)],
            files_analyzed: 2,
            total_lines: 6,
            duplicate_lines: 2,
            duplicate_blocks: 1,
        };
        let baseline = Baseline::from_results(&baseline_result, &source_files, 12345);

        // Create new result with two blocks (one existing, one new)
        let new_result = DuploResult {
            blocks: vec![
                Block::new(0, 1, 0, 0, 2), // Same as baseline
                Block::new(0, 1, 1, 1, 2), // New duplicate
            ],
            files_analyzed: 2,
            total_lines: 6,
            duplicate_lines: 4,
            duplicate_blocks: 2,
        };

        let filtered = baseline.filter_new_duplicates(new_result, &source_files);

        // Should only have the new block
        assert_eq!(filtered.duplicate_blocks, 1);
    }
}
