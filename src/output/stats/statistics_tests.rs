use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::stats::{FileStatistics, ProjectStatistics};
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

// ============================================================================
// ProjectStatistics::new tests
// ============================================================================

#[test]
fn project_statistics_empty() {
    let stats = ProjectStatistics::new(vec![]);
    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.total_lines, 0);
    assert_eq!(stats.total_code, 0);
    assert_eq!(stats.total_comment, 0);
    assert_eq!(stats.total_blank, 0);
}

#[test]
fn project_statistics_single_file() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files);
    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.total_lines, 100);
    assert_eq!(stats.total_code, 80);
    assert_eq!(stats.total_comment, 15);
    assert_eq!(stats.total_blank, 5);
}

#[test]
fn project_statistics_multiple_files() {
    let files = vec![
        file_stats("a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("b.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files);
    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.total_lines, 150);
    assert_eq!(stats.total_code, 120);
    assert_eq!(stats.total_comment, 20);
    assert_eq!(stats.total_blank, 10);
}

// ============================================================================
// Language breakdown tests
// ============================================================================

#[test]
fn language_breakdown_single_language() {
    let files = vec![
        file_stats("a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("b.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_language_breakdown();
    let by_language = stats.by_language.unwrap();

    assert_eq!(by_language.len(), 1);
    assert_eq!(by_language[0].language, "Rust");
    assert_eq!(by_language[0].files, 2);
    assert_eq!(by_language[0].code, 120);
    assert_eq!(by_language[0].comment, 20);
    assert_eq!(by_language[0].blank, 10);
}

#[test]
fn language_breakdown_multiple_languages() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("main.go", 200, 150, 30, 20, "Go"),
        file_stats("lib.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_language_breakdown();
    let by_language = stats.by_language.unwrap();

    assert_eq!(by_language.len(), 2);
    // Sorted by code count descending, Go has more code
    assert_eq!(by_language[0].language, "Go");
    assert_eq!(by_language[0].files, 1);
    assert_eq!(by_language[0].code, 150);

    assert_eq!(by_language[1].language, "Rust");
    assert_eq!(by_language[1].files, 2);
    assert_eq!(by_language[1].code, 120);
}

// ============================================================================
// Directory breakdown tests
// ============================================================================

#[test]
fn directory_breakdown_single_directory() {
    let files = vec![
        file_stats("src/a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("src/b.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown();
    let by_directory = stats.by_directory.unwrap();

    assert_eq!(by_directory.len(), 1);
    assert_eq!(by_directory[0].directory, "src");
    assert_eq!(by_directory[0].files, 2);
    assert_eq!(by_directory[0].code, 120);
    assert_eq!(by_directory[0].comment, 20);
    assert_eq!(by_directory[0].blank, 10);
}

#[test]
fn directory_breakdown_multiple_directories() {
    let files = vec![
        file_stats("src/main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("tests/test.rs", 200, 150, 30, 20, "Rust"),
        file_stats("src/lib.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown();
    let by_directory = stats.by_directory.unwrap();

    assert_eq!(by_directory.len(), 2);
    // Sorted by code count descending, tests has more code
    assert_eq!(by_directory[0].directory, "tests");
    assert_eq!(by_directory[0].files, 1);
    assert_eq!(by_directory[0].code, 150);

    assert_eq!(by_directory[1].directory, "src");
    assert_eq!(by_directory[1].files, 2);
    assert_eq!(by_directory[1].code, 120);
}

// ============================================================================
// Top files tests
// ============================================================================

#[test]
fn with_top_files_sorts_by_code_lines() {
    let files = vec![
        file_stats("small.rs", 50, 30, 10, 10, "Rust"),
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("medium.rs", 100, 80, 15, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_top_files(2);
    let top_files = stats.top_files.unwrap();

    assert_eq!(top_files.len(), 2);
    assert_eq!(top_files[0].path, PathBuf::from("large.rs"));
    assert_eq!(top_files[0].stats.code, 150);
    assert_eq!(top_files[1].path, PathBuf::from("medium.rs"));
    assert_eq!(top_files[1].stats.code, 80);
}

#[test]
fn with_top_files_fewer_than_n() {
    let files = vec![file_stats("only.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files).with_top_files(5);
    let top_files = stats.top_files.unwrap();

    assert_eq!(top_files.len(), 1);
}

#[test]
fn with_top_files_computes_average() {
    let files = vec![
        file_stats("a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("b.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_top_files(10);

    assert!(stats.average_code_lines.is_some());
    let avg = stats.average_code_lines.unwrap();
    assert!((avg - 60.0).abs() < 0.001); // (80 + 40) / 2 = 60
}

#[test]
fn with_top_files_empty_has_no_average() {
    let stats = ProjectStatistics::new(vec![]).with_top_files(5);

    assert!(stats.average_code_lines.is_none());
    assert_eq!(stats.top_files.unwrap().len(), 0);
}

// ============================================================================
// Trend tests
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
fn project_statistics_with_trend() {
    let stats = ProjectStatistics::new(vec![]).with_trend(sample_trend_delta());
    assert!(stats.trend.is_some());
    let trend = stats.trend.unwrap();
    assert_eq!(trend.files_delta, 5);
    assert_eq!(trend.code_delta, 50);
}

// ============================================================================
// Summary-only tests
// ============================================================================

#[test]
fn with_summary_only_clears_files_and_breakdowns() {
    let files = vec![
        file_stats("a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("b.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files)
        .with_language_breakdown()
        .with_top_files(10)
        .with_summary_only();

    // Summary totals are preserved
    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.total_code, 120);
    assert_eq!(stats.total_comment, 20);
    assert_eq!(stats.total_blank, 10);

    // Average is computed
    assert!(stats.average_code_lines.is_some());
    let avg = stats.average_code_lines.unwrap();
    assert!((avg - 60.0).abs() < 0.001);

    // Detailed data is cleared
    assert!(stats.files.is_empty());
    assert!(stats.top_files.is_none());
    assert!(stats.by_language.is_none());
    assert!(stats.by_directory.is_none());
}

#[test]
fn with_summary_only_computes_average_if_not_set() {
    let files = vec![
        file_stats("a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("b.rs", 50, 40, 5, 5, "Rust"),
    ];

    // Without calling with_top_files (which normally computes average)
    let stats = ProjectStatistics::new(files).with_summary_only();

    // Average should be computed anyway
    assert!(stats.average_code_lines.is_some());
    let avg = stats.average_code_lines.unwrap();
    assert!((avg - 60.0).abs() < 0.001);
}

#[test]
fn with_summary_only_empty_project_no_average() {
    let stats = ProjectStatistics::new(vec![]).with_summary_only();

    assert_eq!(stats.total_files, 0);
    assert!(stats.average_code_lines.is_none()); // No division by zero
    assert!(stats.files.is_empty());
}

#[test]
fn with_summary_only_preserves_existing_average() {
    let files = vec![
        file_stats("a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("b.rs", 50, 40, 5, 5, "Rust"),
    ];

    // Compute average via with_top_files first
    let stats = ProjectStatistics::new(files).with_top_files(10);
    let original_avg = stats.average_code_lines;

    let stats = stats.with_summary_only();

    // Average should be preserved, not recomputed
    assert_eq!(stats.average_code_lines, original_avg);
}

#[test]
fn with_summary_only_preserves_trend() {
    let files = vec![file_stats("a.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files)
        .with_trend(sample_trend_delta())
        .with_summary_only();

    // Trend should be preserved
    assert!(stats.trend.is_some());
    assert_eq!(stats.trend.as_ref().unwrap().code_delta, 50);
}
