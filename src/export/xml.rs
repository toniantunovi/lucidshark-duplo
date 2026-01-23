//! XML exporter

use crate::config::Config;
use crate::core::{DuploResult, SourceFile};
use crate::error::Result;
use crate::export::Exporter;
use std::io::Write;

/// XML output exporter
pub struct XmlExporter;

impl XmlExporter {
    /// Escape special XML characters
    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

impl Exporter for XmlExporter {
    fn export(
        &self,
        result: &DuploResult,
        source_files: &[SourceFile],
        _config: &Config,
        writer: &mut dyn Write,
    ) -> Result<()> {
        writeln!(writer, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        writeln!(writer, "<duplo>")?;

        // Output each duplicate block as a set
        for block in &result.blocks {
            let source1 = &source_files[block.source1_idx];
            let source2 = &source_files[block.source2_idx];

            let start1 = source1.get_line(block.line1).line_number();
            let end1 = source1
                .get_line(block.line1 + block.count - 1)
                .line_number();
            let start2 = source2.get_line(block.line2).line_number();
            let end2 = source2
                .get_line(block.line2 + block.count - 1)
                .line_number();

            writeln!(writer, r#"  <set LineCount="{}">"#, block.count)?;
            writeln!(
                writer,
                r#"    <block SourceFile="{}" StartLineNumber="{}" EndLineNumber="{}"/>"#,
                Self::escape_xml(source1.filename()),
                start1,
                end1
            )?;
            writeln!(
                writer,
                r#"    <block SourceFile="{}" StartLineNumber="{}" EndLineNumber="{}"/>"#,
                Self::escape_xml(source2.filename()),
                start2,
                end2
            )?;

            writeln!(writer, r#"    <lines xml:space="preserve">"#)?;
            let lines = source1.get_lines(block.line1, block.line1 + block.count);
            for line in lines {
                writeln!(writer, r#"      <line Text="{}"/>"#, Self::escape_xml(line))?;
            }
            writeln!(writer, "    </lines>")?;
            writeln!(writer, "  </set>")?;
        }

        // Summary element
        writeln!(writer, "  <summary")?;
        writeln!(writer, r#"    FilesAnalyzed="{}""#, result.files_analyzed)?;
        writeln!(writer, r#"    TotalLines="{}""#, result.total_lines)?;
        writeln!(
            writer,
            r#"    DuplicateBlocks="{}""#,
            result.duplicate_blocks
        )?;
        writeln!(writer, r#"    DuplicateLines="{}""#, result.duplicate_lines)?;
        if result.total_lines > 0 {
            let percent = (result.duplicate_lines as f64 / result.total_lines as f64) * 100.0;
            writeln!(writer, r#"    DuplicationPercent="{:.1}""#, percent)?;
        }
        writeln!(writer, "  />")?;

        writeln!(writer, "</duplo>")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Block, SourceLine};

    #[test]
    fn test_xml_export() {
        let lines1 = vec![
            SourceLine::new("line1".to_string(), 1),
            SourceLine::new("line2".to_string(), 2),
        ];
        let lines2 = lines1.clone();

        let sf1 = SourceFile::from_lines("a.c".to_string(), lines1);
        let sf2 = SourceFile::from_lines("b.c".to_string(), lines2);
        let source_files = vec![sf1, sf2];

        let result = DuploResult {
            blocks: vec![Block::new(0, 1, 0, 0, 2)],
            files_analyzed: 2,
            total_lines: 4,
            duplicate_lines: 2,
            duplicate_blocks: 1,
        };

        let config = Config::default();
        let exporter = XmlExporter;
        let mut output = Vec::new();

        exporter
            .export(&result, &source_files, &config, &mut output)
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("<?xml version"));
        assert!(output_str.contains("<duplo>"));
        assert!(output_str.contains("</duplo>"));
        assert!(output_str.contains(r#"LineCount="2""#));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(XmlExporter::escape_xml("a < b"), "a &lt; b");
        assert_eq!(XmlExporter::escape_xml("a & b"), "a &amp; b");
        assert_eq!(XmlExporter::escape_xml(r#"a "b""#), "a &quot;b&quot;");
    }
}
