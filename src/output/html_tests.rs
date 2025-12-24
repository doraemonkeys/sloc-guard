use std::path::PathBuf;

use crate::checker::CheckResult;
use crate::counter::LineStats;
use crate::output::{FileStatistics, OutputFormatter, ProjectStatistics};

use super::{HtmlFormatter, html_escape};

fn make_passed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Passed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        violation_category: None,
    }
}

fn make_warning_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Warning {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

fn make_failed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Failed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

fn make_grandfathered_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Grandfathered {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        violation_category: None,
    }
}

#[test]
fn generates_valid_html_structure() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.starts_with("<!DOCTYPE html>"));
    assert!(output.contains("<html lang=\"en\">"));
    assert!(output.contains("<head>"));
    assert!(output.contains("<title>SLOC Guard Report</title>"));
    assert!(output.contains("<style>"));
    assert!(output.contains("</style>"));
    assert!(output.contains("<body>"));
    assert!(output.contains("</body>"));
    assert!(output.contains("</html>"));
}

#[test]
fn formats_summary_correctly() {
    let results = vec![
        make_passed_result("src/pass.rs", 100, 500),
        make_warning_result("src/warn.rs", 450, 500),
        make_failed_result("src/fail.rs", 600, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("<h1>SLOC Guard Report</h1>"));
    assert!(output.contains("summary-grid"));
    assert!(output.contains(r#"<span class="value">3</span>"#)); // total
    assert!(output.contains("Total Files"));
    assert!(output.contains(r#"<span class="value">1</span>"#)); // passed, warnings, failed each 1
}

#[test]
fn shows_passed_card() {
    let results = vec![make_passed_result("src/pass.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card passed""#));
    assert!(output.contains("Passed"));
}

#[test]
fn shows_warning_card() {
    let results = vec![make_warning_result("src/warn.rs", 450, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card warning""#));
    assert!(output.contains("Warnings"));
}

#[test]
fn shows_failed_card() {
    let results = vec![make_failed_result("src/fail.rs", 600, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card failed""#));
    assert!(output.contains("Failed"));
}

#[test]
fn shows_grandfathered_card_when_present() {
    let results = vec![make_grandfathered_result("src/legacy.rs", 800, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card grandfathered""#));
    assert!(output.contains("Grandfathered"));
}

#[test]
fn hides_grandfathered_card_when_zero() {
    let results = vec![make_passed_result("src/pass.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains(r#"class="summary-card grandfathered""#));
}

#[test]
fn formats_file_table() {
    let results = vec![
        make_failed_result("src/fail.rs", 600, 500),
        make_warning_result("src/warn.rs", 450, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("<h2>All Files</h2>"));
    assert!(output.contains("<table id=\"file-table\">"));
    assert!(output.contains("class=\"sortable\""));
    assert!(output.contains("<th>Status</th>") || output.contains("data-sort=\"status\">Status"));
    assert!(output.contains("<th") && output.contains(">File</th>"));
    assert!(output.contains("<th") && output.contains(">Lines</th>"));
    assert!(output.contains("<th") && output.contains(">Limit</th>"));
    assert!(output.contains("src/fail.rs"));
    assert!(output.contains("src/warn.rs"));
}

#[test]
fn shows_all_files_including_passed() {
    let results = vec![
        make_passed_result("src/pass.rs", 100, 500),
        make_failed_result("src/fail.rs", 600, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Now shows all files including passed
    assert!(output.contains("src/pass.rs"));
    assert!(output.contains("src/fail.rs"));
    // Both should have data-status attributes
    assert!(output.contains("data-status=\"passed\""));
    assert!(output.contains("data-status=\"failed\""));
}

#[test]
fn shows_all_passed_files() {
    let results = vec![
        make_passed_result("src/a.rs", 100, 500),
        make_passed_result("src/b.rs", 200, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Now shows the table with all files even when all passed
    assert!(output.contains("<h2>All Files</h2>"));
    assert!(output.contains("src/a.rs"));
    assert!(output.contains("src/b.rs"));
}

#[test]
fn empty_results() {
    let results: Vec<CheckResult> = vec![];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"<span class="value">0</span>"#));
    assert!(output.contains("No files to display."));
}

#[test]
fn shows_override_reason() {
    let results = vec![CheckResult::Warning {
        path: PathBuf::from("src/legacy.rs"),
        stats: LineStats {
            total: 765,
            code: 750,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit: 800,
        override_reason: Some("Legacy migration code".to_string()),
        suggestions: None,
        violation_category: None,
    }];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="reason""#));
    assert!(output.contains("Legacy migration code"));
}

#[test]
fn shows_split_suggestions_when_enabled() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let result = make_failed_result("src/big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string(), "func2".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
            SplitChunk {
                suggested_name: "big_file_part2".to_string(),
                functions: vec!["func3".to_string()],
                start_line: 301,
                end_line: 600,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = HtmlFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();

    assert!(output.contains(r#"class="suggestions""#));
    assert!(output.contains("Split suggestions:"));
    assert!(output.contains("big_file_part1.*"));
    assert!(output.contains("big_file_part2.*"));
    assert!(output.contains("func1, func2"));
}

#[test]
fn hides_suggestions_when_disabled() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let result = make_failed_result("src/big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = HtmlFormatter::new().with_suggestions(false);
    let output = formatter.format(&[result]).unwrap();

    assert!(!output.contains(r#"class="suggestions""#));
}

#[test]
fn html_escape_special_characters() {
    assert_eq!(html_escape("<script>"), "&lt;script&gt;");
    assert_eq!(html_escape("a & b"), "a &amp; b");
    assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    assert_eq!(html_escape("it's"), "it&#39;s");
}

#[test]
fn escapes_file_paths() {
    let results = vec![CheckResult::Failed {
        path: PathBuf::from("src/<script>.rs"),
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // File path should be escaped
    assert!(output.contains("&lt;script&gt;"));
    // The raw file path should not appear in the table (only the escaped version)
    assert!(!output.contains("src/<script>.rs"));
}

#[test]
fn default_formatter() {
    let formatter = HtmlFormatter::default();
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let output = formatter.format(&results).unwrap();
    assert!(output.contains("<!DOCTYPE html>"));
}

#[test]
fn has_embedded_css() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Check for key CSS properties
    assert!(output.contains(":root {"));
    assert!(output.contains("--color-passed:"));
    assert!(output.contains("--color-warning:"));
    assert!(output.contains("--color-failed:"));
    assert!(output.contains(".summary-card"));
    assert!(output.contains(".status.passed"));
    assert!(output.contains(".status.warning"));
    assert!(output.contains(".status.failed"));
}

#[test]
fn has_footer() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="footer""#));
    assert!(output.contains("sloc-guard"));
}

#[test]
fn status_icons_are_html_entities() {
    let results = vec![
        make_failed_result("src/fail.rs", 600, 500),
        make_warning_result("src/warn.rs", 450, 500),
        make_grandfathered_result("src/grandfather.rs", 800, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Check that HTML entities are used instead of raw unicode
    assert!(output.contains("&#x2717;")); // ✗ for failed
    assert!(output.contains("&#x26A0;")); // ⚠ for warning
    assert!(output.contains("&#x25C9;")); // ◉ for grandfathered
}

#[test]
fn numeric_columns_have_number_class() {
    let results = vec![make_failed_result("src/fail.rs", 600, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Check that numeric cells have the "number" class
    assert!(output.contains(r#"class="number""#));
}

#[test]
fn has_filter_controls() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("filter-controls"));
    assert!(output.contains("filter-btn"));
    assert!(output.contains(r#"data-filter="all""#));
    assert!(output.contains(r#"data-filter="issues""#));
    assert!(output.contains(r#"data-filter="failed""#));
    assert!(output.contains(r#"data-filter="warning""#));
    assert!(output.contains(r#"data-filter="passed""#));
}

#[test]
fn has_sortable_column_headers() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="sortable""#));
    assert!(output.contains(r#"data-sort="status""#));
    assert!(output.contains(r#"data-sort="text""#));
    assert!(output.contains(r#"data-sort="number""#));
}

#[test]
fn rows_have_data_status_attribute() {
    let results = vec![
        make_passed_result("src/pass.rs", 100, 500),
        make_failed_result("src/fail.rs", 600, 500),
        make_warning_result("src/warn.rs", 450, 500),
        make_grandfathered_result("src/legacy.rs", 800, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"data-status="passed""#));
    assert!(output.contains(r#"data-status="failed""#));
    assert!(output.contains(r#"data-status="warning""#));
    assert!(output.contains(r#"data-status="grandfathered""#));
}

#[test]
fn numeric_cells_have_data_value_attribute() {
    let results = vec![make_failed_result("src/test.rs", 600, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Check that numeric cells have data-value for sorting
    assert!(output.contains(r#"data-value="600""#)); // code lines (sloc)
    assert!(output.contains(r#"data-value="500""#)); // limit
}

#[test]
fn has_client_side_javascript() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("<script>"));
    assert!(output.contains("</script>"));
    // Check for filter functionality
    assert!(output.contains("filter-btn"));
    assert!(output.contains("filterBtns"));
    // Check for sort functionality
    assert!(output.contains("sortableHeaders"));
    assert!(output.contains("data-sort"));
}

#[test]
fn table_has_id_for_js() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"id="file-table""#));
}

#[test]
fn has_table_container() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("table-container"));
}

#[test]
fn split_suggestions_with_empty_functions() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let result = make_failed_result("src/big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec![], // Empty functions list
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = HtmlFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();

    assert!(output.contains("Split suggestions:"));
    assert!(output.contains("big_file_part1.*"));
    // Empty functions should not produce parentheses
    assert!(!output.contains("big_file_part1.* (~300 lines) ("));
}

// ============================================================================
// File size distribution histogram tests
// ============================================================================

fn make_file_stats(path: &str, code: usize, language: &str) -> FileStatistics {
    FileStatistics {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        language: language.to_string(),
    }
}

fn make_project_stats(code_lines: &[usize]) -> ProjectStatistics {
    let files: Vec<FileStatistics> = code_lines
        .iter()
        .enumerate()
        .map(|(i, &lines)| make_file_stats(&format!("file{i}.rs"), lines, "Rust"))
        .collect();
    ProjectStatistics::new(files)
}

#[test]
fn histogram_not_shown_without_stats() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // No histogram chart without stats (CSS is always present, but no chart HTML)
    assert!(!output.contains("File Size Distribution (by SLOC)"));
    assert!(!output.contains("<h2>Visualizations</h2>"));
}

#[test]
fn histogram_not_shown_with_insufficient_files() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let stats = make_project_stats(&[25, 50]); // Only 2 files

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Should not show charts with < 3 files
    assert!(!output.contains("File Size Distribution (by SLOC)"));
    assert!(!output.contains("<h2>Visualizations</h2>"));
}

#[test]
fn histogram_shown_with_sufficient_files() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let stats = make_project_stats(&[25, 75, 150]); // 3 files

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Charts section should be present
    assert!(output.contains("charts-section"));
    assert!(output.contains("Visualizations"));
    assert!(output.contains("File Size Distribution"));
    assert!(output.contains("<svg"));
}

#[test]
fn histogram_has_chart_css_variable() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Chart CSS variable should be defined
    assert!(output.contains("--color-chart-primary"));
}

#[test]
fn histogram_has_chart_container_styles() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Chart container styles should be defined
    assert!(output.contains(".chart-container"));
}

// ============================================================================
// Language breakdown chart tests
// ============================================================================

fn make_project_stats_with_languages(files: Vec<FileStatistics>) -> ProjectStatistics {
    ProjectStatistics::new(files).with_language_breakdown()
}

#[test]
fn language_chart_not_shown_without_stats() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("Language Breakdown"));
}

#[test]
fn language_chart_not_shown_without_language_breakdown() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    // Stats without with_language_breakdown()
    let stats = make_project_stats(&[25, 50, 75]);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Histogram should appear but not language chart
    assert!(output.contains("File Size Distribution"));
    assert!(!output.contains("Language Breakdown"));
}

#[test]
fn language_chart_shown_with_language_breakdown() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("lib.rs", 100, "Rust"),
        make_file_stats("app.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Language breakdown section should be present
    assert!(output.contains("Language Breakdown"));
    assert!(output.contains("Rust"));
    assert!(output.contains("Go"));
}

#[test]
fn language_chart_shows_sloc_values() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("lib.rs", 100, "Rust"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Total Rust SLOC: 200 + 100 = 300
    assert!(output.contains("300"));
}

#[test]
fn language_chart_has_horizontal_bars() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("app.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Should have rect elements for bars
    assert!(output.contains("<rect"));
}

#[test]
fn both_charts_shown_when_data_available() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("a.rs", 25, "Rust"),
        make_file_stats("b.rs", 75, "Rust"),
        make_file_stats("c.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Both charts should be present
    assert!(output.contains("File Size Distribution"));
    assert!(output.contains("Language Breakdown"));
}

#[test]
fn only_language_chart_when_insufficient_histogram_files() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    // Only 2 files (not enough for histogram), but has language data
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("app.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Visualization section should still appear for language chart
    assert!(output.contains("Visualizations"));
    assert!(output.contains("Language Breakdown"));
    // But histogram should not appear
    assert!(!output.contains("File Size Distribution"));
}

#[test]
fn language_chart_uses_css_variables() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![make_file_stats("main.rs", 200, "Rust")];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Chart should use CSS variables for theming
    assert!(output.contains("var(--color-"));
}

// ============================================================================
// Trend line chart tests
// ============================================================================

use crate::stats::TrendHistory;

// Timestamp constants for test dates
const TS_2023_12_24: u64 = 1_703_376_000;
const TS_2023_12_25: u64 = 1_703_462_400;
const TS_2023_12_26: u64 = 1_703_548_800;

fn make_trend_entry(timestamp: u64, code: usize) -> crate::stats::TrendEntry {
    crate::stats::TrendEntry {
        timestamp,
        total_files: 10,
        total_lines: code + 100,
        code,
        comment: 50,
        blank: 50,
        git_ref: None,
        git_branch: None,
    }
}

#[test]
fn trend_chart_not_shown_without_history() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("Code Lines Over Time"));
}

#[test]
fn trend_chart_not_shown_with_empty_history() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let history = TrendHistory::new();

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Empty history should not show chart section
    assert!(!output.contains("Code Lines Over Time"));
    assert!(!output.contains("Visualizations"));
}

#[test]
fn trend_chart_shown_with_history() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Visualizations"));
    assert!(output.contains("Code Lines Over Time"));
    assert!(output.contains("<svg"));
}

#[test]
fn trend_chart_has_line_path() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));
    history.add_entry(make_trend_entry(TS_2023_12_26, 500));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Should have path element for the line
    assert!(output.contains("<path"));
}

#[test]
fn trend_chart_has_data_points() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Should have circle elements for data points
    assert!(output.contains("<circle"));
}

#[test]
fn trend_chart_with_stats_shows_both() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    // Trend history
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    // Project stats
    let files = vec![
        make_file_stats("a.rs", 100, "Rust"),
        make_file_stats("b.rs", 200, "Rust"),
        make_file_stats("c.rs", 150, "Rust"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new()
        .with_stats(stats)
        .with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // All three charts should be present
    assert!(output.contains("Code Lines Over Time"));
    assert!(output.contains("File Size Distribution"));
    assert!(output.contains("Language Breakdown"));
}

#[test]
fn trend_chart_appears_first_in_visualizations() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let files = vec![
        make_file_stats("a.rs", 100, "Rust"),
        make_file_stats("b.rs", 200, "Rust"),
        make_file_stats("c.rs", 150, "Rust"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new()
        .with_stats(stats)
        .with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Trend chart should appear before histogram
    let trend_pos = output.find("Code Lines Over Time").unwrap();
    let hist_pos = output.find("File Size Distribution").unwrap();
    assert!(trend_pos < hist_pos, "Trend chart should appear first");
}

#[test]
fn only_trend_chart_when_no_stats() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Only trend chart should appear
    assert!(output.contains("Visualizations"));
    assert!(output.contains("Code Lines Over Time"));
    assert!(!output.contains("File Size Distribution"));
    assert!(!output.contains("Language Breakdown"));
}
