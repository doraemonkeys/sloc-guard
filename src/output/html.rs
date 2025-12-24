use std::fmt::Write;

use crate::checker::CheckResult;
use crate::error::Result;

use super::svg::{FileSizeHistogram, LanguageBreakdownChart, SvgElement};
use super::{OutputFormatter, ProjectStatistics};

const HTML_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SLOC Guard Report</title>
    <style>
        :root {
            --color-passed: #22c55e;
            --color-warning: #eab308;
            --color-failed: #ef4444;
            --color-grandfathered: #3b82f6;
            --color-bg: #f8fafc;
            --color-card: #ffffff;
            --color-border: #e2e8f0;
            --color-text: #1e293b;
            --color-text-muted: #64748b;
            --color-chart-primary: #6366f1;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: var(--color-bg);
            color: var(--color-text);
            line-height: 1.6;
            padding: 2rem;
        }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { font-size: 1.875rem; font-weight: 700; margin-bottom: 1.5rem; color: var(--color-text); }
        h2 { font-size: 1.25rem; font-weight: 600; margin: 1.5rem 0 1rem; color: var(--color-text); }
        .summary-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
        .summary-card { background: var(--color-card); border-radius: 0.5rem; padding: 1.25rem; border: 1px solid var(--color-border); text-align: center; }
        .summary-card .value { font-size: 2rem; font-weight: 700; display: block; }
        .summary-card .label { font-size: 0.875rem; color: var(--color-text-muted); margin-top: 0.25rem; }
        .summary-card.passed .value { color: var(--color-passed); }
        .summary-card.warning .value { color: var(--color-warning); }
        .summary-card.failed .value { color: var(--color-failed); }
        .summary-card.grandfathered .value { color: var(--color-grandfathered); }
        .filter-controls { display: flex; gap: 0.5rem; margin-bottom: 1rem; flex-wrap: wrap; }
        .filter-btn { padding: 0.5rem 1rem; border: 1px solid var(--color-border); background: var(--color-card); border-radius: 0.375rem; cursor: pointer; font-size: 0.875rem; transition: all 0.15s; }
        .filter-btn:hover { background: var(--color-bg); }
        .filter-btn.active { background: var(--color-text); color: var(--color-card); border-color: var(--color-text); }
        .table-container { overflow-x: auto; }
        table { width: 100%; border-collapse: collapse; background: var(--color-card); border-radius: 0.5rem; overflow: hidden; border: 1px solid var(--color-border); }
        th, td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid var(--color-border); }
        th { background: var(--color-bg); font-weight: 600; font-size: 0.875rem; color: var(--color-text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
        th.sortable { cursor: pointer; user-select: none; }
        th.sortable:hover { background: #e2e8f0; }
        th.sortable::after { content: ''; display: inline-block; width: 0; height: 0; margin-left: 0.5rem; vertical-align: middle; opacity: 0.3; }
        th.sortable.asc::after { border-left: 4px solid transparent; border-right: 4px solid transparent; border-bottom: 6px solid currentColor; opacity: 1; }
        th.sortable.desc::after { border-left: 4px solid transparent; border-right: 4px solid transparent; border-top: 6px solid currentColor; opacity: 1; }
        td { font-size: 0.875rem; }
        td.number { text-align: right; font-variant-numeric: tabular-nums; }
        tr:last-child td { border-bottom: none; }
        tbody tr:hover { background: var(--color-bg); }
        tr.hidden { display: none; }
        .status { display: inline-flex; align-items: center; gap: 0.375rem; padding: 0.25rem 0.625rem; border-radius: 9999px; font-size: 0.75rem; font-weight: 600; }
        .status.passed { background: #dcfce7; color: #166534; }
        .status.warning { background: #fef9c3; color: #854d0e; }
        .status.failed { background: #fee2e2; color: #991b1b; }
        .status.grandfathered { background: #dbeafe; color: #1e40af; }
        .file-path { font-family: 'SF Mono', SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace; font-size: 0.8125rem; word-break: break-all; }
        .reason { font-size: 0.75rem; color: var(--color-text-muted); font-style: italic; }
        .suggestions { margin-top: 0.5rem; padding: 0.75rem; background: var(--color-bg); border-radius: 0.375rem; font-size: 0.75rem; }
        .suggestions h4 { font-size: 0.75rem; font-weight: 600; margin-bottom: 0.375rem; }
        .suggestions ul { list-style: none; margin: 0; padding: 0; }
        .suggestions li { padding: 0.25rem 0; font-family: 'SF Mono', SFMono-Regular, Consolas, monospace; }
        .footer { margin-top: 2rem; padding-top: 1rem; border-top: 1px solid var(--color-border); font-size: 0.75rem; color: var(--color-text-muted); text-align: center; }
        .no-results { padding: 2rem; text-align: center; color: var(--color-text-muted); }
        .charts-section { margin: 2rem 0; }
        .chart-container { background: var(--color-card); border-radius: 0.5rem; padding: 1.25rem; border: 1px solid var(--color-border); margin-bottom: 1rem; }
        .chart-container h3 { font-size: 1rem; font-weight: 600; margin-bottom: 1rem; color: var(--color-text); }
        .chart-container svg { width: 100%; height: auto; max-width: 500px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>SLOC Guard Report</h1>
"#;

const HTML_FOOTER: &str = r#"        <div class="footer">
            Generated by <strong>sloc-guard</strong>
        </div>
    </div>
    <script>
        (function() {
            // Status filter functionality
            const filterBtns = document.querySelectorAll('.filter-btn');
            const fileTable = document.getElementById('file-table');
            if (filterBtns.length && fileTable) {
                filterBtns.forEach(btn => {
                    btn.addEventListener('click', () => {
                        filterBtns.forEach(b => b.classList.remove('active'));
                        btn.classList.add('active');
                        const filter = btn.dataset.filter;
                        const rows = fileTable.querySelectorAll('tbody tr');
                        rows.forEach(row => {
                            const status = row.dataset.status;
                            if (filter === 'all') {
                                row.classList.remove('hidden');
                            } else if (filter === 'issues') {
                                row.classList.toggle('hidden', status === 'passed');
                            } else {
                                row.classList.toggle('hidden', status !== filter);
                            }
                        });
                    });
                });
            }

            // Sortable columns functionality
            const sortableHeaders = document.querySelectorAll('th.sortable');
            sortableHeaders.forEach(header => {
                header.addEventListener('click', () => {
                    const table = header.closest('table');
                    const tbody = table.querySelector('tbody');
                    const rows = Array.from(tbody.querySelectorAll('tr'));
                    const colIndex = Array.from(header.parentNode.children).indexOf(header);
                    const isAsc = header.classList.contains('asc');

                    // Update sort indicators
                    table.querySelectorAll('th.sortable').forEach(th => {
                        th.classList.remove('asc', 'desc');
                    });
                    header.classList.add(isAsc ? 'desc' : 'asc');

                    // Sort rows
                    const sortType = header.dataset.sort;
                    rows.sort((a, b) => {
                        let aVal, bVal;
                        if (sortType === 'status') {
                            const order = {failed: 0, warning: 1, grandfathered: 2, passed: 3};
                            aVal = order[a.dataset.status] ?? 4;
                            bVal = order[b.dataset.status] ?? 4;
                        } else if (sortType === 'number') {
                            aVal = parseInt(a.children[colIndex].dataset.value, 10) || 0;
                            bVal = parseInt(b.children[colIndex].dataset.value, 10) || 0;
                        } else {
                            aVal = a.children[colIndex].textContent.trim().toLowerCase();
                            bVal = b.children[colIndex].textContent.trim().toLowerCase();
                        }
                        if (aVal < bVal) return isAsc ? 1 : -1;
                        if (aVal > bVal) return isAsc ? -1 : 1;
                        return 0;
                    });

                    rows.forEach(row => tbody.appendChild(row));
                });
            });
        })();
    </script>
</body>
</html>
"#;

/// HTML formatter for generating standalone HTML reports.
pub struct HtmlFormatter {
    show_suggestions: bool,
    project_stats: Option<ProjectStatistics>,
}

impl HtmlFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_suggestions: false,
            project_stats: None,
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

    fn write_charts_section(output: &mut String, stats: &ProjectStatistics) {
        let histogram = FileSizeHistogram::from_stats(stats);
        let language_chart = LanguageBreakdownChart::from_stats(stats);

        // Only show charts section if there's sufficient data for any chart
        if !histogram.has_sufficient_data() && !language_chart.has_data() {
            return;
        }

        output.push_str("        <div class=\"charts-section\">\n");
        output.push_str("            <h2>Visualizations</h2>\n");

        // File Size Distribution Histogram
        if histogram.has_sufficient_data() {
            output.push_str("            <div class=\"chart-container\">\n");
            output.push_str("                <h3>File Size Distribution (by SLOC)</h3>\n");
            let svg = histogram.render();
            for line in svg.lines() {
                let _ = writeln!(output, "                {line}");
            }
            output.push_str("            </div>\n");
        }

        // Language Breakdown Chart
        if language_chart.has_data() {
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

        // Render charts if project stats are available
        if let Some(stats) = &self.project_stats {
            Self::write_charts_section(&mut output, stats);
        }

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
