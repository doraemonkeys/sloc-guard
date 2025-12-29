use std::path::Path;

use crate::cli::{ReportOutputFormat, StatsOutputFormat};
use crate::output::{
    ColorMode, ProjectStatistics, StatsFormatter, StatsHtmlFormatter, StatsJsonFormatter,
    StatsMarkdownFormatter, StatsTextFormatter,
};
use crate::stats::TrendHistory;

/// Unified stats output formatter for Text/Json/Markdown formats.
///
/// Used by summary, files, breakdown, and trend subcommands.
pub fn format_stats_subcommand_output(
    format: StatsOutputFormat,
    stats: &ProjectStatistics,
    color_mode: ColorMode,
    project_root: &Path,
) -> crate::Result<String> {
    match format {
        StatsOutputFormat::Text => StatsTextFormatter::new(color_mode)
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        StatsOutputFormat::Json => StatsJsonFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        StatsOutputFormat::Markdown => StatsMarkdownFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
    }
}

/// Format output for comprehensive reports (includes HTML support with trend history).
pub fn format_report_output(
    format: ReportOutputFormat,
    stats: &ProjectStatistics,
    color_mode: ColorMode,
    project_root: &Path,
    trend_history: &TrendHistory,
) -> crate::Result<String> {
    match format {
        ReportOutputFormat::Text => StatsTextFormatter::new(color_mode)
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        ReportOutputFormat::Json => StatsJsonFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        ReportOutputFormat::Markdown => StatsMarkdownFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        ReportOutputFormat::Html => StatsHtmlFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .with_trend_history(trend_history.clone())
            .format(stats),
    }
}

/// Format stats output for all supported output formats.
///
/// Kept for backward compatibility with existing tests.
#[cfg(test)]
pub fn format_stats_output(
    format: crate::output::OutputFormat,
    stats: &ProjectStatistics,
    color_mode: ColorMode,
    project_root: Option<&Path>,
    trend_history: Option<&TrendHistory>,
) -> crate::Result<String> {
    use crate::output::OutputFormat;
    match format {
        OutputFormat::Text => StatsTextFormatter::new(color_mode)
            .with_project_root(project_root.map(Path::to_path_buf))
            .format(stats),
        OutputFormat::Json => StatsJsonFormatter::new()
            .with_project_root(project_root.map(Path::to_path_buf))
            .format(stats),
        OutputFormat::Sarif => Err(crate::SlocGuardError::Config(
            "SARIF output format is not supported for stats command".to_string(),
        )),
        OutputFormat::Markdown => StatsMarkdownFormatter::new()
            .with_project_root(project_root.map(Path::to_path_buf))
            .format(stats),
        OutputFormat::Html => {
            let mut formatter =
                StatsHtmlFormatter::new().with_project_root(project_root.map(Path::to_path_buf));
            if let Some(history) = trend_history {
                formatter = formatter.with_trend_history(history.clone());
            }
            formatter.format(stats)
        }
    }
}
