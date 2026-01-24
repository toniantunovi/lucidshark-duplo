//! Console (human-readable) exporter

use crate::config::Config;
use crate::core::{DuploResult, SourceFile};
use crate::error::Result;
use crate::export::Exporter;
use std::io::Write;

/// Human-readable console output exporter
pub struct ConsoleExporter;

impl Exporter for ConsoleExporter {
    fn export(
        &self,
        result: &DuploResult,
        source_files: &[SourceFile],
        config: &Config,
        writer: &mut dyn Write,
    ) -> Result<()> {
        // Output each duplicate block
        for block in &result.blocks {
            let source1 = &source_files[block.source1_idx];
            let source2 = &source_files[block.source2_idx];

            // Get original line numbers
            let start1 = source1.get_line(block.line1).line_number();
            let end1 = source1
                .get_line(block.line1 + block.count - 1)
                .line_number();
            let start2 = source2.get_line(block.line2).line_number();
            let end2 = source2
                .get_line(block.line2 + block.count - 1)
                .line_number();

            writeln!(
                writer,
                "{}({}-{}) <-> {}({}-{})",
                source1.filename(),
                start1,
                end1,
                source2.filename(),
                start2,
                end2
            )?;

            // Output the duplicate lines (indented)
            let lines = source1.get_lines(block.line1, block.line1 + block.count);
            for line in lines {
                writeln!(writer, "    {}", line)?;
            }
            writeln!(writer)?;
        }

        // Output summary
        writeln!(writer, "Configuration:")?;
        writeln!(
            writer,
            "  Minimum block size: {} lines",
            config.min_block_size
        )?;
        writeln!(
            writer,
            "  Minimum characters per line: {}",
            config.min_chars
        )?;
        writeln!(
            writer,
            "  Block percentage threshold: {}%",
            config.block_percent_threshold
        )?;
        writeln!(writer)?;

        writeln!(writer, "Summary:")?;
        writeln!(writer, "  Files analyzed: {}", result.files_analyzed)?;
        writeln!(writer, "  Total lines: {}", result.total_lines)?;
        writeln!(writer, "  Duplicate blocks: {}", result.duplicate_blocks)?;
        writeln!(writer, "  Duplicate lines: {}", result.duplicate_lines)?;
        if result.total_lines > 0 {
            let percent = (result.duplicate_lines as f64 / result.total_lines as f64) * 100.0;
            writeln!(writer, "  Duplication: {:.1}%", percent)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Block, SourceLine};

    #[test]
    fn test_console_export() {
        let lines1 = vec![
            SourceLine::new("line1".to_string(), 1),
            SourceLine::new("line2".to_string(), 2),
            SourceLine::new("line3".to_string(), 3),
            SourceLine::new("line4".to_string(), 4),
        ];
        let lines2 = lines1.clone();

        let sf1 = SourceFile::from_lines("a.c".to_string(), lines1);
        let sf2 = SourceFile::from_lines("b.c".to_string(), lines2);
        let source_files = vec![sf1, sf2];

        let result = DuploResult {
            blocks: vec![Block::new(0, 1, 0, 0, 4)],
            files_analyzed: 2,
            total_lines: 8,
            duplicate_lines: 4,
            duplicate_blocks: 1,
        };

        let config = Config::default();
        let exporter = ConsoleExporter;
        let mut output = Vec::new();

        exporter
            .export(&result, &source_files, &config, &mut output)
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("a.c(1-4) <-> b.c(1-4)"));
        assert!(output_str.contains("Duplicate blocks: 1"));
    }
}
