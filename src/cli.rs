//! CLI argument parsing using clap

use crate::config::{Config, OutputFormat};
use crate::error::{DuploError, Result};
use clap::Parser;

/// Code duplication detection tool
#[derive(Parser, Debug)]
#[command(name = "lucidshark-duplo")]
#[command(author = "Voldeq GmbH")]
#[command(version)]
#[command(about = "Detect code duplication in source files", long_about = None)]
pub struct Cli {
    /// Input file containing list of source files to analyze (one per line)
    /// Use "-" to read from stdin
    #[arg(value_name = "FILE_LIST")]
    pub file_list: String,

    /// Output file for results (use "-" for stdout)
    #[arg(value_name = "OUTPUT", default_value = "-")]
    pub output: String,

    /// Minimum block size in lines
    #[arg(short = 'm', long = "min-lines", value_name = "N", default_value = "4")]
    pub min_lines: u32,

    /// Block percentage threshold (0-100)
    #[arg(short = 'p', long = "percent", value_name = "N", default_value = "100")]
    pub percent: u8,

    /// Minimum characters per line
    #[arg(short = 'c', long = "min-chars", value_name = "N", default_value = "3")]
    pub min_chars: u32,

    /// Analyze only the first N files
    #[arg(short = 'n', long = "num-files", value_name = "N")]
    pub num_files: Option<usize>,

    /// Number of threads for parallel processing
    #[arg(short = 'j', long = "threads", value_name = "N")]
    pub threads: Option<usize>,

    /// Ignore file pairs with the same filename
    #[arg(short = 'd', long = "ignore-same-name")]
    pub ignore_same_name: bool,

    /// Output in JSON format
    #[arg(long = "json")]
    pub json: bool,

    /// Output in XML format
    #[arg(long = "xml")]
    pub xml: bool,
}

impl Cli {
    /// Parse command line arguments into a Config
    pub fn into_config(self) -> Result<Config> {
        // Check for conflicting output format options
        if self.json && self.xml {
            return Err(DuploError::OutputFormatConflict);
        }

        let output_format = if self.json {
            OutputFormat::Json
        } else if self.xml {
            OutputFormat::Xml
        } else {
            OutputFormat::Console
        };

        Ok(Config {
            min_chars: self.min_chars,
            min_block_size: self.min_lines,
            block_percent_threshold: self.percent,
            files_to_check: self.num_files.unwrap_or(0),
            num_threads: self.threads.unwrap_or_else(num_cpus::get),
            output_format,
            ignore_same_filename: self.ignore_same_name,
            list_filename: self.file_list,
            output_filename: self.output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_values() {
        let cli = Cli::parse_from(["duplo", "files.txt"]);
        let config = cli.into_config().unwrap();

        assert_eq!(config.min_block_size, 4);
        assert_eq!(config.min_chars, 3);
        assert_eq!(config.block_percent_threshold, 100);
        assert!(!config.ignore_same_filename);
        assert_eq!(config.output_format, OutputFormat::Console);
    }

    #[test]
    fn test_cli_json_output() {
        let cli = Cli::parse_from(["duplo", "--json", "files.txt"]);
        let config = cli.into_config().unwrap();

        assert_eq!(config.output_format, OutputFormat::Json);
    }

    #[test]
    fn test_cli_xml_output() {
        let cli = Cli::parse_from(["duplo", "--xml", "files.txt"]);
        let config = cli.into_config().unwrap();

        assert_eq!(config.output_format, OutputFormat::Xml);
    }

    #[test]
    fn test_cli_conflicting_output() {
        let cli = Cli::parse_from(["duplo", "--json", "--xml", "files.txt"]);
        let result = cli.into_config();

        assert!(matches!(result, Err(DuploError::OutputFormatConflict)));
    }

    #[test]
    fn test_cli_all_options() {
        let cli = Cli::parse_from([
            "duplo",
            "-m",
            "10",
            "-p",
            "50",
            "-c",
            "5",
            "-n",
            "100",
            "-j",
            "4",
            "-d",
            "--json",
            "files.txt",
            "output.json",
        ]);
        let config = cli.into_config().unwrap();

        assert_eq!(config.min_block_size, 10);
        assert_eq!(config.block_percent_threshold, 50);
        assert_eq!(config.min_chars, 5);
        assert_eq!(config.files_to_check, 100);
        assert_eq!(config.num_threads, 4);
        assert!(config.ignore_same_filename);
        assert_eq!(config.output_format, OutputFormat::Json);
        assert_eq!(config.list_filename, "files.txt");
        assert_eq!(config.output_filename, "output.json");
    }
}
