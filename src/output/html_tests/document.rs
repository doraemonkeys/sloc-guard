use crate::output::OutputFormatter;

use super::{make_passed_result, make_warning_result};
use crate::output::HtmlFormatter;

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
fn status_icons_are_html_entities() {
    let results = vec![
        super::make_failed_result("src/fail.rs", 600, 500),
        make_warning_result("src/warn.rs", 450, 500),
        super::make_grandfathered_result("src/grandfather.rs", 800, 500),
    ];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Check that HTML entities are used instead of raw unicode
    assert!(output.contains("&#x2717;")); // ✗ for failed
    assert!(output.contains("&#x26A0;")); // ⚠ for warning
    assert!(output.contains("&#x25C9;")); // ◉ for grandfathered
}
