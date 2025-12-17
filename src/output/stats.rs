use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;

use crate::counter::LineStats;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct FileStatistics {
    pub path: PathBuf,
    pub stats: LineStats,
    pub language: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LanguageStats {
    pub language: String,
    pub files: usize,
    pub total_lines: usize,
    pub code: usize,
    pub comment: usize,
    pub blank: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectStatistics {
    pub files: Vec<FileStatistics>,
    pub total_files: usize,
    pub total_lines: usize,
    pub total_code: usize,
    pub total_comment: usize,
    pub total_blank: usize,
    pub by_language: Option<Vec<LanguageStats>>,
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
            by_language: None,
        }
    }

    #[must_use]
    pub fn with_language_breakdown(mut self) -> Self {
        let mut lang_map: HashMap<String, LanguageStats> = HashMap::new();

        for file in &self.files {
            let entry = lang_map.entry(file.language.clone()).or_insert_with(|| {
                LanguageStats {
                    language: file.language.clone(),
                    ..Default::default()
                }
            });
            entry.files += 1;
            entry.total_lines += file.stats.total;
            entry.code += file.stats.code;
            entry.comment += file.stats.comment;
            entry.blank += file.stats.blank;
        }

        let mut by_language: Vec<LanguageStats> = lang_map.into_values().collect();
        by_language.sort_by(|a, b| b.code.cmp(&a.code));

        self.by_language = Some(by_language);
        self
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

        // Show language breakdown if available
        if let Some(ref by_language) = stats.by_language {
            writeln!(output, "By Language:").ok();
            writeln!(output).ok();

            for lang in by_language {
                writeln!(
                    output,
                    "{} ({} files):",
                    lang.language, lang.files
                )
                .ok();
                writeln!(output, "  Total lines: {}", lang.total_lines).ok();
                writeln!(output, "  Code: {}", lang.code).ok();
                writeln!(output, "  Comments: {}", lang.comment).ok();
                writeln!(output, "  Blank: {}", lang.blank).ok();
                writeln!(output).ok();
            }
        } else {
            // Original behavior: show per-file stats
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
    #[serde(skip_serializing_if = "Option::is_none")]
    by_language: Option<Vec<LanguageStats>>,
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
    language: String,
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
            by_language: stats.by_language.clone(),
            files: stats
                .files
                .iter()
                .map(|f| JsonFileStats {
                    path: f.path.display().to_string(),
                    language: f.language.clone(),
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
