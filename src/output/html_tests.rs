use std::path::PathBuf;

use crate::checker::{CheckResult, CheckStatus};
use crate::counter::LineStats;
use crate::output::OutputFormatter;

use super::{html_escape, HtmlFormatter};

fn make_result(path: &str, status: CheckStatus, code: usize, limit: usize) -> CheckResult {
    CheckResult {
        path: PathBuf::from(path),
        status,
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

#[test]
fn generates_valid_html_structure() {
    let results = vec![make_result("src/test.rs", CheckStatus::Passed, 100, 500)];

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
        make_result("src/pass.rs", CheckStatus::Passed, 100, 500),
        make_result("src/warn.rs", CheckStatus::Warning, 450, 500),
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
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
    let results = vec![make_result("src/pass.rs", CheckStatus::Passed, 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card passed""#));
    assert!(output.contains("Passed"));
}

#[test]
fn shows_warning_card() {
    let results = vec![make_result("src/warn.rs", CheckStatus::Warning, 450, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card warning""#));
    assert!(output.contains("Warnings"));
}

#[test]
fn shows_failed_card() {
    let results = vec![make_result("src/fail.rs", CheckStatus::Failed, 600, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card failed""#));
    assert!(output.contains("Failed"));
}

#[test]
fn shows_grandfathered_card_when_present() {
    let results = vec![make_result(
        "src/legacy.rs",
        CheckStatus::Grandfathered,
        800,
        500,
    )];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="summary-card grandfathered""#));
    assert!(output.contains("Grandfathered"));
}

#[test]
fn hides_grandfathered_card_when_zero() {
    let results = vec![make_result("src/pass.rs", CheckStatus::Passed, 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains(r#"class="summary-card grandfathered""#));
}

#[test]
fn formats_file_table() {
    let results = vec![
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
        make_result("src/warn.rs", CheckStatus::Warning, 450, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("<h2>Details</h2>"));
    assert!(output.contains("<table>"));
    assert!(output.contains("<th>Status</th>"));
    assert!(output.contains("<th>File</th>"));
    assert!(output.contains("<th>Lines</th>"));
    assert!(output.contains("<th>Limit</th>"));
    assert!(output.contains("src/fail.rs"));
    assert!(output.contains("src/warn.rs"));
}

#[test]
fn excludes_passed_from_details() {
    let results = vec![
        make_result("src/pass.rs", CheckStatus::Passed, 100, 500),
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("src/pass.rs"));
    assert!(output.contains("src/fail.rs"));
}

#[test]
fn no_details_when_all_passed() {
    let results = vec![
        make_result("src/a.rs", CheckStatus::Passed, 100, 500),
        make_result("src/b.rs", CheckStatus::Passed, 200, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("All files passed the SLOC check!"));
    assert!(!output.contains("<h2>Details</h2>"));
}

#[test]
fn empty_results() {
    let results: Vec<CheckResult> = vec![];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"<span class="value">0</span>"#));
    assert!(output.contains("All files passed the SLOC check!"));
}

#[test]
fn shows_override_reason() {
    let results = vec![CheckResult {
        path: PathBuf::from("src/legacy.rs"),
        status: CheckStatus::Warning,
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

    let mut result = make_result("src/big_file.rs", CheckStatus::Failed, 600, 500);
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
    result.suggestions = Some(suggestion);

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

    let mut result = make_result("src/big_file.rs", CheckStatus::Failed, 600, 500);
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
    result.suggestions = Some(suggestion);

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
    let results = vec![CheckResult {
        path: PathBuf::from("src/<script>.rs"),
        status: CheckStatus::Failed,
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

    assert!(output.contains("&lt;script&gt;"));
    assert!(!output.contains("<script>"));
}

#[test]
fn default_formatter() {
    let formatter = HtmlFormatter::default();
    let results = vec![make_result("src/test.rs", CheckStatus::Passed, 100, 500)];

    let output = formatter.format(&results).unwrap();
    assert!(output.contains("<!DOCTYPE html>"));
}

#[test]
fn has_embedded_css() {
    let results = vec![make_result("src/test.rs", CheckStatus::Passed, 100, 500)];

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
    let results = vec![make_result("src/test.rs", CheckStatus::Passed, 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains(r#"class="footer""#));
    assert!(output.contains("sloc-guard"));
}

#[test]
fn status_icons_are_html_entities() {
    let results = vec![
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
        make_result("src/warn.rs", CheckStatus::Warning, 450, 500),
        make_result("src/grandfather.rs", CheckStatus::Grandfathered, 800, 500),
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
    let results = vec![make_result("src/fail.rs", CheckStatus::Failed, 600, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Check that numeric cells have the "number" class
    assert!(output.contains(r#"class="number""#));
}
