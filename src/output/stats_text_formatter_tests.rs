use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::stats::{FileStatistics, ProjectStatistics, StatsFormatter, StatsTextFormatter};
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
fn text_formatter_empty() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("Summary:"));
    assert!(output.contains("Files: 0"));
    assert!(output.contains("Total lines: 0"));
}

#[test]
fn text_formatter_with_files() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files);
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("test.rs"));
    assert!(output.contains("100 lines"));
    assert!(output.contains("code=80"));
    assert!(output.contains("comment=15"));
    assert!(output.contains("blank=5"));
    assert!(output.contains("Files: 1"));
}

#[test]
fn text_formatter_with_language_breakdown() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("main.go", 50, 40, 5, 5, "Go"),
    ];

    let stats = ProjectStatistics::new(files).with_language_breakdown();
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("By Language:"));
    assert!(output.contains("Rust (1 files):"));
    assert!(output.contains("Go (1 files):"));
    assert!(output.contains("Summary:"));
}

#[test]
fn text_formatter_with_top_files() {
    let files = vec![
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("small.rs", 50, 30, 10, 10, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_top_files(5);
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("Top 2 Largest Files:"));
    assert!(output.contains("large.rs (150 lines)"));
    assert!(output.contains("Average code lines: 90.0"));
}

#[test]
fn text_formatter_with_directory_breakdown() {
    let files = vec![
        file_stats("src/main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("tests/test.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown();
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("By Directory:"));
    assert!(output.contains("src (1 files):"));
    assert!(output.contains("tests (1 files):"));
    assert!(output.contains("Summary:"));
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
fn text_formatter_with_trend() {
    let stats = ProjectStatistics::new(vec![]).with_trend(sample_trend_delta());
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("Changes from previous run:"));
    assert!(output.contains("Files: +5"));
    assert!(output.contains("Code: +50"));
}

#[test]
fn text_formatter_with_negative_trend() {
    let trend = TrendDelta {
        files_delta: -3,
        lines_delta: -50,
        code_delta: -30,
        comment_delta: -10,
        blank_delta: -10,
        previous_timestamp: Some(12345),
    };
    let stats = ProjectStatistics::new(vec![]).with_trend(trend);
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("Files: -3"));
    assert!(output.contains("Code: -30"));
}

#[test]
fn text_formatter_with_zero_trend() {
    let trend = TrendDelta {
        files_delta: 0,
        lines_delta: 0,
        code_delta: 0,
        comment_delta: 0,
        blank_delta: 0,
        previous_timestamp: Some(12345),
    };
    let stats = ProjectStatistics::new(vec![]).with_trend(trend);
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("Files: 0"));
    assert!(output.contains("Code: 0"));
}
