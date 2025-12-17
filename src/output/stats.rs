use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;

use crate::counter::LineStats;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct FileStatistics {
    pub path: PathBuf,
    pub stats: LineStats,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectStatistics {
    pub files: Vec<FileStatistics>,
    pub total_files: usize,
    pub total_lines: usize,
    pub total_code: usize,
    pub total_comment: usize,
    pub total_blank: usize,
}

impl ProjectStatistics {
    #[must_use]
    pub fn new(files: Vec<FileStatistics>) -> Self {
        let total_files = files.len();
        let (total_lines, total_code, total_comment, total_blank) =
            files.iter().fold((0, 0, 0, 0), |acc, f| {
                (
                    acc.0 + f.stats.total,
                    acc.1 + f.stats.code,
                    acc.2 + f.stats.comment,
                    acc.3 + f.stats.blank,
                )
            });

        Self {
            files,
            total_files,
            total_lines,
            total_code,
            total_comment,
            total_blank,
        }
    }
}

pub trait StatsFormatter {
    /// Format the project statistics into a string.
    ///
    /// # Errors
    /// Returns an error if the formatting fails.
    fn format(&self, stats: &ProjectStatistics) -> Result<String>;
}

pub struct StatsTextFormatter;

impl StatsFormatter for StatsTextFormatter {
    fn format(&self, stats: &ProjectStatistics) -> Result<String> {
        let mut output = Vec::new();

        for file in &stats.files {
            writeln!(
                output,
                "{}: {} lines (code={}, comment={}, blank={})",
                file.path.display(),
                file.stats.total,
                file.stats.code,
                file.stats.comment,
                file.stats.blank
            )
            .ok();
        }

        if !stats.files.is_empty() {
            writeln!(output).ok();
        }

        writeln!(output, "Summary:").ok();
        writeln!(output, "  Files: {}", stats.total_files).ok();
        writeln!(output, "  Total lines: {}", stats.total_lines).ok();
        writeln!(output, "  Code: {}", stats.total_code).ok();
        writeln!(output, "  Comments: {}", stats.total_comment).ok();
        writeln!(output, "  Blank: {}", stats.total_blank).ok();

        Ok(String::from_utf8_lossy(&output).to_string())
    }
}

pub struct StatsJsonFormatter;

#[derive(Serialize)]
struct JsonStatsOutput {
    summary: JsonStatsSummary,
    files: Vec<JsonFileStats>,
}

#[derive(Serialize)]
struct JsonStatsSummary {
    total_files: usize,
    total_lines: usize,
    code: usize,
    comment: usize,
    blank: usize,
}

#[derive(Serialize)]
struct JsonFileStats {
    path: String,
    total: usize,
    code: usize,
    comment: usize,
    blank: usize,
}

impl StatsFormatter for StatsJsonFormatter {
    fn format(&self, stats: &ProjectStatistics) -> Result<String> {
        let output = JsonStatsOutput {
            summary: JsonStatsSummary {
                total_files: stats.total_files,
                total_lines: stats.total_lines,
                code: stats.total_code,
                comment: stats.total_comment,
                blank: stats.total_blank,
            },
            files: stats
                .files
                .iter()
                .map(|f| JsonFileStats {
                    path: f.path.display().to_string(),
                    total: f.stats.total,
                    code: f.stats.code,
                    comment: f.stats.comment,
                    blank: f.stats.blank,
                })
                .collect(),
        };

        Ok(serde_json::to_string_pretty(&output)?)
    }
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
