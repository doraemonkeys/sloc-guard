use crate::checker::CheckResult;
use crate::output::OutputFormatter;

use super::{
    make_failed_result, make_grandfathered_result, make_passed_result, make_warning_result,
};
use crate::output::HtmlFormatter;

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
fn shows_aggregate_line_stats() {
    let results = vec![
        make_passed_result("src/a.rs", 100, 500), // total: 115 (100 + 10 + 5)
        make_failed_result("src/b.rs", 200, 150), // total: 215 (200 + 10 + 5)
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Check for aggregate stats labels
    assert!(output.contains("Total Lines"));
    assert!(output.contains("Code"));
    assert!(output.contains("Comments"));
    assert!(output.contains("Blanks"));

    // Check for aggregate values (100+200=300 code, 10+10=20 comments, 5+5=10 blanks, 115+215=330 total)
    assert!(output.contains(r#"<span class="value">330</span>"#)); // total lines
    assert!(output.contains(r#"<span class="value">300</span>"#)); // code
    assert!(output.contains(r#"<span class="value">20</span>"#)); // comments
    assert!(output.contains(r#"<span class="value">10</span>"#)); // blanks
}

#[test]
fn aggregate_stats_with_empty_results() {
    let results: Vec<CheckResult> = vec![];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Should still show aggregate stats section with zeros
    assert!(output.contains("Total Lines"));
    assert!(output.contains("Code"));
    assert!(output.contains("Comments"));
    assert!(output.contains("Blanks"));
}
