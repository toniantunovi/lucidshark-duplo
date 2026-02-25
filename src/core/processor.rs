//! Core duplicate detection algorithm
//!
//! This module implements the LCS-based matrix algorithm for detecting
//! code duplicates, ported from the C++ Duplo implementation.

use crate::cache::FileCache;
use crate::config::Config;
use crate::core::{Block, SourceFile};

#[cfg(test)]
use crate::core::SourceLine;
use crate::error::{DuploError, Result};
use bitvec::prelude::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Result of duplicate detection
#[derive(Debug)]
pub struct DuploResult {
    /// All detected duplicate blocks
    pub blocks: Vec<Block>,
    /// Total number of files analyzed
    pub files_analyzed: usize,
    /// Total lines of code analyzed
    pub total_lines: usize,
    /// Total duplicate lines found
    pub duplicate_lines: usize,
    /// Total duplicate blocks found
    pub duplicate_blocks: usize,
}

/// Maps line hashes to file indices that contain that line
type HashToFiles = HashMap<u32, Vec<usize>>;

/// Thread-local context for processing
struct ThreadContext {
    /// Reusable boolean matrix for line comparison
    matrix: BitVec,
}

impl ThreadContext {
    fn new(max_lines: usize) -> Self {
        Self {
            matrix: bitvec![0; max_lines * max_lines],
        }
    }

    /// Reset matrix for a new comparison
    fn reset_matrix(&mut self, m: usize, n: usize) {
        // Only clear the portion we'll use
        let size = m * n;
        self.matrix[..size].fill(false);
    }
}

/// Load file list from path (or stdin if "-")
pub fn load_file_list(path: &str) -> Result<Vec<String>> {
    let lines = if path == "-" {
        let stdin = std::io::stdin();
        stdin.lock().lines().collect::<std::io::Result<Vec<_>>>()?
    } else {
        let file = File::open(path).map_err(|e| DuploError::FileNotFound {
            path: path.to_string(),
            reason: e.to_string(),
        })?;
        BufReader::new(file)
            .lines()
            .collect::<std::io::Result<Vec<_>>>()?
    };

    // Filter out short lines and whitespace-only lines
    Ok(lines.into_iter().filter(|l| l.trim().len() > 5).collect())
}

/// Load all source files from the file list (without caching)
#[allow(dead_code)]
fn load_source_files(
    file_list: &[String],
    config: &Config,
    progress: &impl Fn(&str),
) -> Result<(Vec<SourceFile>, usize)> {
    load_source_files_with_cache(file_list, config, None, progress)
}

/// Load all source files from the file list with optional caching
fn load_source_files_with_cache(
    file_list: &[String],
    config: &Config,
    cache: Option<&FileCache>,
    progress: &impl Fn(&str),
) -> Result<(Vec<SourceFile>, usize)> {
    let mut source_files = Vec::new();
    let mut max_lines = 0usize;
    let mut cache_hits = 0usize;

    for path in file_list {
        // Try to load from cache first
        if let Some(cache) = cache {
            if let Some(lines) = cache.get(path) {
                let sf = SourceFile::from_cached_lines(path.clone(), lines);
                let num_lines = sf.num_lines();
                if num_lines > 0 {
                    max_lines = max_lines.max(num_lines);
                    source_files.push(sf);
                    cache_hits += 1;
                }
                continue;
            }
        }

        // Load from disk
        match SourceFile::load(path, config.min_chars) {
            Ok(sf) => {
                let num_lines = sf.num_lines();
                if num_lines > 0 {
                    // Save to cache if enabled
                    if let Some(cache) = cache {
                        if let Err(e) = cache.put(path, sf.lines_slice()) {
                            progress(&format!("Warning: Failed to cache '{}': {}", path, e));
                        }
                    }
                    max_lines = max_lines.max(num_lines);
                    source_files.push(sf);
                }
            }
            Err(e) => {
                // Log warning but continue
                progress(&format!("Warning: {}", e));
            }
        }
    }

    if cache.is_some() && cache_hits > 0 {
        progress(&format!(
            "Cache: {} hits, {} misses",
            cache_hits,
            source_files.len() - cache_hits
        ));
    }

    // Validate memory requirements
    // Limit to ~1GB of matrix memory per thread (8 billion bits = 1GB)
    const MAX_BITS_PER_THREAD: usize = 8_000_000_000;
    let max_matrix_size = MAX_BITS_PER_THREAD;
    let required_size = max_lines * max_lines;
    if required_size > max_matrix_size {
        // Find the longest files for the error message
        let mut sorted: Vec<_> = source_files.iter().collect();
        sorted.sort_by_key(|f| std::cmp::Reverse(f.num_lines()));

        return Err(DuploError::FileTooLarge {
            path: sorted[0].filename().to_string(),
            lines: max_lines,
            threads: config.num_threads,
            max_lines: ((max_matrix_size / config.num_threads) as f64).sqrt() as usize,
        });
    }

    Ok((source_files, max_lines))
}

