use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::stats::{FileStatistics, ProjectStatistics, StatsFormatter, StatsJsonFormatter};
use crate::stats::TrendDelta;

// Helper function to create test FileStatistics
fn file_stats(
    path: &str,
    total: usize,
    code: usize,
    comment: usize,
    blank: usize,
    language: &str,
) -> FileStatistics {
    FileStatistics {
        path: PathBuf::from(path),
        stats: LineStats {
            total,
            code,
            comment,
            blank,
            ignored: 0,
        },
        language: language.to_string(),
    }
}

#[test]
fn json_formatter_empty() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    assert!(output.contains("\"total_files\": 0"));
    // Empty files array is now omitted (skip_serializing_if = "Vec::is_empty")
    assert!(
        !output.contains("\"files\":"),
        "empty files array should be omitted"
    );
}

#[test]
fn json_formatter_with_files() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    assert!(output.contains("\"total_files\": 1"));
    assert!(output.contains("\"total_lines\": 100"));
    assert!(output.contains("\"code\": 80"));
    assert!(output.contains("\"test.rs\""));
    assert!(output.contains("\"language\": \"Rust\""));
}

#[test]
fn json_formatter_valid_json() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("summary").is_some());
    assert!(parsed.get("files").is_some());
}

#[test]
fn json_formatter_with_language_breakdown() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("main.go", 50, 40, 5, 5, "Go"),
    ];

    let stats = ProjectStatistics::new(files).with_language_breakdown();
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("by_language").is_some());
    let by_language = parsed.get("by_language").unwrap().as_array().unwrap();
    assert_eq!(by_language.len(), 2);
}

#[test]
fn json_formatter_with_top_files() {
    let files = vec![
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("small.rs", 50, 30, 10, 10, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_top_files(5);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("top_files").is_some());
    let top_files = parsed.get("top_files").unwrap().as_array().unwrap();
    assert_eq!(top_files.len(), 2);

    let summary = parsed.get("summary").unwrap();
    assert!(summary.get("average_code_lines").is_some());
}

#[test]
fn json_formatter_with_directory_breakdown() {
    let files = vec![
        file_stats("src/main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("tests/test.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown();
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("by_directory").is_some());
    let by_directory = parsed.get("by_directory").unwrap().as_array().unwrap();
    assert_eq!(by_directory.len(), 2);
}

// ============================================================================
// Trend formatting tests
// ============================================================================

fn sample_trend_delta() -> TrendDelta {
    TrendDelta {
        files_delta: 5,
        lines_delta: 100,
        code_delta: 50,
        comment_delta: 30,
        blank_delta: 20,
        previous_timestamp: Some(12345),
        previous_git_ref: None,
        previous_git_branch: None,
    }
}

#[test]
fn json_formatter_with_trend() {
    let stats = ProjectStatistics::new(vec![]).with_trend(sample_trend_delta());
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("trend").is_some());
    let trend = parsed.get("trend").unwrap();
    assert_eq!(trend.get("files").unwrap().as_i64().unwrap(), 5);
    assert_eq!(trend.get("code").unwrap().as_i64().unwrap(), 50);
}

#[test]
fn json_formatter_without_trend() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("trend").is_none());
}

#[test]
fn json_formatter_trend_with_git_context() {
    let trend_with_git = TrendDelta {
        files_delta: 3,
        lines_delta: 75,
        code_delta: 40,
        comment_delta: 20,
        blank_delta: 15,
        previous_timestamp: Some(1_700_000_000),
        previous_git_ref: Some("a1b2c3d".to_string()),
        previous_git_branch: Some("main".to_string()),
    };

    let stats = ProjectStatistics::new(vec![]).with_trend(trend_with_git);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let trend = parsed.get("trend").expect("trend should be present");

    // Verify git context fields are included in JSON output
    assert_eq!(
        trend.get("previous_commit").unwrap().as_str().unwrap(),
        "a1b2c3d",
        "previous_commit should contain the git ref"
    );
    assert_eq!(
        trend.get("previous_branch").unwrap().as_str().unwrap(),
        "main",
        "previous_branch should contain the branch name"
    );
}

