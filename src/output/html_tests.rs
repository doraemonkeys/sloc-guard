use std::path::PathBuf;

use crate::checker::CheckResult;
use crate::counter::LineStats;
use crate::output::OutputFormatter;

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
        limit,
        override_reason: None,
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
        limit,
        override_reason: None,
        suggestions: None,
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
        limit,
        override_reason: None,
        suggestions: None,
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
        limit,
        override_reason: None,
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
        limit: 800,
        override_reason: Some("Legacy migration code".to_string()),
        suggestions: None,
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
        limit: 500,
        override_reason: None,
        suggestions: None,
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
