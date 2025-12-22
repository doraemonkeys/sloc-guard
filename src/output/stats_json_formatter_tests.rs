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
    let output = StatsJsonFormatter.format(&stats).unwrap();

    assert!(output.contains("\"total_files\": 0"));
    assert!(output.contains("\"files\": []"));
}

#[test]
fn json_formatter_with_files() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files);
    let output = StatsJsonFormatter.format(&stats).unwrap();

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
    let output = StatsJsonFormatter.format(&stats).unwrap();

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
    let output = StatsJsonFormatter.format(&stats).unwrap();

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
    let output = StatsJsonFormatter.format(&stats).unwrap();

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
    let output = StatsJsonFormatter.format(&stats).unwrap();

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
    }
}

#[test]
fn json_formatter_with_trend() {
    let stats = ProjectStatistics::new(vec![]).with_trend(sample_trend_delta());
    let output = StatsJsonFormatter.format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("trend").is_some());
    let trend = parsed.get("trend").unwrap();
    assert_eq!(trend.get("files").unwrap().as_i64().unwrap(), 5);
    assert_eq!(trend.get("code").unwrap().as_i64().unwrap(), 50);
}

#[test]
fn json_formatter_without_trend() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsJsonFormatter.format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("trend").is_none());
}