#[test]
fn json_formatter_trend_git_context_omitted_when_none() {
    // When git context is None, the fields should be omitted (not null)
    let trend_without_git = TrendDelta {
        files_delta: 1,
        lines_delta: 10,
        code_delta: 5,
        comment_delta: 3,
        blank_delta: 2,
        previous_timestamp: Some(1_700_000_000),
        previous_git_ref: None,
        previous_git_branch: None,
    };

    let stats = ProjectStatistics::new(vec![]).with_trend(trend_without_git);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let trend = parsed.get("trend").expect("trend should be present");

    // Fields should be omitted entirely, not present as null
    assert!(
        trend.get("previous_commit").is_none(),
        "previous_commit should be omitted when git_ref is None"
    );
    assert!(
        trend.get("previous_branch").is_none(),
        "previous_branch should be omitted when git_branch is None"
    );
}

#[test]
fn json_formatter_trend_partial_git_context() {
    // Test with only commit (detached HEAD scenario - no branch)
    let trend_commit_only = TrendDelta {
        files_delta: 2,
        lines_delta: 20,
        code_delta: 10,
        comment_delta: 5,
        blank_delta: 5,
        previous_timestamp: Some(1_700_000_000),
        previous_git_ref: Some("deadbeef".to_string()),
        previous_git_branch: None,
    };

    let stats = ProjectStatistics::new(vec![]).with_trend(trend_commit_only);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let trend = parsed.get("trend").expect("trend should be present");

    assert_eq!(
        trend.get("previous_commit").unwrap().as_str().unwrap(),
        "deadbeef"
    );
    assert!(
        trend.get("previous_branch").is_none(),
        "previous_branch should be omitted for detached HEAD"
    );
}

// ============================================================================
// Files-only mode tests (stats files subcommand)
// ============================================================================

use crate::output::stats::FileSortOrder;

#[test]
fn json_formatter_files_only_mode_skips_summary() {
    let files = vec![
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("small.rs", 50, 30, 10, 10, "Rust"),
    ];

    // with_sorted_files creates files-only mode
    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Code, None);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Summary should be omitted in files-only mode
    assert!(
        parsed.get("summary").is_none(),
        "summary should be omitted in files-only mode"
    );

    // top_files should contain the sorted files
    assert!(parsed.get("top_files").is_some());
    let top_files = parsed.get("top_files").unwrap().as_array().unwrap();
    assert_eq!(top_files.len(), 2);

    // Files should be empty (cleared by with_sorted_files)
    assert!(
        parsed.get("files").is_none()
            || parsed.get("files").unwrap().as_array().unwrap().is_empty(),
        "files should be empty or omitted"
    );
}

#[test]
fn json_formatter_files_only_sorted_by_name() {
    let files = vec![
        file_stats("charlie.rs", 100, 80, 10, 10, "Rust"),
        file_stats("alpha.rs", 100, 50, 10, 40, "Rust"),
        file_stats("beta.rs", 100, 70, 10, 20, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Name, None);
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let top_files = parsed.get("top_files").unwrap().as_array().unwrap();

    // Verify sort order (by file name ascending)
    assert!(top_files[0]["path"].as_str().unwrap().contains("alpha"));
    assert!(top_files[1]["path"].as_str().unwrap().contains("beta"));
    assert!(top_files[2]["path"].as_str().unwrap().contains("charlie"));
}

#[test]
fn json_formatter_files_only_with_limit() {
    let files = vec![
        file_stats("a.rs", 200, 150, 30, 20, "Rust"),
        file_stats("b.rs", 100, 80, 10, 10, "Rust"),
        file_stats("c.rs", 50, 30, 10, 10, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Code, Some(2));
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let top_files = parsed.get("top_files").unwrap().as_array().unwrap();

    // Only top 2 files
    assert_eq!(top_files.len(), 2);
    assert!(top_files[0]["path"].as_str().unwrap().contains("a.rs"));
    assert!(top_files[1]["path"].as_str().unwrap().contains("b.rs"));
}

#[test]
fn json_formatter_summary_only_no_empty_files() {
    let files = vec![
        file_stats("a.rs", 100, 80, 10, 10, "Rust"),
        file_stats("b.rs", 50, 30, 10, 10, "Rust"),
    ];

    // with_summary_only clears files but doesn't set top_files
    let stats = ProjectStatistics::new(files).with_summary_only();
    let output = StatsJsonFormatter::new().format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Summary should be present
    assert!(parsed.get("summary").is_some());

    // files array should be omitted when empty (skip_serializing_if Vec::is_empty)
    // Note: Vec::is_empty skips serialization, so files key shouldn't be present
    let files_value = parsed.get("files");
    assert!(
        files_value.is_none() || files_value.unwrap().as_array().unwrap().is_empty(),
        "files should be omitted or empty for summary-only mode"
    );
}
