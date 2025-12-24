use std::fmt::Write;

use crate::checker::CheckResult;
use crate::error::Result;

use super::html_template::{HTML_FOOTER, HTML_HEADER};
use super::svg::{FileSizeHistogram, LanguageBreakdownChart, SvgElement, TrendLineChart};
use super::{OutputFormatter, ProjectStatistics};
use crate::stats::TrendHistory;

/// HTML formatter for generating standalone HTML reports.
pub struct HtmlFormatter {
    show_suggestions: bool,
    project_stats: Option<ProjectStatistics>,
    trend_history: Option<TrendHistory>,
}

impl HtmlFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_suggestions: false,
            project_stats: None,
            trend_history: None,
        }
    }

    #[must_use]
    pub const fn with_suggestions(mut self, show: bool) -> Self {
        self.show_suggestions = show;
        self
    }

    /// Attach project statistics for chart generation.
    #[must_use]
    pub fn with_stats(mut self, stats: ProjectStatistics) -> Self {
        self.project_stats = Some(stats);
        self
    }

    /// Attach trend history for trend line chart generation.
    #[must_use]
    pub fn with_trend_history(mut self, history: TrendHistory) -> Self {
        self.trend_history = Some(history);
        self
    }

    const fn status_class(result: &CheckResult) -> &'static str {
        match result {
            CheckResult::Passed { .. } => "passed",
            CheckResult::Warning { .. } => "warning",
            CheckResult::Failed { .. } => "failed",
            CheckResult::Grandfathered { .. } => "grandfathered",
        }
    }

    const fn status_icon(result: &CheckResult) -> &'static str {
        match result {
            CheckResult::Passed { .. } => "&#x2713;",        // ✓
            CheckResult::Warning { .. } => "&#x26A0;",       // ⚠
            CheckResult::Failed { .. } => "&#x2717;",        // ✗
            CheckResult::Grandfathered { .. } => "&#x25C9;", // ◉
        }
    }

    const fn status_text(result: &CheckResult) -> &'static str {
        match result {
            CheckResult::Passed { .. } => "Passed",
            CheckResult::Warning { .. } => "Warning",
            CheckResult::Failed { .. } => "Failed",
            CheckResult::Grandfathered { .. } => "Grandfathered",
        }
    }

    fn write_html_header(output: &mut String) {
        output.push_str(HTML_HEADER);
    }

    fn write_html_footer(output: &mut String) {
        output.push_str(HTML_FOOTER);
    }

    fn write_charts_section(
        output: &mut String,
        stats: Option<&ProjectStatistics>,
        trend_history: Option<&TrendHistory>,
    ) {
        let histogram = stats.map(FileSizeHistogram::from_stats);
        let language_chart = stats.map(LanguageBreakdownChart::from_stats);
        let trend_chart = trend_history.map(TrendLineChart::from_history);

        // Check if any chart has data
        let has_histogram = histogram
            .as_ref()
            .is_some_and(FileSizeHistogram::has_sufficient_data);
        let has_language = language_chart
            .as_ref()
            .is_some_and(LanguageBreakdownChart::has_data);
        let has_trend = trend_chart.as_ref().is_some_and(TrendLineChart::has_data);

        // Only show charts section if there's data for at least one chart
        if !has_histogram && !has_language && !has_trend {
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

        // File Size Distribution Histogram
        if let Some(histogram) = &histogram
            && histogram.has_sufficient_data()
        {
            output.push_str("            <div class=\"chart-container\">\n");
            output.push_str("                <h3>File Size Distribution (by SLOC)</h3>\n");
            let svg = histogram.render();
            for line in svg.lines() {
                let _ = writeln!(output, "                {line}");
            }
            output.push_str("            </div>\n");
        }

        // Language Breakdown Chart
        if let Some(language_chart) = &language_chart
            && language_chart.has_data()
        {
            output.push_str("            <div class=\"chart-container\">\n");
            output.push_str("                <h3>Language Breakdown</h3>\n");
            let svg = language_chart.render();
            for line in svg.lines() {
                let _ = writeln!(output, "                {line}");
            }
            output.push_str("            </div>\n");
        }

        output.push_str("        </div>\n");
    }

    fn write_summary(
        output: &mut String,
        total: usize,
        passed: usize,
        warnings: usize,
        failed: usize,
        grandfathered: usize,
    ) {
        output.push_str("        <div class=\"summary-grid\">\n");

        // Total files card
        writeln!(
            output,
            r#"            <div class="summary-card">
                <span class="value">{total}</span>
                <span class="label">Total Files</span>
            </div>"#
        )
        .ok();

        // Passed card
        writeln!(
            output,
            r#"            <div class="summary-card passed">
                <span class="value">{passed}</span>
                <span class="label">Passed</span>
            </div>"#
        )
        .ok();

        // Warnings card
        writeln!(
            output,
            r#"            <div class="summary-card warning">
                <span class="value">{warnings}</span>
                <span class="label">Warnings</span>
            </div>"#
        )
        .ok();

        // Failed card
        writeln!(
            output,
            r#"            <div class="summary-card failed">
                <span class="value">{failed}</span>
                <span class="label">Failed</span>
            </div>"#
        )
        .ok();

        // Grandfathered card (only if there are grandfathered files)
        if grandfathered > 0 {
            writeln!(
                output,
                r#"            <div class="summary-card grandfathered">
                <span class="value">{grandfathered}</span>
                <span class="label">Grandfathered</span>
            </div>"#
            )
            .ok();
        }

        output.push_str("        </div>\n");
    }

    fn write_file_table(&self, output: &mut String, results: &[CheckResult]) {
        if results.is_empty() {
            output.push_str("        <p class=\"no-results\">No files to display.</p>\n");
            return;
        }

        output.push_str("        <h2>All Files</h2>\n");

        // Filter controls
        output.push_str("        <div class=\"filter-controls\">\n");
        output.push_str(
            "            <button class=\"filter-btn active\" data-filter=\"all\">All</button>\n",
        );
        output.push_str(
            "            <button class=\"filter-btn\" data-filter=\"issues\">Issues Only</button>\n",
        );
        output.push_str(
            "            <button class=\"filter-btn\" data-filter=\"failed\">Failed</button>\n",
        );
        output.push_str(
            "            <button class=\"filter-btn\" data-filter=\"warning\">Warning</button>\n",
        );
        output.push_str(
            "            <button class=\"filter-btn\" data-filter=\"passed\">Passed</button>\n",
        );
        output.push_str("        </div>\n");

        output.push_str("        <div class=\"table-container\">\n");
        output.push_str("        <table id=\"file-table\">\n");
        output.push_str("            <thead>\n");
        output.push_str("                <tr>\n");
        output.push_str(
            "                    <th class=\"sortable\" data-sort=\"status\">Status</th>\n",
        );
        output
            .push_str("                    <th class=\"sortable\" data-sort=\"text\">File</th>\n");
        output.push_str(
            "                    <th class=\"sortable\" data-sort=\"number\">Lines</th>\n",
        );
        output.push_str(
            "                    <th class=\"sortable\" data-sort=\"number\">Limit</th>\n",
        );
        output.push_str(
            "                    <th class=\"sortable\" data-sort=\"number\">Code</th>\n",
        );
        output.push_str(
            "                    <th class=\"sortable\" data-sort=\"number\">Comment</th>\n",
        );
        output.push_str(
            "                    <th class=\"sortable\" data-sort=\"number\">Blank</th>\n",
        );
        output.push_str("                </tr>\n");
        output.push_str("            </thead>\n");
        output.push_str("            <tbody>\n");

        for result in results {
            self.write_file_row(output, result);
        }

        output.push_str("            </tbody>\n");
        output.push_str("        </table>\n");
        output.push_str("        </div>\n");
    }

    fn write_file_row(&self, output: &mut String, result: &CheckResult) {
        let class = Self::status_class(result);
        let icon = Self::status_icon(result);
        let text = Self::status_text(result);
        let path = html_escape(&result.path().display().to_string());

        // Add data-status for filtering
        writeln!(output, "                <tr data-status=\"{class}\">").ok();

        // Status cell
        writeln!(
            output,
            r#"                    <td><span class="status {class}">{icon} {text}</span></td>"#
        )
        .ok();

        // File path cell
        output.push_str("                    <td>\n");
        writeln!(
            output,
            r#"                        <div class="file-path">{path}</div>"#
        )
        .ok();

        // Optional reason
        if let Some(reason) = result.override_reason() {
            let escaped_reason = html_escape(reason);
            writeln!(
                output,
                r#"                        <div class="reason">{escaped_reason}</div>"#
            )
            .ok();
        }

        // Optional split suggestions
        if self.show_suggestions
            && let Some(suggestion) = result.suggestions()
            && suggestion.has_suggestions()
        {
            output.push_str("                        <div class=\"suggestions\">\n");
            output.push_str("                            <h4>Split suggestions:</h4>\n");
            output.push_str("                            <ul>\n");
            for chunk in &suggestion.chunks {
                let funcs = if chunk.functions.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", chunk.functions.join(", "))
                };
                writeln!(
                    output,
                    r"                                <li>{}.* (~{} lines){}</li>",
                    html_escape(&chunk.suggested_name),
                    chunk.line_count,
                    html_escape(&funcs)
                )
                .ok();
            }
            output.push_str("                            </ul>\n");
            output.push_str("                        </div>\n");
        }

        output.push_str("                    </td>\n");

        // Numeric cells with data-value for sorting
        let sloc = result.stats().sloc();
        writeln!(
            output,
            r#"                    <td class="number" data-value="{sloc}">{sloc}</td>"#
        )
        .ok();
        let limit = result.limit();
        writeln!(
            output,
            r#"                    <td class="number" data-value="{limit}">{limit}</td>"#
        )
        .ok();
        let code = result.stats().code;
        writeln!(
            output,
            r#"                    <td class="number" data-value="{code}">{code}</td>"#
        )
        .ok();
        let comment = result.stats().comment;
        writeln!(
            output,
            r#"                    <td class="number" data-value="{comment}">{comment}</td>"#
        )
        .ok();
        let blank = result.stats().blank;
        writeln!(
            output,
            r#"                    <td class="number" data-value="{blank}">{blank}</td>"#
        )
        .ok();

        output.push_str("                </tr>\n");
    }
}

impl Default for HtmlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for HtmlFormatter {
    fn format(&self, results: &[CheckResult]) -> Result<String> {
        let mut output = String::new();

        // Count by status
        let (passed, warnings, failed, grandfathered) =
            results
                .iter()
                .fold((0, 0, 0, 0), |(p, w, f, g), r| match r {
                    CheckResult::Passed { .. } => (p + 1, w, f, g),
                    CheckResult::Warning { .. } => (p, w + 1, f, g),
                    CheckResult::Failed { .. } => (p, w, f + 1, g),
                    CheckResult::Grandfathered { .. } => (p, w, f, g + 1),
                });

        Self::write_html_header(&mut output);
        Self::write_summary(
            &mut output,
            results.len(),
            passed,
            warnings,
            failed,
            grandfathered,
        );

        // Render charts if project stats or trend history are available
        Self::write_charts_section(
            &mut output,
            self.project_stats.as_ref(),
            self.trend_history.as_ref(),
        );

        self.write_file_table(&mut output, results);
        Self::write_html_footer(&mut output);

        Ok(output)
    }
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
