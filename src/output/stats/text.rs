use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::Result;

use super::super::path::display_path;
use super::super::text::ColorMode;
use super::super::trend_formatting::{TrendLineFormatter, format_trend_header};
use super::{ProjectStatistics, StatsFormatter};

/// Characters for rendering progress bars (using Unicode block elements).
const PROGRESS_FILLED: char = '█';
const PROGRESS_EMPTY: char = '░';
const PROGRESS_WIDTH: usize = 25;

/// Render a visual progress bar showing the percentage filled.
///
/// # Arguments
/// - `filled_ratio`: Value between 0.0 and 1.0 representing the fill percentage.
///   Values outside this range are clamped.
/// - `width`: Total width of the bar in characters
///
/// # Safety justification for `#[allow]` attributes
/// - `cast_precision_loss`: width is a small constant (25), so f64 can represent it exactly
/// - `cast_possible_truncation`: result of `width * ratio` is at most `width` after clamping
/// - `cast_sign_loss`: ratio is clamped to [0,1], so product is always non-negative
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn render_progress_bar(filled_ratio: f64, width: usize) -> String {
    let filled_ratio = filled_ratio.clamp(0.0, 1.0);
    let filled_count = (filled_ratio * width as f64).round() as usize;
    let empty_count = width.saturating_sub(filled_count);

    format!(
        "{}{}",
        PROGRESS_FILLED.to_string().repeat(filled_count),
        PROGRESS_EMPTY.to_string().repeat(empty_count)
    )
}

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

/// Format language breakdown with progress bars.
#[allow(clippy::cast_precision_loss)] // Precision loss acceptable: percentages for display
fn format_language_breakdown(
    output: &mut Vec<u8>,
    by_language: &[super::LanguageStats],
    total_code: usize,
) {
    writeln!(output, "By Language:").ok();
    writeln!(output).ok();

    for lang in by_language {
        let ratio = if total_code > 0 {
            lang.code as f64 / total_code as f64
        } else {
            0.0
        };
        let progress = render_progress_bar(ratio, PROGRESS_WIDTH);
        let percent = ratio * 100.0;

        writeln!(output, "{} ({} files):", lang.language, lang.files).ok();
        writeln!(output, "  {progress} {percent:5.1}%  ({} code)", lang.code).ok();
        writeln!(
            output,
            "  Total: {}  Comments: {}  Blank: {}",
            lang.total_lines, lang.comment, lang.blank
        )
        .ok();
        writeln!(output).ok();
    }
}

/// Format directory breakdown with progress bars.
#[allow(clippy::cast_precision_loss)] // Precision loss acceptable: percentages for display
fn format_directory_breakdown(
    output: &mut Vec<u8>,
    by_directory: &[super::DirectoryStats],
    total_code: usize,
) {
    writeln!(output, "By Directory:").ok();
    writeln!(output).ok();

    for dir in by_directory {
        let ratio = if total_code > 0 {
            dir.code as f64 / total_code as f64
        } else {
            0.0
        };
        let progress = render_progress_bar(ratio, PROGRESS_WIDTH);
        let percent = ratio * 100.0;

        writeln!(output, "{} ({} files):", dir.directory, dir.files).ok();
        writeln!(output, "  {progress} {percent:5.1}%  ({} code)", dir.code).ok();
        writeln!(
            output,
            "  Total: {}  Comments: {}  Blank: {}",
            dir.total_lines, dir.comment, dir.blank
        )
        .ok();
        writeln!(output).ok();
    }
}

impl StatsFormatter for StatsTextFormatter {
    fn format(&self, stats: &ProjectStatistics) -> Result<String> {
        let mut output = Vec::new();

        // Detect files-only mode: files cleared and top_files populated (from with_sorted_files)
        let is_files_only = stats.files.is_empty() && stats.top_files.is_some();

        // Show top files if available
        if let Some(ref top_files) = stats.top_files {
            // In files-only mode, show as file list without "Top N" header
            if is_files_only {
                writeln!(output, "Files ({} total):", top_files.len()).ok();
                writeln!(output).ok();

                for file in top_files {
                    writeln!(
                        output,
                        "  {} - {} code, {} total (comment={}, blank={})",
                        self.display_path(&file.path),
                        file.stats.code,
                        file.stats.total,
                        file.stats.comment,
                        file.stats.blank
                    )
                    .ok();
                }
            } else {
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
        }

        // Skip remaining output sections in files-only mode
        if is_files_only {
            return Ok(String::from_utf8_lossy(&output).to_string());
        }

        // Show language breakdown if available
        if let Some(ref by_language) = stats.by_language {
            format_language_breakdown(&mut output, by_language, stats.total_code);
        } else if let Some(ref by_directory) = stats.by_directory {
            format_directory_breakdown(&mut output, by_directory, stats.total_code);
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
