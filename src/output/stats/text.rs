use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::Result;

use super::super::path::display_path;
use super::super::text::ColorMode;
use super::super::trend_formatting::{TrendLineFormatter, format_trend_header};
use super::{ProjectStatistics, StatsFormatter};

pub struct StatsTextFormatter {
    trend_formatter: TrendLineFormatter,
    project_root: Option<PathBuf>,
}

impl Default for StatsTextFormatter {
    fn default() -> Self {
        Self::new(ColorMode::Auto)
    }
}

impl StatsTextFormatter {
    #[must_use]
    pub fn new(mode: ColorMode) -> Self {
        Self {
            trend_formatter: TrendLineFormatter::new(mode),
            project_root: None,
        }
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
                    self.display_path(&file.path),
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
                    self.display_path(&file.path),
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

            // Format header with git context and/or relative time
            let header = format_trend_header(trend);
            writeln!(output, "{header}").ok();

            // Format each metric with arrows, colors, and percentages
            writeln!(
                output,
                "{}",
                self.trend_formatter
                    .format_line("Files", trend.files_delta, stats.total_files)
            )
            .ok();
            writeln!(
                output,
                "{}",
                self.trend_formatter.format_line(
                    "Total lines",
                    trend.lines_delta,
                    stats.total_lines
                )
            )
            .ok();
            writeln!(
                output,
                "{}",
                self.trend_formatter
                    .format_line("Code", trend.code_delta, stats.total_code)
            )
            .ok();
            writeln!(
                output,
                "{}",
                self.trend_formatter.format_line(
                    "Comments",
                    trend.comment_delta,
                    stats.total_comment
                )
            )
            .ok();
            writeln!(
                output,
                "{}",
                self.trend_formatter
                    .format_line("Blank", trend.blank_delta, stats.total_blank)
            )
            .ok();
        }

        Ok(String::from_utf8_lossy(&output).to_string())
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