/// Build hash-to-files index for optimization
fn build_hash_index(source_files: &[SourceFile]) -> HashToFiles {
    let mut index: HashToFiles = HashMap::new();

    for (file_idx, sf) in source_files.iter().enumerate() {
        for line in sf.lines() {
            index.entry(line.hash()).or_default().push(file_idx);
        }
    }

    index
}

/// Get set of file indices that share at least one line hash with the given file
fn get_matching_files(source_file: &SourceFile, hash_index: &HashToFiles) -> HashSet<usize> {
    let mut matching = HashSet::new();

    for line in source_file.lines() {
        if let Some(files) = hash_index.get(&line.hash()) {
            matching.extend(files.iter().copied());
        }
    }

    matching
}

/// Calculate the effective minimum block size
fn calc_min_block_size(config: &Config, m: usize, n: usize) -> usize {
    let min_from_threshold = if config.block_percent_threshold > 0 {
        (m.max(n) * 100) / config.block_percent_threshold as usize
    } else {
        0
    };

    (config.min_block_size as usize).max((config.min_block_size as usize).min(min_from_threshold))
}

/// Process a pair of files and find duplicates
fn process_file_pair(
    source1: &SourceFile,
    source2: &SourceFile,
    source1_idx: usize,
    source2_idx: usize,
    config: &Config,
    context: &mut ThreadContext,
) -> Vec<Block> {
    let m = source1.num_lines();
    let n = source2.num_lines();

    if m == 0 || n == 0 {
        return Vec::new();
    }

    // Reset and build matrix
    context.reset_matrix(m, n);

    for y in 0..m {
        let line1 = source1.get_line(y);
        for x in 0..n {
            if *line1 == *source2.get_line(x) {
                context.matrix.set(x + n * y, true);
            }
        }
    }

    let min_block_size = calc_min_block_size(config, m, n);
    let mut blocks = Vec::new();

    let is_same_file = source1_idx == source2_idx;

    // Vertical diagonal scan
    for y in 0..m {
        let mut seq_len = 0usize;
        let max_x = n.min(m - y);

        for x in 0..max_x {
            if context.matrix[x + n * (y + x)] {
                seq_len += 1;
            } else {
                if seq_len >= min_block_size {
                    let line1 = y + x - seq_len;
                    let line2 = x - seq_len;
                    // For self-comparison, only report if positions differ
                    if !is_same_file || line1 != line2 {
                        blocks.push(Block::new(source1_idx, source2_idx, line1, line2, seq_len));
                    }
                }
                seq_len = 0;
            }
        }

        // Check for sequence at end
        if seq_len >= min_block_size {
            let line1 = m - seq_len;
            let line2 = n.min(m - y) - seq_len;
            if !is_same_file || line1 != line2 {
                blocks.push(Block::new(source1_idx, source2_idx, line1, line2, seq_len));
            }
        }
    }

    // Horizontal diagonal scan (only for different files)
    if !is_same_file {
        for x in 1..n {
            let mut seq_len = 0usize;
            let max_y = m.min(n - x);

            for y in 0..max_y {
                if context.matrix[x + y + n * y] {
                    seq_len += 1;
                } else {
                    if seq_len >= min_block_size {
                        blocks.push(Block::new(
                            source1_idx,
                            source2_idx,
                            y - seq_len,
                            x + y - seq_len,
                            seq_len,
                        ));
                    }
                    seq_len = 0;
                }
            }

            if seq_len >= min_block_size {
                blocks.push(Block::new(
                    source1_idx,
                    source2_idx,
                    m - seq_len,
                    n - seq_len,
                    seq_len,
                ));
            }
        }
    }

    blocks
}

/// Main entry point for processing files from a file list path.
/// For git-based discovery, use `process_files_with_list` instead.
#[allow(dead_code)]
pub fn process_files(
    config: &Config,
    progress: impl Fn(&str) + Send + Sync,
) -> Result<(DuploResult, Vec<SourceFile>)> {
    let file_list = match &config.list_filename {
        Some(path) => load_file_list(path)?,
        None => {
            return Err(DuploError::InvalidConfig(
                "No file list provided. Use --git or provide a file list.".to_string(),
            ))
        }
    };

    process_files_with_list(&file_list, config, progress)
}

/// Process files from a pre-resolved file list.
/// This is the main processing function that handles duplicate detection.
#[allow(dead_code)]
pub fn process_files_with_list(
    file_list: &[String],
    config: &Config,
    progress: impl Fn(&str) + Send + Sync,
) -> Result<(DuploResult, Vec<SourceFile>)> {
    process_files_with_cache(file_list, config, None, progress)
}

