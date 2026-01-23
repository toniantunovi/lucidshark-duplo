//! lucidshark-duplo - Code duplication detection tool
//!
//! A Rust reimplementation of Duplo with modernized output formats
//! and expanded language support.

mod cli;
mod config;
mod core;
mod error;
mod export;
mod filetype;

use clap::Parser;
use cli::Cli;
use core::process_files;
use export::{create_exporter, get_output_writer};
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

    // Process files
    let (result, source_files) = match process_files(&config, progress) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            return ExitCode::from(2);
        }
    };

    // Create exporter and writer
    let exporter = create_exporter(config.output_format);
    let mut writer = match get_output_writer(&config.output_filename) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error creating output: {}", e);
            return ExitCode::from(2);
        }
    };

    // Export results
    if let Err(e) = exporter.export(&result, &source_files, &config, &mut *writer) {
        eprintln!("Error writing output: {}", e);
        return ExitCode::from(2);
    }

    // Flush output
    if let Err(e) = writer.flush() {
        eprintln!("Error flushing output: {}", e);
        return ExitCode::from(2);
    }

    // Return appropriate exit code
    if result.duplicate_blocks > 0 {
        ExitCode::from(1) // Duplicates found
    } else {
        ExitCode::SUCCESS // No duplicates
    }
}
