//! Export system for duplicate detection results

mod console;
mod json;
mod xml;

use crate::config::{Config, OutputFormat};
use crate::core::{DuploResult, SourceFile};
use crate::error::{DuploError, Result};
use std::fs::File;
use std::io::{self, BufWriter, Write};

pub use console::ConsoleExporter;
pub use json::JsonExporter;
pub use xml::XmlExporter;

/// Trait for output formatting
pub trait Exporter {
    /// Write the complete output for the given result
    fn export(
        &self,
        result: &DuploResult,
        source_files: &[SourceFile],
        config: &Config,
        writer: &mut dyn Write,
    ) -> Result<()>;
}

/// Create an appropriate exporter based on configuration
pub fn create_exporter(format: OutputFormat) -> Box<dyn Exporter> {
    match format {
        OutputFormat::Console => Box::new(ConsoleExporter),
        OutputFormat::Json => Box::new(JsonExporter),
        OutputFormat::Xml => Box::new(XmlExporter),
    }
}

/// Get a writer for the output (file or stdout)
pub fn get_output_writer(path: &str) -> Result<Box<dyn Write>> {
    if path == "-" {
        Ok(Box::new(BufWriter::new(io::stdout())))
    } else {
        let file = File::create(path).map_err(|e| DuploError::Io(e))?;
        Ok(Box::new(BufWriter::new(file)))
    }
}
