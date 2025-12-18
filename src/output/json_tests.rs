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
            ignored: 0,
        },
        limit,
        override_reason: None,
        suggestions: None,
    }
}

#[test]
fn json_output_is_valid() {
    let formatter = JsonFormatter::new();
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(parsed.get("summary").is_some());
    assert!(parsed.get("results").is_some());
}

#[test]
fn json_summary_counts() {
    let formatter = JsonFormatter::new();
    let results = vec![
        make_result("pass.rs", 100, 500, CheckStatus::Passed),
        make_result("warn.rs", 460, 500, CheckStatus::Warning),
        make_result("fail.rs", 600, 500, CheckStatus::Failed),
    ];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let summary = parsed.get("summary").unwrap();
    assert_eq!(summary.get("total_files").unwrap(), 3);
    assert_eq!(summary.get("passed").unwrap(), 1);
    assert_eq!(summary.get("warnings").unwrap(), 1);
    assert_eq!(summary.get("failed").unwrap(), 1);
}

#[test]
fn json_result_fields() {
    let formatter = JsonFormatter::new();
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let file_result = &parsed.get("results").unwrap()[0];
    assert_eq!(file_result.get("path").unwrap(), "test.rs");
    assert_eq!(file_result.get("status").unwrap(), "passed");
    assert_eq!(file_result.get("sloc").unwrap(), 100);
    assert_eq!(file_result.get("limit").unwrap(), 500);

    let stats = file_result.get("stats").unwrap();
    assert_eq!(stats.get("code").unwrap(), 100);
    assert_eq!(stats.get("comment").unwrap(), 5);
    assert_eq!(stats.get("blank").unwrap(), 5);
}

#[test]
fn json_status_values() {
    let formatter = JsonFormatter::new();
    let results = vec![
        make_result("pass.rs", 100, 500, CheckStatus::Passed),
        make_result("warn.rs", 460, 500, CheckStatus::Warning),
        make_result("fail.rs", 600, 500, CheckStatus::Failed),
    ];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let results_arr = parsed.get("results").unwrap().as_array().unwrap();
    assert_eq!(results_arr[0].get("status").unwrap(), "passed");
    assert_eq!(results_arr[1].get("status").unwrap(), "warning");
    assert_eq!(results_arr[2].get("status").unwrap(), "failed");
}

#[test]
fn json_empty_results() {
    let formatter = JsonFormatter::new();
    let results: Vec<CheckResult> = vec![];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let summary = parsed.get("summary").unwrap();
    assert_eq!(summary.get("total_files").unwrap(), 0);
}

#[test]
fn json_override_reason_included() {
    let formatter = JsonFormatter::new();
    let results = vec![CheckResult {
        path: PathBuf::from("legacy.rs"),
        status: CheckStatus::Warning,
        stats: LineStats {
            total: 760,
            code: 750,
            comment: 5,
            blank: 5, ignored: 0,
        },
        limit: 800,
        override_reason: Some("Legacy code from migration".to_string()),
        suggestions: None,
    }];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let file_result = &parsed.get("results").unwrap()[0];
    assert_eq!(
        file_result.get("override_reason").unwrap(),
        "Legacy code from migration"
    );
}

#[test]
fn json_override_reason_excluded_when_none() {
    let formatter = JsonFormatter::new();
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let file_result = &parsed.get("results").unwrap()[0];
    assert!(file_result.get("override_reason").is_none());
}

#[test]
fn json_grandfathered_status() {
    let formatter = JsonFormatter::new();
    let results = vec![CheckResult {
        path: PathBuf::from("legacy.rs"),
        status: CheckStatus::Grandfathered,
        stats: LineStats {
            total: 610,
            code: 600,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
        suggestions: None,
    }];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let summary = parsed.get("summary").unwrap();
    assert_eq!(summary.get("grandfathered").unwrap(), 1);

    let file_result = &parsed.get("results").unwrap()[0];
    assert_eq!(file_result.get("status").unwrap(), "grandfathered");
}

#[test]
fn json_with_suggestions() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let mut result = make_result("big_file.rs", 600, 500, CheckStatus::Failed);
    let suggestion = SplitSuggestion::new(PathBuf::from("big_file.rs"), 600, 500)
        .with_chunks(vec![SplitChunk {
            suggested_name: "big_file_part1".to_string(),
            functions: vec!["func1".to_string()],
            start_line: 1,
            end_line: 300,
            line_count: 300,
        }]);
    result.suggestions = Some(suggestion);

    let formatter = JsonFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let file_result = &parsed.get("results").unwrap()[0];
    assert!(file_result.get("suggestions").is_some());
}

#[test]
fn json_without_suggestions_flag_excludes_suggestions() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let mut result = make_result("big_file.rs", 600, 500, CheckStatus::Failed);
    let suggestion = SplitSuggestion::new(PathBuf::from("big_file.rs"), 600, 500)
        .with_chunks(vec![SplitChunk {
            suggested_name: "big_file_part1".to_string(),
            functions: vec!["func1".to_string()],
            start_line: 1,
            end_line: 300,
            line_count: 300,
        }]);
    result.suggestions = Some(suggestion);

    let formatter = JsonFormatter::new().with_suggestions(false);
    let output = formatter.format(&[result]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let file_result = &parsed.get("results").unwrap()[0];
    assert!(file_result.get("suggestions").is_none());
}

#[test]
fn json_default_formatter() {
    let formatter = JsonFormatter::default();
    let results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    let output = formatter.format(&results).unwrap();
    assert!(output.contains("summary"));
}
