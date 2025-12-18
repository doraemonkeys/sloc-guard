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
            ignored: 0,
        },
        limit,
        override_reason: None,
        suggestions: None,
    }
}

#[test]
fn formats_summary_correctly() {
    let results = vec![
        make_result("src/pass.rs", CheckStatus::Passed, 100, 500),
        make_result("src/warn.rs", CheckStatus::Warning, 450, 500),
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
    ];

    let formatter = MarkdownFormatter::new();
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

    let formatter = MarkdownFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("### Details"));
    assert!(output.contains("| Status | File | Lines | Limit | Code | Comment | Blank | Reason |"));
    assert!(output.contains("| ❌ Failed | `src/fail.rs` | 600 | 500 | 600 | 10 | 5 | - |"));
    assert!(output.contains("| ⚠️ Warning | `src/warn.rs` | 450 | 500 | 450 | 10 | 5 | - |"));
}

#[test]
fn excludes_passed_from_details() {
    let results = vec![
        make_result("src/pass.rs", CheckStatus::Passed, 100, 500),
        make_result("src/fail.rs", CheckStatus::Failed, 600, 500),
    ];

    let formatter = MarkdownFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("`src/pass.rs`"));
    assert!(output.contains("`src/fail.rs`"));
}

#[test]
fn shows_grandfathered_count() {
    let results = vec![
        make_result("src/legacy.rs", CheckStatus::Grandfathered, 800, 500),
    ];

    let formatter = MarkdownFormatter::new();
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

    let formatter = MarkdownFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("| ✅ Passed | 2 |"));
    assert!(!output.contains("### Details"));
}

#[test]
fn empty_results() {
    let results: Vec<CheckResult> = vec![];

    let formatter = MarkdownFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("| Total Files | 0 |"));
    assert!(!output.contains("### Details"));
}

#[test]
fn override_reason_shown_in_table() {
    let results = vec![CheckResult {
        path: PathBuf::from("src/legacy.rs"),
        status: CheckStatus::Warning,
        stats: LineStats {
            total: 765,
            code: 750,
            comment: 10,
            blank: 5, ignored: 0,
        },
        limit: 800,
        override_reason: Some("Legacy migration code".to_string()),
        suggestions: None,
    }];

    let formatter = MarkdownFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Legacy migration code"));
    assert!(output.contains("| ⚠️ Warning | `src/legacy.rs` | 750 | 800 | 750 | 10 | 5 | Legacy migration code |"));
}

#[test]
fn with_suggestions_shows_split_suggestions_section() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let mut result = make_result("src/big_file.rs", CheckStatus::Failed, 600, 500);
    let suggestion = SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500)
        .with_chunks(vec![
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

    let formatter = MarkdownFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();

    assert!(output.contains("### Split Suggestions"));
    assert!(output.contains("`big_file_part1.*`"));
    assert!(output.contains("`big_file_part2.*`"));
    assert!(output.contains("func1, func2"));
}

#[test]
fn without_suggestions_flag_hides_split_suggestions_section() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let mut result = make_result("src/big_file.rs", CheckStatus::Failed, 600, 500);
    let suggestion = SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500)
        .with_chunks(vec![SplitChunk {
            suggested_name: "big_file_part1".to_string(),
            functions: vec!["func1".to_string()],
            start_line: 1,
            end_line: 300,
            line_count: 300,
        }]);
    result.suggestions = Some(suggestion);

    let formatter = MarkdownFormatter::new().with_suggestions(false);
    let output = formatter.format(&[result]).unwrap();

    assert!(!output.contains("### Split Suggestions"));
}

#[test]
fn empty_functions_shows_dash() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let mut result = make_result("src/big_file.rs", CheckStatus::Failed, 600, 500);
    let suggestion = SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500)
        .with_chunks(vec![SplitChunk {
            suggested_name: "big_file_part1".to_string(),
            functions: vec![], // Empty functions
            start_line: 1,
            end_line: 300,
            line_count: 300,
        }]);
    result.suggestions = Some(suggestion);

    let formatter = MarkdownFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();

    assert!(output.contains("### Split Suggestions"));
    assert!(output.contains("| - |")); // Should show dash for empty functions
}

#[test]
fn default_formatter() {
    let formatter = MarkdownFormatter::default();
    let results = vec![make_result("src/test.rs", CheckStatus::Passed, 100, 500)];

    let output = formatter.format(&results).unwrap();
    assert!(output.contains("## SLOC Guard Results"));
}

#[test]
fn no_grandfathered_row_when_count_is_zero() {
    let results = vec![
        make_result("src/pass.rs", CheckStatus::Passed, 100, 500),
    ];

    let formatter = MarkdownFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("Grandfathered"));
}
