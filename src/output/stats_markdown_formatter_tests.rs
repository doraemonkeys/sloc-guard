use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::stats::{
    FileStatistics, ProjectStatistics, StatsFormatter, StatsMarkdownFormatter,
};
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
fn markdown_formatter_empty() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsMarkdownFormatter.format(&stats).unwrap();

    assert!(output.contains("## SLOC Statistics"));
    assert!(output.contains("### Summary"));
    assert!(output.contains("| Total Files | 0 |"));
}

#[test]
fn markdown_formatter_with_files() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files);
    let output = StatsMarkdownFormatter.format(&stats).unwrap();

    assert!(output.contains("| Total Files | 1 |"));
    assert!(output.contains("| Total Lines | 100 |"));
    assert!(output.contains("| Code | 80 |"));
    assert!(output.contains("| Comments | 15 |"));
    assert!(output.contains("| Blank | 5 |"));
}

#[test]
fn markdown_formatter_with_top_files() {
    let files = vec![
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("small.rs", 50, 30, 10, 10, "Go"),
    ];

    let stats = ProjectStatistics::new(files).with_top_files(5);
    let output = StatsMarkdownFormatter.format(&stats).unwrap();

    assert!(output.contains("### Top 2 Largest Files"));
    assert!(output.contains("| # | File | Language | Code |"));
    assert!(output.contains("| 1 | `large.rs` | Rust | 150 |"));
    assert!(output.contains("| 2 | `small.rs` | Go | 30 |"));
    assert!(output.contains("| Average Code Lines | 90.0 |"));
}

#[test]
fn markdown_formatter_with_language_breakdown() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("main.go", 50, 40, 5, 5, "Go"),
    ];

    let stats = ProjectStatistics::new(files).with_language_breakdown();
    let output = StatsMarkdownFormatter.format(&stats).unwrap();

    assert!(output.contains("### By Language"));
    assert!(output.contains("| Language | Files | Code | Comments | Blank |"));
    assert!(output.contains("| Rust | 1 | 80 | 15 | 5 |"));
    assert!(output.contains("| Go | 1 | 40 | 5 | 5 |"));
}

#[test]
fn markdown_formatter_with_directory_breakdown() {
    let files = vec![
        file_stats("src/main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("tests/test.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown();
    let output = StatsMarkdownFormatter.format(&stats).unwrap();

    assert!(output.contains("### By Directory"));
    assert!(output.contains("| Directory | Files | Code | Comments | Blank |"));
    assert!(output.contains("| `src` | 1 | 80 | 15 | 5 |"));
    assert!(output.contains("| `tests` | 1 | 40 | 5 | 5 |"));
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
fn markdown_formatter_with_trend() {
    let stats = ProjectStatistics::new(vec![]).with_trend(sample_trend_delta());
    let output = StatsMarkdownFormatter.format(&stats).unwrap();

    // Header contains "Changes" and trend metrics
    assert!(output.contains("### Changes"));
    assert!(output.contains("| Metric | Delta |"));
    assert!(output.contains("| Files | +5 |"));
    assert!(output.contains("| Code | +50 |"));
}

#[test]
fn markdown_formatter_without_trend() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsMarkdownFormatter.format(&stats).unwrap();

    assert!(!output.contains("### Changes"));
}
