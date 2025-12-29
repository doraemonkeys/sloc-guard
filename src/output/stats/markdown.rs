use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use crate::error::Result;

use super::super::path::display_path;
use super::super::trend_formatting::{format_delta, format_trend_header_markdown};
use super::{ProjectStatistics, StatsFormatter};

pub struct StatsMarkdownFormatter {
    project_root: Option<PathBuf>,
}

impl StatsMarkdownFormatter {
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

impl Default for StatsMarkdownFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsFormatter for StatsMarkdownFormatter {
    fn format(&self, stats: &ProjectStatistics) -> Result<String> {
        let mut output = String::new();

        // Detect files-only mode: files cleared and top_files populated (from with_sorted_files)
        let is_files_only = stats.files.is_empty() && stats.top_files.is_some();

        // In files-only mode, show only the file list without summary
        if is_files_only {
            if let Some(ref top_files) = stats.top_files {
                writeln!(output, "## Files ({} total)\n", top_files.len()).ok();
                writeln!(
                    output,
                    "| File | Language | Code | Total | Comment | Blank |"
                )
                .ok();
                writeln!(
                    output,
                    "|------|----------|-----:|------:|--------:|------:|"
                )
                .ok();
                for file in top_files {
                    writeln!(
                        output,
                        "| `{}` | {} | {} | {} | {} | {} |",
                        self.display_path(&file.path),
                        file.language,
                        file.stats.code,
                        file.stats.total,
                        file.stats.comment,
                        file.stats.blank
                    )
                    .ok();
                }
            }
            return Ok(output);
        }

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
            let header = format_trend_header_markdown(trend);
            writeln!(output, "### {header}\n").ok();
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
                    self.display_path(&file.path),
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
#[path = "markdown_tests.rs"]
mod tests;
