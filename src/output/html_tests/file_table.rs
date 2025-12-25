use crate::checker::CheckResult;
use crate::output::OutputFormatter;

use super::{
    make_failed_result, make_grandfathered_result, make_passed_result, make_warning_result,
};
use crate::output::HtmlFormatter;

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
    assert!(output.contains("<th") && output.contains(">Total</th>"));
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
    assert!(output.contains(r#"data-value="615""#)); // total lines (code + comment + blank)
    assert!(output.contains(r#"data-value="600""#)); // code lines (sloc)
    assert!(output.contains(r#"data-value="500""#)); // limit
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
fn shows_override_reason() {
    use std::path::PathBuf;

    use crate::counter::LineStats;

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
