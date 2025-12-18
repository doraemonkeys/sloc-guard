use std::path::PathBuf;

use crate::checker::{CheckResult, CheckStatus};
use crate::counter::LineStats;
use crate::output::OutputFormatter;

use super::MarkdownFormatter;

fn make_result(path: &str, status: CheckStatus, code: usize, limit: usize) -> CheckResult {
    CheckResult {
        path: PathBuf::from(path),
        status,
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
        },
        limit,
    }
}

#[test]
fn formats_summary_correctly() {
    let results = vec![
        make_result("src/pass.rs", CheckStatus::Passed, 100, 500),
        make_result("src/warn.rs", CheckStatus::Warning, 450, 500),
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
    ];

    let formatter = MarkdownFormatter;
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("## SLOC Guard Results"));
    assert!(output.contains("| Total Files | 3 |"));
    assert!(output.contains("| ✅ Passed | 1 |"));
    assert!(output.contains("| ⚠️ Warnings | 1 |"));
    assert!(output.contains("| ❌ Failed | 1 |"));
}

#[test]
fn formats_details_table() {
    let results = vec![
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
        make_result("src/warn.rs", CheckStatus::Warning, 450, 500),
    ];

    let formatter = MarkdownFormatter;
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("### Details"));
    assert!(output.contains("| Status | File | Lines | Limit | Code | Comment | Blank |"));
    assert!(output.contains("| ❌ Failed | `src/fail.rs` | 600 | 500 | 600 | 10 | 5 |"));
    assert!(output.contains("| ⚠️ Warning | `src/warn.rs` | 450 | 500 | 450 | 10 | 5 |"));
}

#[test]
fn excludes_passed_from_details() {
    let results = vec![
        make_result("src/pass.rs", CheckStatus::Passed, 100, 500),
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
    ];

    let formatter = MarkdownFormatter;
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("`src/pass.rs`"));
    assert!(output.contains("`src/fail.rs`"));
}

#[test]
fn shows_grandfathered_count() {
    let results = vec![
        make_result("src/legacy.rs", CheckStatus::Grandfathered, 800, 500),
    ];

    let formatter = MarkdownFormatter;
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("| 🔵 Grandfathered | 1 |"));
    assert!(output.contains("| 🔵 Grandfathered | `src/legacy.rs`"));
}

#[test]
fn no_details_section_when_all_passed() {
    let results = vec![
        make_result("src/a.rs", CheckStatus::Passed, 100, 500),
        make_result("src/b.rs", CheckStatus::Passed, 200, 500),
    ];

    let formatter = MarkdownFormatter;
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("| ✅ Passed | 2 |"));
    assert!(!output.contains("### Details"));
}

#[test]
fn empty_results() {
    let results: Vec<CheckResult> = vec![];

    let formatter = MarkdownFormatter;
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("| Total Files | 0 |"));
    assert!(!output.contains("### Details"));
}
