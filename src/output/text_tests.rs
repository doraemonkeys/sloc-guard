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
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("1 passed"));
    assert!(output.contains("test.rs") || output.contains("Summary"));
}

#[test]
fn format_failed_result() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("FAILED"));
    assert!(output.contains("600"));
    assert!(output.contains("limit: 500"));
}

#[test]
fn format_warning_result() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_result("test.rs", 460, 500, CheckStatus::Warning)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("WARNING"));
}

#[test]
fn format_mixed_results() {
    let formatter = TextFormatter::new(ColorMode::Never);
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
    let formatter = TextFormatter::new(ColorMode::Never);
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
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Summary:"));
    assert!(output.contains("1 files checked"));
}

#[test]
fn color_mode_always_produces_colored_output() {
    let formatter = TextFormatter::new(ColorMode::Always);
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

    let output = formatter.format(&results).unwrap();

    // ANSI escape codes start with \x1b[
    assert!(output.contains("\x1b["), "Output should contain ANSI color codes");
}

#[test]
fn color_mode_never_produces_plain_output() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

    let output = formatter.format(&results).unwrap();

    // Should not contain ANSI escape codes
    assert!(!output.contains("\x1b["), "Output should not contain ANSI color codes");
}
