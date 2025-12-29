use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;
use crate::stats::TrendDelta;

use super::super::path::display_path;
use super::{DirectoryStats, LanguageStats, ProjectStatistics, StatsFormatter};

pub struct StatsJsonFormatter {
    project_root: Option<PathBuf>,
}

impl StatsJsonFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self { project_root: None }
    }

    #[must_use]
    pub fn with_project_root(mut self, root: Option<PathBuf>) -> Self {
        self.project_root = root;
        self
    }

    fn display_path(&self, path: &Path) -> String {
        display_path(path, self.project_root.as_deref())
    }
}

impl Default for StatsJsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct JsonStatsOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<JsonStatsSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trend: Option<JsonTrendDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_language: Option<Vec<LanguageStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_directory: Option<Vec<DirectoryStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_files: Option<Vec<JsonFileStats>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
struct JsonTrendDelta {
    files: i64,
    lines: i64,
    code: i64,
    comment: i64,
    blank: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_branch: Option<String>,
}

impl From<&TrendDelta> for JsonTrendDelta {
    fn from(trend: &TrendDelta) -> Self {
        Self {
            files: trend.files_delta,
            lines: trend.lines_delta,
            code: trend.code_delta,
            comment: trend.comment_delta,
            blank: trend.blank_delta,
            previous_commit: trend.previous_git_ref.clone(),
            previous_branch: trend.previous_git_branch.clone(),
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
        // Detect files-only mode: files cleared and top_files populated (from with_sorted_files)
        let is_files_only = stats.files.is_empty() && stats.top_files.is_some();

        // Include summary unless in files-only mode
        let summary = if is_files_only {
            None
        } else {
            Some(JsonStatsSummary {
                total_files: stats.total_files,
                total_lines: stats.total_lines,
                code: stats.total_code,
                comment: stats.total_comment,
                blank: stats.total_blank,
                average_code_lines: stats.average_code_lines,
            })
        };

        let output = JsonStatsOutput {
            summary,
            trend: stats.trend.as_ref().map(JsonTrendDelta::from),
            by_language: stats.by_language.clone(),
            by_directory: stats.by_directory.clone(),
            top_files: stats.top_files.as_ref().map(|files| {
                files
                    .iter()
                    .map(|f| JsonFileStats {
                        path: self.display_path(&f.path),
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
                    path: self.display_path(&f.path),
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
#[path = "json_tests.rs"]
mod tests;
