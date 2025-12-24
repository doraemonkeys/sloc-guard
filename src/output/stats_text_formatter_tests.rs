use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::ColorMode;
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

/// Create formatter without colors for predictable test output.
fn formatter() -> StatsTextFormatter {
    StatsTextFormatter::new(ColorMode::Never)
}

#[test]
fn text_formatter_empty() {
    let stats = ProjectStatistics::new(vec![]);
    let output = formatter().format(&stats).unwrap();

    assert!(output.contains("Summary:"));
    assert!(output.contains("Files: 0"));
    assert!(output.contains("Total lines: 0"));
}

#[test]
fn text_formatter_with_files() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];

    let stats = ProjectStatistics::new(files);
    let output = formatter().format(&stats).unwrap();

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
    let output = formatter().format(&stats).unwrap();

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
    let output = formatter().format(&stats).unwrap();

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
    let output = formatter().format(&stats).unwrap();

    assert!(output.contains("By Directory:"));
    assert!(output.contains("src (1 files):"));
    assert!(output.contains("tests (1 files):"));
    assert!(output.contains("Summary:"));
}

// ============================================================================
// Trend formatting tests
// ============================================================================

#[test]
fn text_formatter_with_trend_shows_arrows() {
    // Create stats with some current values so percentage can be calculated
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];
    let trend = TrendDelta {
        files_delta: 1,
        lines_delta: 10,
        code_delta: 8,
        comment_delta: 1,
        blank_delta: 1,
        previous_timestamp: None,
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let stats = ProjectStatistics::new(files).with_trend(trend);
    let output = formatter().format(&stats).unwrap();

    // Should show arrows for trend direction
    assert!(output.contains("↑")); // Positive delta
    assert!(output.contains("+1")); // Positive delta shown with +
    assert!(output.contains("Changes from previous run:"));
}

#[test]
fn text_formatter_with_negative_trend_shows_down_arrow() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];
    let trend = TrendDelta {
        files_delta: -1,
        lines_delta: -10,
        code_delta: -8,
        comment_delta: -1,
        blank_delta: -1,
        previous_timestamp: None,
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let stats = ProjectStatistics::new(files).with_trend(trend);
    let output = formatter().format(&stats).unwrap();

    // Should show down arrows for negative trend
    assert!(output.contains("↓"));
    assert!(output.contains("-1"));
}

#[test]
fn text_formatter_with_zero_trend_shows_tilde() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];
    let trend = TrendDelta {
        files_delta: 0,
        lines_delta: 0,
        code_delta: 0,
        comment_delta: 0,
        blank_delta: 0,
        previous_timestamp: None,
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let stats = ProjectStatistics::new(files).with_trend(trend);
    let output = formatter().format(&stats).unwrap();

    // Should show tilde for no change
    assert!(output.contains('~'));
    assert!(output.contains('0'));
}

#[test]
fn text_formatter_trend_shows_percentage() {
    // 10 files now, was 8 (delta=2), so +25%
    let files = vec![
        file_stats("a.rs", 100, 80, 15, 5, "Rust"),
        file_stats("b.rs", 100, 80, 15, 5, "Rust"),
        file_stats("c.rs", 100, 80, 15, 5, "Rust"),
        file_stats("d.rs", 100, 80, 15, 5, "Rust"),
        file_stats("e.rs", 100, 80, 15, 5, "Rust"),
        file_stats("f.rs", 100, 80, 15, 5, "Rust"),
        file_stats("g.rs", 100, 80, 15, 5, "Rust"),
        file_stats("h.rs", 100, 80, 15, 5, "Rust"),
        file_stats("i.rs", 100, 80, 15, 5, "Rust"),
        file_stats("j.rs", 100, 80, 15, 5, "Rust"),
    ];
    let trend = TrendDelta {
        files_delta: 2, // was 8, now 10 -> +25%
        lines_delta: 200,
        code_delta: 160,
        comment_delta: 30,
        blank_delta: 10,
        previous_timestamp: None,
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let stats = ProjectStatistics::new(files).with_trend(trend);
    let output = formatter().format(&stats).unwrap();

    // Should show percentage change
    assert!(output.contains("(+25.0%)"));
}

#[test]
fn text_formatter_trend_with_relative_time() {
    // Use a timestamp from 2 hours ago
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let two_hours_ago = now - 2 * 60 * 60;

    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];
    let trend = TrendDelta {
        files_delta: 1,
        lines_delta: 10,
        code_delta: 8,
        comment_delta: 1,
        blank_delta: 1,
        previous_timestamp: Some(two_hours_ago),
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let stats = ProjectStatistics::new(files).with_trend(trend);
    let output = formatter().format(&stats).unwrap();

    // Should show relative time in header
    assert!(output.contains("2 hours ago"));
}

#[test]
fn text_formatter_trend_with_colors() {
    let files = vec![file_stats("test.rs", 100, 80, 15, 5, "Rust")];
    let trend = TrendDelta {
        files_delta: 5,
        lines_delta: 50,
        code_delta: 40,
        comment_delta: 5,
        blank_delta: 5,
        previous_timestamp: None,
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let stats = ProjectStatistics::new(files).with_trend(trend);

    // With colors enabled
    let output = StatsTextFormatter::new(ColorMode::Always)
        .format(&stats)
        .unwrap();

    // Should contain ANSI escape codes
    assert!(output.contains("\x1b["));
}

#[test]
fn text_formatter_trend_percentage_from_zero_base() {
    // Edge case: current = delta means previous was 0, can't calculate percentage
    let files = vec![
        file_stats("a.rs", 50, 50, 0, 0, "Rust"),
        file_stats("b.rs", 50, 50, 0, 0, "Rust"),
        file_stats("c.rs", 50, 50, 0, 0, "Rust"),
        file_stats("d.rs", 50, 50, 0, 0, "Rust"),
        file_stats("e.rs", 50, 50, 0, 0, "Rust"),
    ];
    // 5 files with delta of 5 means previous was 0
    let trend = TrendDelta {
        files_delta: 5,
        lines_delta: 250,
        code_delta: 250,
        comment_delta: 0,
        blank_delta: 0,
        previous_timestamp: None,
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let stats = ProjectStatistics::new(files).with_trend(trend);
    let output = formatter().format(&stats).unwrap();

    // Should not show percentage when previous was 0 (can't divide by zero)
    assert!(
        !output.contains('%'),
        "Should not show percentage when previous was 0. Output: {output}"
    );
}