/// Process files from a pre-resolved file list with optional caching.
/// This is the main processing function that handles duplicate detection.
pub fn process_files_with_cache(
    file_list: &[String],
    config: &Config,
    cache: Option<&FileCache>,
    progress: impl Fn(&str) + Send + Sync,
) -> Result<(DuploResult, Vec<SourceFile>)> {
    progress("Loading and hashing files...");

    // Load source files (with optional cache)
    let (source_files, max_lines) =
        load_source_files_with_cache(file_list, config, cache, &progress)?;

    if source_files.is_empty() {
        return Ok((
            DuploResult {
                blocks: Vec::new(),
                files_analyzed: 0,
                total_lines: 0,
                duplicate_lines: 0,
                duplicate_blocks: 0,
            },
            source_files,
        ));
    }

    progress(&format!(
        "Loaded {} files, {} total lines",
        source_files.len(),
        source_files.iter().map(|f| f.num_lines()).sum::<usize>()
    ));

    // Build hash index
    let hash_index = build_hash_index(&source_files);

    // Determine how many files to check
    let files_to_check = config.effective_files_to_check().min(source_files.len());

    // Set up thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.num_threads)
        .build()
        .map_err(|e| DuploError::Other(format!("Failed to create thread pool: {}", e)))?;

    // Process files in parallel
    let results: Vec<Vec<Block>> = pool.install(|| {
        (0..files_to_check)
            .into_par_iter()
            .map(|i| {
                let source1 = &source_files[i];
                let matching = get_matching_files(source1, &hash_index);
                let mut context = ThreadContext::new(max_lines);
                let mut all_blocks = Vec::new();

                // Compare with self
                let self_blocks = process_file_pair(source1, source1, i, i, config, &mut context);
                all_blocks.extend(self_blocks);

                // Compare with subsequent files
                for (j, source2) in source_files.iter().enumerate().skip(i + 1) {
                    // Skip if configured to ignore same filename
                    if config.ignore_same_filename && source1.has_same_basename(source2) {
                        continue;
                    }

                    // Skip if no matching lines
                    if !matching.contains(&j) {
                        continue;
                    }

                    let blocks = process_file_pair(source1, source2, i, j, config, &mut context);
                    all_blocks.extend(blocks);
                }

                all_blocks
            })
            .collect()
    });

    // Aggregate results
    let all_blocks: Vec<Block> = results.into_iter().flatten().collect();
    let duplicate_lines: usize = all_blocks.iter().map(|b| b.count).sum();
    let duplicate_blocks = all_blocks.len();
    let total_lines: usize = source_files.iter().map(|f| f.num_lines()).sum();

    Ok((
        DuploResult {
            blocks: all_blocks,
            files_analyzed: files_to_check,
            total_lines,
            duplicate_lines,
            duplicate_blocks,
        },
        source_files,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_min_block_size() {
        let mut config = Config::default();
        config.min_block_size = 4;
        config.block_percent_threshold = 100;

        // With 100% threshold, should just return min_block_size
        assert_eq!(calc_min_block_size(&config, 100, 100), 4);

        // With lower threshold, might return larger value
        config.block_percent_threshold = 10;
        let result = calc_min_block_size(&config, 100, 100);
        assert!(result >= 4);
    }

    #[test]
    fn test_build_hash_index() {
        let lines1 = vec![
            SourceLine::new("int x = 5;".to_string(), 1),
            SourceLine::new("int y = 10;".to_string(), 2),
        ];
        let lines2 = vec![
            SourceLine::new("int x = 5;".to_string(), 1), // duplicate
            SourceLine::new("int z = 15;".to_string(), 2),
        ];
        let sf1 = SourceFile::from_lines("a.c".to_string(), lines1);
        let sf2 = SourceFile::from_lines("b.c".to_string(), lines2);
        let files = vec![sf1, sf2];

        let index = build_hash_index(&files);

        // The hash of "int x = 5;" should map to both files
        let hash = crate::core::hash_line("int x = 5;");
        let files_with_hash = index.get(&hash).unwrap();
        assert!(files_with_hash.contains(&0));
        assert!(files_with_hash.contains(&1));
    }

    #[test]
    fn test_process_identical_files() {
        let lines = vec![
            SourceLine::new("line1".to_string(), 1),
            SourceLine::new("line2".to_string(), 2),
            SourceLine::new("line3".to_string(), 3),
            SourceLine::new("line4".to_string(), 4),
            SourceLine::new("line5".to_string(), 5),
        ];
        let sf1 = SourceFile::from_lines("a.c".to_string(), lines.clone());
        let sf2 = SourceFile::from_lines("b.c".to_string(), lines);

        let mut config = Config::default();
        config.min_block_size = 4;

        let mut context = ThreadContext::new(10);
        let blocks = process_file_pair(&sf1, &sf2, 0, 1, &config, &mut context);

        // Should find one block of 5 lines
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].count, 5);
    }

    #[test]
    fn test_process_no_duplicates() {
        let lines1 = vec![
            SourceLine::new("aaa".to_string(), 1),
            SourceLine::new("bbb".to_string(), 2),
        ];
        let lines2 = vec![
            SourceLine::new("ccc".to_string(), 1),
            SourceLine::new("ddd".to_string(), 2),
        ];
        let sf1 = SourceFile::from_lines("a.c".to_string(), lines1);
        let sf2 = SourceFile::from_lines("b.c".to_string(), lines2);

        let config = Config::default();
        let mut context = ThreadContext::new(10);
        let blocks = process_file_pair(&sf1, &sf2, 0, 1, &config, &mut context);

        assert!(blocks.is_empty());
    }
}
