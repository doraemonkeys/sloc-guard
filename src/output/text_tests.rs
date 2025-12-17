use std::path::PathBuf;

use super::*;
use crate::counter::LineStats;

fn make_result(path: &str, code: usize, limit: usize, status: CheckStatus) -> CheckResult {
    CheckResult {
        path: PathBuf::from(path),
        status,
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
        },
        limit,
    }
}

#[test]
fn format_passed_result() {
    let formatter = TextFormatter::new(false);
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("1 passed"));
    assert!(output.contains("test.rs") || output.contains("Summary"));
}

#[test]
fn format_failed_result() {
    let formatter = TextFormatter::new(false);
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("FAILED"));
    assert!(output.contains("600"));
    assert!(output.contains("limit: 500"));
}

#[test]
fn format_warning_result() {
    let formatter = TextFormatter::new(false);
    let results = vec![make_result("test.rs", 460, 500, CheckStatus::Warning)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("WARNING"));
}

#[test]
fn format_mixed_results() {
    let formatter = TextFormatter::new(false);
    let results = vec![
        make_result("passed.rs", 100, 500, CheckStatus::Passed),
        make_result("warning.rs", 460, 500, CheckStatus::Warning),
        make_result("failed.rs", 600, 500, CheckStatus::Failed),
    ];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("1 passed"));
    assert!(output.contains("1 warnings"));
    assert!(output.contains("1 failed"));
}

#[test]
fn failed_results_shown_first() {
    let formatter = TextFormatter::new(false);
    let results = vec![
        make_result("passed.rs", 100, 500, CheckStatus::Passed),
        make_result("failed.rs", 600, 500, CheckStatus::Failed),
    ];

    let output = formatter.format(&results).unwrap();
    let failed_pos = output.find("failed.rs").unwrap();
    let summary_pos = output.find("Summary").unwrap();

    assert!(failed_pos < summary_pos);
}

#[test]
fn summary_line_included() {
    let formatter = TextFormatter::new(false);
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Summary:"));
    assert!(output.contains("1 files checked"));
}
