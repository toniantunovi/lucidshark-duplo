//! lucidshark-duplo - Code duplication detection tool
//!
//! A fast, feature-rich code duplication detector with git integration,
//! incremental caching, baseline comparison, and multi-language support.

mod baseline;
mod cache;
mod cli;
mod config;
mod core;
mod error;
mod export;
mod filetype;
mod git;

use baseline::{load_baseline, save_baseline, Baseline};
use cache::{clear_cache, FileCache};
use clap::Parser;
use cli::Cli;
use core::{load_file_list, process_files_with_cache, DuploResult, SourceFile};
use export::{create_exporter, get_output_writer};
use std::collections::HashSet;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse command line arguments
    let cli = Cli::parse();

    // Convert to config
    let config = match cli.into_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            return ExitCode::from(2);
        }
    };

    // Progress callback for logging
    let progress = |msg: &str| {
        eprintln!("{}", msg);
    };

    // === Phase 0: Handle --clear-cache ===
    if config.clear_cache {
        progress("Clearing cache...");
        if let Err(e) = clear_cache(&config) {
            eprintln!("Warning: Failed to clear cache: {}", e);
        }
    }

    // === Phase 1: File Discovery ===
    let (file_list, changed_files) = if config.git_mode {
        match git::discover_files_with_changed_set(&config, &progress) {
            Ok(result) => (result.files, result.changed_files),
            Err(e) => {
                eprintln!("Error: {}", e);
                return ExitCode::from(2);
            }
        }
    } else {
        match &config.list_filename {
            Some(path) => match load_file_list(path) {
                Ok(files) => (files, None),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return ExitCode::from(2);
                }
            },
            None => {
                eprintln!("Error: No file list provided. Use --git or provide a file list.");
                return ExitCode::from(2);
            }
        }
    };

    // === Phase 1.5: Setup Cache ===
    let cache = if config.cache_enabled {
        match FileCache::new(&config) {
            Ok(c) => {
                progress("Caching enabled");
                Some(c)
            }
            Err(e) => {
                eprintln!("Warning: Failed to initialize cache: {}", e);
                None
            }
        }
    } else {
        None
    };

    // === Phase 2: Process Files ===
    let (result, source_files) =
        match process_files_with_cache(&file_list, &config, cache.as_ref(), progress) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: {}", e);
                return ExitCode::from(2);
            }
        };

    // === Phase 3: Filter Results (for --changed-only) ===
    let result = if let Some(changed_set) = changed_files {
        filter_to_changed_files(result, &source_files, &changed_set)
    } else {
        result
    };

    // === Phase 3.5: Load and Apply Baseline ===
    let baseline = if let Some(ref baseline_path) = config.baseline_path {
        match load_baseline(baseline_path) {
            Ok(b) => {
                // Warn if config hash differs
                if b.config_hash != config.detection_config_hash() {
                    eprintln!(
                        "Warning: Baseline was created with different detection settings. \
                         Results may not be comparable."
                    );
                }
                progress(&format!(
                    "Loaded baseline with {} known duplicates",
                    b.entries.len()
                ));
                Some(b)
            }
            Err(e) => {
                eprintln!("Error loading baseline: {}", e);
                return ExitCode::from(2);
            }
        }
    } else {
        None
    };

    // Filter to only new duplicates if baseline is provided
    let result = if let Some(ref baseline) = baseline {
        let filtered = baseline.filter_new_duplicates(result, &source_files);
        progress(&format!(
            "Found {} NEW duplicate blocks (filtered from baseline)",
            filtered.duplicate_blocks
        ));
        filtered
    } else {
        result
    };

    // === Phase 4: Export Results ===
    let exporter = create_exporter(config.output_format);
    let mut writer = match get_output_writer(&config.output_filename) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error creating output: {}", e);
            return ExitCode::from(2);
        }
    };

    if let Err(e) = exporter.export(&result, &source_files, &config, &mut *writer) {
        eprintln!("Error writing output: {}", e);
        return ExitCode::from(2);
    }

    if let Err(e) = writer.flush() {
        eprintln!("Error flushing output: {}", e);
        return ExitCode::from(2);
    }

    // === Phase 4.5: Save Baseline ===
    if let Some(ref save_path) = config.save_baseline_path {
        let new_baseline =
            Baseline::from_results(&result, &source_files, config.detection_config_hash());
        if let Err(e) = save_baseline(&new_baseline, save_path) {
            eprintln!("Error saving baseline: {}", e);
            return ExitCode::from(2);
        }
        progress(&format!(
            "Saved baseline with {} duplicates to '{}'",
            new_baseline.entries.len(),
            save_path.display()
        ));
    }

    // === Phase 5: Exit Code ===
    if result.duplicate_blocks > 0 {
        ExitCode::from(1) // Duplicates found
    } else {
        ExitCode::SUCCESS // No duplicates
    }
}

/// Filter duplicate results to only include blocks where at least one file is in the changed set
fn filter_to_changed_files(
    result: DuploResult,
    source_files: &[SourceFile],
    changed_files: &HashSet<String>,
) -> DuploResult {
    let filtered_blocks: Vec<_> = result
        .blocks
        .into_iter()
        .filter(|block| {
            let file1 = source_files[block.source1_idx].filename();
            let file2 = source_files[block.source2_idx].filename();
            changed_files.contains(file1) || changed_files.contains(file2)
        })
        .collect();

    let duplicate_lines: usize = filtered_blocks.iter().map(|b| b.count).sum();
    let duplicate_blocks = filtered_blocks.len();

    DuploResult {
        blocks: filtered_blocks,
        files_analyzed: result.files_analyzed,
        total_lines: result.total_lines,
        duplicate_lines,
        duplicate_blocks,
    }
}
