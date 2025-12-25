//! HTML formatter for stats command output.
//!
//! Generates a standalone HTML document with styled tables and SVG charts.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::stats::TrendHistory;

use super::super::html::{HTML_FOOTER, HTML_HEADER, html_escape};
use super::super::path::display_path;
use super::super::svg::{LanguageBreakdownChart, SvgElement, TrendLineChart};
use super::super::trend_formatting::{format_delta, format_trend_header_markdown};
use super::{ProjectStatistics, StatsFormatter};

/// HTML formatter for project statistics.
///
/// Generates a styled HTML report with:
/// - Summary cards showing total files, lines, code, comments, blanks
/// - Trend delta section if trend data is available
/// - Language breakdown table (if `--group-by lang`)
/// - Directory breakdown table (if `--group-by dir`)
/// - Top files table (if `--top N`)
/// - Inline SVG charts: language breakdown, trend line
pub struct StatsHtmlFormatter {
    project_root: Option<PathBuf>,
    trend_history: Option<TrendHistory>,
}

impl StatsHtmlFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            project_root: None,
            trend_history: None,
        }
    }

    /// Set project root for relative path display.
    #[must_use]
    pub fn with_project_root(mut self, root: Option<PathBuf>) -> Self {
        self.project_root = root;
        self
    }

    /// Attach trend history for trend line chart generation.
    #[must_use]
    pub fn with_trend_history(mut self, history: TrendHistory) -> Self {
        self.trend_history = Some(history);
        self
    }

    fn display_path(&self, path: &Path) -> String {
        display_path(path, self.project_root.as_deref())
    }

    fn write_summary_cards(output: &mut String, stats: &ProjectStatistics) {
        output.push_str("        <div class=\"summary-grid\">\n");

        // Total Files
        let _ = writeln!(
            output,
            r#"            <div class="summary-card">
                <span class="value">{}</span>
                <span class="label">Total Files</span>
            </div>"#,
            stats.total_files
        );

        // Total Lines
        let _ = writeln!(
            output,
            r#"            <div class="summary-card">
                <span class="value">{}</span>
                <span class="label">Total Lines</span>
            </div>"#,
            stats.total_lines
        );

        // Code
        let _ = writeln!(
            output,
            r#"            <div class="summary-card">
                <span class="value">{}</span>
                <span class="label">Code</span>
            </div>"#,
            stats.total_code
        );

        // Comments
        let _ = writeln!(
            output,
            r#"            <div class="summary-card">
                <span class="value">{}</span>
                <span class="label">Comments</span>
            </div>"#,
            stats.total_comment
        );

        // Blanks
        let _ = writeln!(
            output,
            r#"            <div class="summary-card">
                <span class="value">{}</span>
                <span class="label">Blanks</span>
            </div>"#,
            stats.total_blank
        );

        // Average Code Lines (if available)
        if let Some(avg) = stats.average_code_lines {
            let _ = writeln!(
                output,
                r#"            <div class="summary-card">
                <span class="value">{avg:.1}</span>
                <span class="label">Avg Code/File</span>
            </div>"#
            );
        }

        output.push_str("        </div>\n");
    }

    fn write_trend_section(output: &mut String, stats: &ProjectStatistics) {
        let Some(ref trend) = stats.trend else {
            return;
        };

        let header = format_trend_header_markdown(trend);
        let _ = writeln!(output, "        <h2>{header}</h2>");
        output.push_str("        <div class=\"summary-grid\">\n");

        // Files delta
        Self::write_delta_card(output, "Files", trend.files_delta);

        // Lines delta
        Self::write_delta_card(output, "Total Lines", trend.lines_delta);

        // Code delta
        Self::write_delta_card(output, "Code", trend.code_delta);

        // Comments delta
        Self::write_delta_card(output, "Comments", trend.comment_delta);

        // Blanks delta
        Self::write_delta_card(output, "Blanks", trend.blank_delta);

        output.push_str("        </div>\n");
    }

    fn write_delta_card(output: &mut String, label: &str, delta: i64) {
        use std::cmp::Ordering;

        let delta_str = format_delta(delta);
        // Use semantically neutral class names for trend deltas
        let class = match delta.cmp(&0) {
            Ordering::Greater => "delta-increase",
            Ordering::Less => "delta-decrease",
            Ordering::Equal => "",
        };

        let _ = writeln!(
            output,
            r#"            <div class="summary-card {class}">
                <span class="value">{delta_str}</span>
                <span class="label">{label}</span>
            </div>"#
        );
    }

    fn write_language_breakdown(output: &mut String, stats: &ProjectStatistics) {
        let Some(ref by_language) = stats.by_language else {
            return;
        };

        if by_language.is_empty() {
            return;
        }

        output.push_str("        <h2>Language Breakdown</h2>\n");
        output.push_str("        <div class=\"table-container\">\n");
        output.push_str("        <table>\n");
        output.push_str("            <thead>\n");
        output.push_str("                <tr>\n");
        output.push_str("                    <th>Language</th>\n");
        output.push_str("                    <th class=\"number\">Files</th>\n");
        output.push_str("                    <th class=\"number\">Code</th>\n");
        output.push_str("                    <th class=\"number\">Comments</th>\n");
        output.push_str("                    <th class=\"number\">Blanks</th>\n");
        output.push_str("                    <th class=\"number\">Total</th>\n");
        output.push_str("                </tr>\n");
        output.push_str("            </thead>\n");
        output.push_str("            <tbody>\n");

        for lang in by_language {
            let _ = writeln!(
                output,
                r#"                <tr>
                    <td>{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                </tr>"#,
                html_escape(&lang.language),
                lang.files,
                lang.code,
                lang.comment,
                lang.blank,
                lang.total_lines
            );
        }

        output.push_str("            </tbody>\n");
        output.push_str("        </table>\n");
        output.push_str("        </div>\n");
    }

    fn write_directory_breakdown(output: &mut String, stats: &ProjectStatistics) {
        let Some(ref by_directory) = stats.by_directory else {
            return;
        };

        if by_directory.is_empty() {
            return;
        }

        output.push_str("        <h2>Directory Breakdown</h2>\n");
        output.push_str("        <div class=\"table-container\">\n");
        output.push_str("        <table>\n");
        output.push_str("            <thead>\n");
        output.push_str("                <tr>\n");
        output.push_str("                    <th>Directory</th>\n");
        output.push_str("                    <th class=\"number\">Files</th>\n");
        output.push_str("                    <th class=\"number\">Code</th>\n");
        output.push_str("                    <th class=\"number\">Comments</th>\n");
        output.push_str("                    <th class=\"number\">Blanks</th>\n");
        output.push_str("                    <th class=\"number\">Total</th>\n");
        output.push_str("                </tr>\n");
        output.push_str("            </thead>\n");
        output.push_str("            <tbody>\n");

        for dir in by_directory {
            let _ = writeln!(
                output,
                r#"                <tr>
                    <td class="file-path">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                </tr>"#,
                html_escape(&dir.directory),
                dir.files,
                dir.code,
                dir.comment,
                dir.blank,
                dir.total_lines
            );
        }

        output.push_str("            </tbody>\n");
        output.push_str("        </table>\n");
        output.push_str("        </div>\n");
    }

    fn write_top_files(&self, output: &mut String, stats: &ProjectStatistics) {
        let Some(ref top_files) = stats.top_files else {
            return;
        };

        if top_files.is_empty() {
            return;
        }

        let _ = writeln!(
            output,
            "        <h2>Top {} Largest Files</h2>",
            top_files.len()
        );
        output.push_str("        <div class=\"table-container\">\n");
        output.push_str("        <table>\n");
        output.push_str("            <thead>\n");
        output.push_str("                <tr>\n");
        output.push_str("                    <th class=\"number\">#</th>\n");
        output.push_str("                    <th>File</th>\n");
        output.push_str("                    <th>Language</th>\n");
        output.push_str("                    <th class=\"number\">Code</th>\n");
        output.push_str("                    <th class=\"number\">Comments</th>\n");
        output.push_str("                    <th class=\"number\">Blanks</th>\n");
        output.push_str("                    <th class=\"number\">Total</th>\n");
        output.push_str("                </tr>\n");
        output.push_str("            </thead>\n");
        output.push_str("            <tbody>\n");

        for (i, file) in top_files.iter().enumerate() {
            let _ = writeln!(
                output,
                r#"                <tr>
                    <td class="number">{}</td>
                    <td class="file-path">{}</td>
                    <td>{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                    <td class="number">{}</td>
                </tr>"#,
                i + 1,
                html_escape(&self.display_path(&file.path)),
                html_escape(&file.language),
                file.stats.code,
                file.stats.comment,
                file.stats.blank,
                file.stats.total
            );
        }

        output.push_str("            </tbody>\n");
        output.push_str("        </table>\n");
        output.push_str("        </div>\n");
    }

    fn write_charts_section(&self, output: &mut String, stats: &ProjectStatistics) {
        let language_chart = stats
            .by_language
            .as_ref()
            .map(|_| LanguageBreakdownChart::from_stats(stats));
        let trend_chart = self
            .trend_history
            .as_ref()
            .map(TrendLineChart::from_history);

        // Check if any chart has data
        let has_language = language_chart
            .as_ref()
            .is_some_and(LanguageBreakdownChart::has_data);
        let has_trend = trend_chart.as_ref().is_some_and(TrendLineChart::has_data);

        if !has_language && !has_trend {
            return;
        }

        output.push_str("        <div class=\"charts-section\">\n");
        output.push_str("            <h2>Visualizations</h2>\n");

        // Trend Line Chart (show first as it's the most important)
        if let Some(chart) = &trend_chart
            && chart.has_data()
        {
            output.push_str("            <div class=\"chart-container\">\n");
            output.push_str("                <h3>Code Lines Over Time</h3>\n");
            let svg = chart.render();
            for line in svg.lines() {
                let _ = writeln!(output, "                {line}");
            }
            output.push_str("            </div>\n");
        }

        // Language Breakdown Chart
        if let Some(chart) = &language_chart
            && chart.has_data()
        {
            output.push_str("            <div class=\"chart-container\">\n");
            output.push_str("                <h3>Language Breakdown</h3>\n");
            let svg = chart.render();
            for line in svg.lines() {
                let _ = writeln!(output, "                {line}");
            }
            output.push_str("            </div>\n");
        }

        output.push_str("        </div>\n");
    }
}

impl Default for StatsHtmlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsFormatter for StatsHtmlFormatter {
    fn format(&self, stats: &ProjectStatistics) -> Result<String> {
        let mut output = String::new();

        // HTML header with styles
        output.push_str(HTML_HEADER);

        // Replace the title for stats report
        // Note: HTML_HEADER has "SLOC Guard Report" - we'll leave it as is for consistency

        // Summary cards
        Self::write_summary_cards(&mut output, stats);

        // Trend section if available
        Self::write_trend_section(&mut output, stats);

        // Charts section (language breakdown chart, trend chart)
        self.write_charts_section(&mut output, stats);

        // Language breakdown table
        Self::write_language_breakdown(&mut output, stats);

        // Directory breakdown table
        Self::write_directory_breakdown(&mut output, stats);

        // Top files table
        self.write_top_files(&mut output, stats);

        // HTML footer
        output.push_str(HTML_FOOTER);

        Ok(output)
    }
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
