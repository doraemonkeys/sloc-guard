use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::stats::{FileStatistics, ProjectStatistics};
use crate::stats::TrendDelta;

use super::truncate_path_to_depth;

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
// Directory breakdown depth tests
// ============================================================================

#[test]
fn directory_breakdown_depth_1_groups_top_level() {
    let files = vec![
        file_stats("src/commands/check.rs", 100, 80, 15, 5, "Rust"),
        file_stats("src/commands/stats.rs", 100, 80, 15, 5, "Rust"),
        file_stats("src/output/text.rs", 50, 40, 5, 5, "Rust"),
        file_stats("tests/integration/test.rs", 75, 60, 10, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown_depth(None, Some(1));
    let by_directory = stats.by_directory.unwrap();

    assert_eq!(by_directory.len(), 2);

    // Find src entry (should have all 3 src/* files)
    let src = by_directory.iter().find(|d| d.directory == "src").unwrap();
    assert_eq!(src.files, 3);
    assert_eq!(src.code, 200); // 80 + 80 + 40

    // Find tests entry (should have 1 file)
    let tests = by_directory
        .iter()
        .find(|d| d.directory == "tests")
        .unwrap();
    assert_eq!(tests.files, 1);
    assert_eq!(tests.code, 60);
}

#[test]
fn directory_breakdown_depth_2_shows_two_levels() {
    let files = vec![
        file_stats("src/commands/check.rs", 100, 80, 15, 5, "Rust"),
        file_stats("src/commands/stats.rs", 100, 70, 20, 10, "Rust"),
        file_stats("src/output/text.rs", 50, 40, 5, 5, "Rust"),
        file_stats("src/output/json.rs", 60, 50, 5, 5, "Rust"),
        file_stats("tests/integration/test.rs", 75, 60, 10, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown_depth(None, Some(2));
    let by_directory = stats.by_directory.unwrap();

    assert_eq!(by_directory.len(), 3); // src/commands, src/output, tests/integration

    let commands = by_directory
        .iter()
        .find(|d| d.directory == "src/commands")
        .unwrap();
    assert_eq!(commands.files, 2);
    assert_eq!(commands.code, 150); // 80 + 70

    let output = by_directory
        .iter()
        .find(|d| d.directory == "src/output")
        .unwrap();
    assert_eq!(output.files, 2);
    assert_eq!(output.code, 90); // 40 + 50
}

#[test]
fn directory_breakdown_depth_none_shows_full_path() {
    let files = vec![
        file_stats("src/commands/check/runner.rs", 100, 80, 15, 5, "Rust"),
        file_stats("src/commands/check/tests.rs", 50, 40, 5, 5, "Rust"),
    ];

    // Without depth limiting
    let stats = ProjectStatistics::new(files).with_directory_breakdown_depth(None, None);
    let by_directory = stats.by_directory.unwrap();

    assert_eq!(by_directory.len(), 1);
    assert_eq!(by_directory[0].directory, "src/commands/check");
}

#[test]
fn directory_breakdown_depth_handles_root_files() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("lib.rs", 50, 40, 5, 5, "Rust"),
        file_stats("src/mod.rs", 30, 25, 3, 2, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown_depth(None, Some(1));
    let by_directory = stats.by_directory.unwrap();

    // Root files grouped as "."
    let root = by_directory.iter().find(|d| d.directory == ".").unwrap();
    assert_eq!(root.files, 2);
    assert_eq!(root.code, 120); // 80 + 40

    // src files grouped separately
    let src = by_directory.iter().find(|d| d.directory == "src").unwrap();
    assert_eq!(src.files, 1);
    assert_eq!(src.code, 25);
}

#[test]
fn directory_breakdown_depth_zero_shows_full_path() {
    // depth=0 should behave like no depth limit (show full paths)
    let files = vec![file_stats("src/commands/check.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files).with_directory_breakdown_depth(None, Some(0));
    let by_directory = stats.by_directory.unwrap();

    assert_eq!(by_directory[0].directory, "src/commands");
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

// ============================================================================
// Sorted files tests (stats files subcommand)
// ============================================================================

use crate::output::stats::FileSortOrder;

#[test]
fn with_sorted_files_default_code_order() {
    let files = vec![
        file_stats("small.rs", 50, 30, 10, 10, "Rust"),
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("medium.rs", 100, 80, 15, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Code, None);
    let sorted = stats.top_files.unwrap();

    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].path, PathBuf::from("large.rs"));
    assert_eq!(sorted[1].path, PathBuf::from("medium.rs"));
    assert_eq!(sorted[2].path, PathBuf::from("small.rs"));

    // Files should be cleared (files-only mode)
    assert!(stats.files.is_empty());
}

#[test]
fn with_sorted_files_total_order() {
    let files = vec![
        file_stats("a.rs", 100, 50, 30, 20, "Rust"), // high total, low code
        file_stats("b.rs", 50, 40, 5, 5, "Rust"),    // low total, high code
        file_stats("c.rs", 150, 60, 50, 40, "Rust"), // highest total
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Total, None);
    let sorted = stats.top_files.unwrap();

    assert_eq!(sorted[0].path, PathBuf::from("c.rs")); // 150 total
    assert_eq!(sorted[1].path, PathBuf::from("a.rs")); // 100 total
    assert_eq!(sorted[2].path, PathBuf::from("b.rs")); // 50 total
}

#[test]
fn with_sorted_files_comment_order() {
    let files = vec![
        file_stats("a.rs", 100, 80, 10, 10, "Rust"), // 10 comments
        file_stats("b.rs", 100, 50, 40, 10, "Rust"), // 40 comments
        file_stats("c.rs", 100, 70, 25, 5, "Rust"),  // 25 comments
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Comment, None);
    let sorted = stats.top_files.unwrap();

    assert_eq!(sorted[0].path, PathBuf::from("b.rs")); // 40 comments
    assert_eq!(sorted[1].path, PathBuf::from("c.rs")); // 25 comments
    assert_eq!(sorted[2].path, PathBuf::from("a.rs")); // 10 comments
}

#[test]
fn with_sorted_files_blank_order() {
    let files = vec![
        file_stats("a.rs", 100, 80, 10, 10, "Rust"), // 10 blanks
        file_stats("b.rs", 100, 50, 10, 40, "Rust"), // 40 blanks
        file_stats("c.rs", 100, 70, 10, 20, "Rust"), // 20 blanks
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Blank, None);
    let sorted = stats.top_files.unwrap();

    assert_eq!(sorted[0].path, PathBuf::from("b.rs")); // 40 blanks
    assert_eq!(sorted[1].path, PathBuf::from("c.rs")); // 20 blanks
    assert_eq!(sorted[2].path, PathBuf::from("a.rs")); // 10 blanks
}

#[test]
fn with_sorted_files_name_order() {
    let files = vec![
        file_stats("src/charlie.rs", 100, 80, 10, 10, "Rust"),
        file_stats("src/alpha.rs", 100, 50, 10, 40, "Rust"),
        file_stats("tests/beta.rs", 100, 70, 10, 20, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Name, None);
    let sorted = stats.top_files.unwrap();

    // Sorted by file name only (not full path)
    assert_eq!(sorted[0].path, PathBuf::from("src/alpha.rs"));
    assert_eq!(sorted[1].path, PathBuf::from("tests/beta.rs"));
    assert_eq!(sorted[2].path, PathBuf::from("src/charlie.rs"));
}

#[test]
fn with_sorted_files_with_limit() {
    let files = vec![
        file_stats("small.rs", 50, 30, 10, 10, "Rust"),
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("medium.rs", 100, 80, 15, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Code, Some(2));
    let sorted = stats.top_files.unwrap();

    assert_eq!(sorted.len(), 2);
    assert_eq!(sorted[0].path, PathBuf::from("large.rs"));
    assert_eq!(sorted[1].path, PathBuf::from("medium.rs"));
}

#[test]
fn with_sorted_files_limit_exceeds_count() {
    let files = vec![
        file_stats("a.rs", 100, 80, 10, 10, "Rust"),
        file_stats("b.rs", 50, 30, 10, 10, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_sorted_files(FileSortOrder::Code, Some(10));
    let sorted = stats.top_files.unwrap();

    assert_eq!(sorted.len(), 2); // Only 2 files exist
}

#[test]
fn with_sorted_files_empty() {
    let stats = ProjectStatistics::new(vec![]).with_sorted_files(FileSortOrder::Code, None);
    let sorted = stats.top_files.unwrap();

    assert!(sorted.is_empty());
    assert!(stats.files.is_empty());
}

// ============================================================================
// truncate_path_to_depth unit tests
// ============================================================================

#[test]
fn truncate_path_to_depth_dot_returns_unchanged() {
    // Special case: "." always returns unchanged regardless of depth
    assert_eq!(truncate_path_to_depth(".", 0), ".");
    assert_eq!(truncate_path_to_depth(".", 1), ".");
    assert_eq!(truncate_path_to_depth(".", 5), ".");
}

#[test]
fn truncate_path_to_depth_zero_returns_unchanged() {
    // depth=0 means no truncation (show full path)
    assert_eq!(
        truncate_path_to_depth("src/commands/check", 0),
        "src/commands/check"
    );
    assert_eq!(truncate_path_to_depth("a/b/c/d/e", 0), "a/b/c/d/e");
}

#[test]
fn truncate_path_to_depth_one_returns_first_component() {
    assert_eq!(truncate_path_to_depth("src/commands/check", 1), "src");
    assert_eq!(truncate_path_to_depth("a/b/c", 1), "a");
}

#[test]
fn truncate_path_to_depth_two_returns_two_components() {
    assert_eq!(
        truncate_path_to_depth("src/commands/check", 2),
        "src/commands"
    );
    assert_eq!(truncate_path_to_depth("a/b/c/d", 2), "a/b");
}

#[test]
fn truncate_path_to_depth_exceeds_path_components() {
    // When depth exceeds actual components, return full path
    assert_eq!(truncate_path_to_depth("src/commands", 5), "src/commands");
    assert_eq!(truncate_path_to_depth("single", 3), "single");
}

#[test]
fn truncate_path_to_depth_single_component() {
    // Single component path with various depths
    assert_eq!(truncate_path_to_depth("src", 1), "src");
    assert_eq!(truncate_path_to_depth("src", 2), "src");
}

#[test]
fn truncate_path_to_depth_exact_match() {
    // Depth exactly matches number of components
    assert_eq!(truncate_path_to_depth("a/b/c", 3), "a/b/c");
}
