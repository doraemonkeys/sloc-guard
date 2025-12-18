use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;

use crate::counter::LineStats;
use crate::error::Result;
use crate::stats::TrendDelta;

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

#[derive(Debug, Clone, Default, Serialize)]
pub struct DirectoryStats {
    pub directory: String,
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
    pub by_directory: Option<Vec<DirectoryStats>>,
    pub top_files: Option<Vec<FileStatistics>>,
    pub average_code_lines: Option<f64>,
    pub trend: Option<TrendDelta>,
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
            by_directory: None,
            top_files: None,
            average_code_lines: None,
            trend: None,
        }
    }

    #[must_use]
    pub fn with_language_breakdown(mut self) -> Self {
        let mut lang_map: HashMap<String, LanguageStats> = HashMap::new();

        for file in &self.files {
            let entry = lang_map
                .entry(file.language.clone())
                .or_insert_with(|| LanguageStats {
                    language: file.language.clone(),
                    ..Default::default()
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

    #[must_use]
    pub fn with_directory_breakdown(mut self) -> Self {
        let mut dir_map: HashMap<String, DirectoryStats> = HashMap::new();

        for file in &self.files {
            let dir_name = file
                .path
                .parent()
                .map_or_else(|| ".".to_string(), |p| p.display().to_string());
            let entry = dir_map
                .entry(dir_name.clone())
                .or_insert_with(|| DirectoryStats {
                    directory: dir_name,
                    ..Default::default()
                });
            entry.files += 1;
            entry.total_lines += file.stats.total;
            entry.code += file.stats.code;
            entry.comment += file.stats.comment;
            entry.blank += file.stats.blank;
        }

        let mut by_directory: Vec<DirectoryStats> = dir_map.into_values().collect();
        by_directory.sort_by(|a, b| b.code.cmp(&a.code));

        self.by_directory = Some(by_directory);
        self
    }

    /// Compute top N largest files by code lines and average code lines per file.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn with_top_files(mut self, n: usize) -> Self {
        let mut sorted_files = self.files.clone();
        sorted_files.sort_by(|a, b| b.stats.code.cmp(&a.stats.code));
        self.top_files = Some(sorted_files.into_iter().take(n).collect());

        if self.total_files > 0 {
            self.average_code_lines = Some(self.total_code as f64 / self.total_files as f64);
        }

        self
    }

    /// Set trend delta from previous run.
    #[must_use]
    pub const fn with_trend(mut self, trend: TrendDelta) -> Self {
        self.trend = Some(trend);
        self
    }
}

/// Format a delta value with +/- sign.
fn format_delta(value: i64) -> String {
    use std::cmp::Ordering;
    match value.cmp(&0) {
        Ordering::Greater => format!("+{value}"),
        Ordering::Less => format!("{value}"),
        Ordering::Equal => "0".to_string(),
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

        // Show top files if available
        if let Some(ref top_files) = stats.top_files {
            writeln!(output, "Top {} Largest Files:", top_files.len()).ok();
            writeln!(output).ok();

            for (i, file) in top_files.iter().enumerate() {
                writeln!(
                    output,
                    "  {}. {} ({} lines)",
                    i + 1,
                    file.path.display(),
                    file.stats.code
                )
                .ok();
            }
            writeln!(output).ok();
        }

        // Show language breakdown if available
        if let Some(ref by_language) = stats.by_language {
            writeln!(output, "By Language:").ok();
            writeln!(output).ok();

            for lang in by_language {
                writeln!(output, "{} ({} files):", lang.language, lang.files).ok();
                writeln!(output, "  Total lines: {}", lang.total_lines).ok();
                writeln!(output, "  Code: {}", lang.code).ok();
                writeln!(output, "  Comments: {}", lang.comment).ok();
                writeln!(output, "  Blank: {}", lang.blank).ok();
                writeln!(output).ok();
            }
        } else if let Some(ref by_directory) = stats.by_directory {
            writeln!(output, "By Directory:").ok();
            writeln!(output).ok();

            for dir in by_directory {
                writeln!(output, "{} ({} files):", dir.directory, dir.files).ok();
                writeln!(output, "  Total lines: {}", dir.total_lines).ok();
                writeln!(output, "  Code: {}", dir.code).ok();
                writeln!(output, "  Comments: {}", dir.comment).ok();
                writeln!(output, "  Blank: {}", dir.blank).ok();
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
        if let Some(avg) = stats.average_code_lines {
            writeln!(output, "  Average code lines: {avg:.1}").ok();
        }

        // Show trend delta if available
        if let Some(ref trend) = stats.trend {
            writeln!(output).ok();
            writeln!(output, "Changes from previous run:").ok();
            writeln!(output, "  Files: {}", format_delta(trend.files_delta)).ok();
            writeln!(output, "  Total lines: {}", format_delta(trend.lines_delta)).ok();
            writeln!(output, "  Code: {}", format_delta(trend.code_delta)).ok();
            writeln!(output, "  Comments: {}", format_delta(trend.comment_delta)).ok();
            writeln!(output, "  Blank: {}", format_delta(trend.blank_delta)).ok();
        }

        Ok(String::from_utf8_lossy(&output).to_string())
    }
}

pub struct StatsJsonFormatter;

#[derive(Serialize)]
struct JsonStatsOutput {
    summary: JsonStatsSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    trend: Option<JsonTrendDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_language: Option<Vec<LanguageStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_directory: Option<Vec<DirectoryStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_files: Option<Vec<JsonFileStats>>,
    files: Vec<JsonFileStats>,
}

#[derive(Serialize)]
struct JsonStatsSummary {
    total_files: usize,
    total_lines: usize,
    code: usize,
    comment: usize,
    blank: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    average_code_lines: Option<f64>,
}

#[derive(Serialize)]
#[allow(clippy::struct_field_names)]
struct JsonTrendDelta {
    files_delta: i64,
    lines_delta: i64,
    code_delta: i64,
    comment_delta: i64,
    blank_delta: i64,
}

impl From<&TrendDelta> for JsonTrendDelta {
    fn from(trend: &TrendDelta) -> Self {
        Self {
            files_delta: trend.files_delta,
            lines_delta: trend.lines_delta,
            code_delta: trend.code_delta,
            comment_delta: trend.comment_delta,
            blank_delta: trend.blank_delta,
        }
    }
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
                average_code_lines: stats.average_code_lines,
            },
            trend: stats.trend.as_ref().map(JsonTrendDelta::from),
            by_language: stats.by_language.clone(),
            by_directory: stats.by_directory.clone(),
            top_files: stats.top_files.as_ref().map(|files| {
                files
                    .iter()
                    .map(|f| JsonFileStats {
                        path: f.path.display().to_string(),
                        language: f.language.clone(),
                        total: f.stats.total,
                        code: f.stats.code,
                        comment: f.stats.comment,
                        blank: f.stats.blank,
                    })
                    .collect()
            }),
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

pub struct StatsMarkdownFormatter;

impl StatsFormatter for StatsMarkdownFormatter {
    fn format(&self, stats: &ProjectStatistics) -> Result<String> {
        let mut output = String::new();

        writeln!(output, "## SLOC Statistics\n").ok();

        // Summary section
        writeln!(output, "### Summary\n").ok();
        writeln!(output, "| Metric | Value |").ok();
        writeln!(output, "|--------|------:|").ok();
        writeln!(output, "| Total Files | {} |", stats.total_files).ok();
        writeln!(output, "| Total Lines | {} |", stats.total_lines).ok();
        writeln!(output, "| Code | {} |", stats.total_code).ok();
        writeln!(output, "| Comments | {} |", stats.total_comment).ok();
        writeln!(output, "| Blank | {} |", stats.total_blank).ok();
        if let Some(avg) = stats.average_code_lines {
            writeln!(output, "| Average Code Lines | {avg:.1} |").ok();
        }
        writeln!(output).ok();

        // Trend section if available
        if let Some(ref trend) = stats.trend {
            writeln!(output, "### Changes from Previous Run\n").ok();
            writeln!(output, "| Metric | Delta |").ok();
            writeln!(output, "|--------|------:|").ok();
            writeln!(output, "| Files | {} |", format_delta(trend.files_delta)).ok();
            writeln!(
                output,
                "| Total Lines | {} |",
                format_delta(trend.lines_delta)
            )
            .ok();
            writeln!(output, "| Code | {} |", format_delta(trend.code_delta)).ok();
            writeln!(
                output,
                "| Comments | {} |",
                format_delta(trend.comment_delta)
            )
            .ok();
            writeln!(output, "| Blank | {} |", format_delta(trend.blank_delta)).ok();
            writeln!(output).ok();
        }

        // Top files if available
        if let Some(ref top_files) = stats.top_files {
            writeln!(output, "### Top {} Largest Files\n", top_files.len()).ok();
            writeln!(output, "| # | File | Language | Code |").ok();
            writeln!(output, "|--:|------|----------|-----:|").ok();
            for (i, file) in top_files.iter().enumerate() {
                writeln!(
                    output,
                    "| {} | `{}` | {} | {} |",
                    i + 1,
                    file.path.display(),
                    file.language,
                    file.stats.code
                )
                .ok();
            }
            writeln!(output).ok();
        }

        // Language breakdown if available
        if let Some(ref by_language) = stats.by_language {
            writeln!(output, "### By Language\n").ok();
            writeln!(output, "| Language | Files | Code | Comments | Blank |").ok();
            writeln!(output, "|----------|------:|-----:|---------:|------:|").ok();
            for lang in by_language {
                writeln!(
                    output,
                    "| {} | {} | {} | {} | {} |",
                    lang.language, lang.files, lang.code, lang.comment, lang.blank
                )
                .ok();
            }
        }

        // Directory breakdown if available
        if let Some(ref by_directory) = stats.by_directory {
            writeln!(output, "### By Directory\n").ok();
            writeln!(output, "| Directory | Files | Code | Comments | Blank |").ok();
            writeln!(output, "|-----------|------:|-----:|---------:|------:|").ok();
            for dir in by_directory {
                writeln!(
                    output,
                    "| `{}` | {} | {} | {} | {} |",
                    dir.directory, dir.files, dir.code, dir.comment, dir.blank
                )
                .ok();
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
