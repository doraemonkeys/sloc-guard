use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::counter::LineStats;
use crate::stats::TrendDelta;

use super::super::path::display_path;

/// Truncate a path string to the specified depth.
///
/// # Arguments
/// - `path`: Path string with forward-slash separators (from `display_path`)
/// - `depth`: Maximum number of path components to keep (1 = first component only)
///
/// # Examples
/// - `truncate_path_to_depth("src/commands/check", 1)` → `"src"`
/// - `truncate_path_to_depth("src/commands/check", 2)` → `"src/commands"`
/// - `truncate_path_to_depth(".", 1)` → `"."`
fn truncate_path_to_depth(path: &str, depth: usize) -> String {
    if path == "." || depth == 0 {
        return path.to_string();
    }

    // split('/').take(depth) always produces at least one element when depth > 0
    // because split yields at least one item (possibly empty string for leading /)
    path.split('/').take(depth).collect::<Vec<_>>().join("/")
}

/// Sort order for file statistics output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FileSortOrder {
    /// Sort by code lines descending (default)
    #[default]
    Code,
    /// Sort by total lines descending
    Total,
    /// Sort by comment lines descending
    Comment,
    /// Sort by blank lines descending
    Blank,
    /// Sort alphabetically by file name ascending
    Name,
}

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
        self.with_directory_breakdown_depth(None, None)
    }

    /// Compute directory breakdown with paths relative to project root.
    #[must_use]
    pub fn with_directory_breakdown_relative(self, project_root: Option<&Path>) -> Self {
        self.with_directory_breakdown_depth(project_root, None)
    }

    /// Compute directory breakdown with optional depth limiting.
    ///
    /// - `project_root`: Optional project root for relative path display
    /// - `max_depth`: Maximum directory depth to show (1 = top-level only, 2 = two levels, etc.)
    ///   If None, shows the immediate parent directory of each file.
    #[must_use]
    pub fn with_directory_breakdown_depth(
        mut self,
        project_root: Option<&Path>,
        max_depth: Option<usize>,
    ) -> Self {
        let mut dir_map: HashMap<String, DirectoryStats> = HashMap::new();

        for file in &self.files {
            // Get full relative path of directory
            let dir_path = file
                .path
                .parent()
                .map_or_else(|| ".".to_string(), |p| display_path(p, project_root));

            // Apply depth limiting if specified
            let dir_name = match max_depth {
                Some(depth) if depth > 0 => truncate_path_to_depth(&dir_path, depth),
                _ => dir_path,
            };

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

    /// Sort files using the specified sort order and optionally limit to top N.
    ///
    /// This method is used by `stats files` subcommand for custom sorting.
    /// Unlike `with_top_files`, this puts the sorted files in `top_files` field
    /// and marks the output as files-only mode (no summary appended).
    #[must_use]
    pub fn with_sorted_files(mut self, sort: FileSortOrder, limit: Option<usize>) -> Self {
        let mut sorted_files = self.files.clone();

        match sort {
            FileSortOrder::Code => {
                sorted_files.sort_by(|a, b| b.stats.code.cmp(&a.stats.code));
            }
            FileSortOrder::Total => {
                sorted_files.sort_by(|a, b| b.stats.total.cmp(&a.stats.total));
            }
            FileSortOrder::Comment => {
                sorted_files.sort_by(|a, b| b.stats.comment.cmp(&a.stats.comment));
            }
            FileSortOrder::Blank => {
                sorted_files.sort_by(|a, b| b.stats.blank.cmp(&a.stats.blank));
            }
            FileSortOrder::Name => {
                sorted_files.sort_by(|a, b| {
                    // Sort by file name, not full path, for more intuitive ordering
                    let name_a = a.path.file_name().map(|n| n.to_string_lossy());
                    let name_b = b.path.file_name().map(|n| n.to_string_lossy());
                    match (name_a, name_b) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => Ordering::Less,
                        (None, Some(_)) => Ordering::Greater,
                        (None, None) => a.path.cmp(&b.path),
                    }
                });
            }
        }

        // Apply limit if specified
        let sorted_files = if let Some(n) = limit {
            sorted_files.into_iter().take(n).collect()
        } else {
            sorted_files
        };

        self.top_files = Some(sorted_files);
        // Clear files to indicate files-only mode (formatters will use top_files)
        self.files = Vec::new();
        self
    }

    /// Set trend delta from previous run.
    #[must_use]
    pub fn with_trend(mut self, trend: TrendDelta) -> Self {
        self.trend = Some(trend);
        self
    }

    /// Return summary-only statistics (no file list, no breakdown).
    /// Computes average code lines per file and clears detailed data.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Precision loss is acceptable for average calculation
    pub fn with_summary_only(mut self) -> Self {
        // Compute average if not already set
        if self.average_code_lines.is_none() && self.total_files > 0 {
            self.average_code_lines = Some(self.total_code as f64 / self.total_files as f64);
        }

        // Clear detailed data - formatters will skip these sections
        self.files = Vec::new();
        self.top_files = None;
        self.by_language = None;
        self.by_directory = None;

        self
    }
}

#[cfg(test)]
#[path = "statistics_tests.rs"]
mod tests;
