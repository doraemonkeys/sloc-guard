use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::counter::LineStats;
use crate::stats::TrendDelta;

use super::super::path::display_path;

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
    pub fn with_directory_breakdown(self) -> Self {
        self.with_directory_breakdown_relative(None)
    }

    /// Compute directory breakdown with paths relative to project root.
    #[must_use]
    pub fn with_directory_breakdown_relative(mut self, project_root: Option<&Path>) -> Self {
        let mut dir_map: HashMap<String, DirectoryStats> = HashMap::new();

        for file in &self.files {
            // display_path returns "." for empty relative paths
            let dir_name = file
                .path
                .parent()
                .map_or_else(|| ".".to_string(), |p| display_path(p, project_root));
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
    #[allow(clippy::cast_precision_loss)] // Precision loss is acceptable for average calculation
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
    pub fn with_trend(mut self, trend: TrendDelta) -> Self {
        self.trend = Some(trend);
        self
    }
}

#[cfg(test)]
#[path = "statistics_tests.rs"]
mod tests;
