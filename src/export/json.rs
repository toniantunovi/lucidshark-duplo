//! JSON exporter

use crate::config::Config;
use crate::core::{DuploResult, SourceFile};
use crate::error::Result;
use crate::export::Exporter;
use serde::Serialize;
use std::io::Write;

/// JSON output exporter
pub struct JsonExporter;

#[derive(Serialize)]
struct JsonOutput {
    duplicates: Vec<JsonDuplicate>,
    summary: JsonSummary,
}

#[derive(Serialize)]
struct JsonDuplicate {
    line_count: usize,
    file1: JsonFileRef,
    file2: JsonFileRef,
    lines: Vec<String>,
}

#[derive(Serialize)]
struct JsonFileRef {
    path: String,
    start_line: usize,
    end_line: usize,
}

#[derive(Serialize)]
struct JsonSummary {
    files_analyzed: usize,
    total_lines: usize,
    duplicate_blocks: usize,
    duplicate_lines: usize,
    duplication_percent: f64,
}

impl Exporter for JsonExporter {
    fn export(
        &self,
        result: &DuploResult,
        source_files: &[SourceFile],
        _config: &Config,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let duplicates: Vec<JsonDuplicate> = result
            .blocks
            .iter()
            .map(|block| {
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

                let lines: Vec<String> = source1
                    .get_lines(block.line1, block.line1 + block.count)
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();

                JsonDuplicate {
                    line_count: block.count,
                    file1: JsonFileRef {
                        path: source1.filename().to_string(),
                        start_line: start1,
                        end_line: end1,
                    },
                    file2: JsonFileRef {
                        path: source2.filename().to_string(),
                        start_line: start2,
                        end_line: end2,
                    },
                    lines,
                }
            })
            .collect();

        let duplication_percent = if result.total_lines > 0 {
            (result.duplicate_lines as f64 / result.total_lines as f64) * 100.0
        } else {
            0.0
        };

        let output = JsonOutput {
            duplicates,
            summary: JsonSummary {
                files_analyzed: result.files_analyzed,
                total_lines: result.total_lines,
                duplicate_blocks: result.duplicate_blocks,
                duplicate_lines: result.duplicate_lines,
                duplication_percent,
            },
        };

        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| crate::error::DuploError::Other(e.to_string()))?;
        writeln!(writer, "{}", json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Block, SourceLine};

    #[test]
    fn test_json_export() {
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
        let exporter = JsonExporter;
        let mut output = Vec::new();

        exporter
            .export(&result, &source_files, &config, &mut output)
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

        assert_eq!(parsed["summary"]["files_analyzed"], 2);
        assert_eq!(parsed["duplicates"].as_array().unwrap().len(), 1);
    }
}
