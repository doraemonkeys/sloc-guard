use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::ColorMode;
use crate::output::stats::{FileStatistics, ProjectStatistics, StatsFormatter, StatsTextFormatter};
use crate::stats::TrendDelta;

use super::{PROGRESS_EMPTY, PROGRESS_FILLED, render_progress_bar};

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
// Progress bar tests
// ============================================================================

#[test]
fn text_formatter_language_breakdown_shows_progress_bar() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("main.go", 50, 20, 20, 10, "Go"),
    ];

    let stats = ProjectStatistics::new(files).with_language_breakdown();
    let output = formatter().format(&stats).unwrap();

    // Progress bar characters should be present
    assert!(output.contains('█'), "Should contain filled bar chars");
    assert!(output.contains('░'), "Should contain empty bar chars");
    // Rust has 80% of code (80/100)
    assert!(output.contains("80.0%"), "Should show Rust percentage");
    // Go has 20% of code (20/100)
    assert!(output.contains("20.0%"), "Should show Go percentage");
    // Should show code counts
    assert!(output.contains("(80 code)"), "Should show Rust code count");
    assert!(output.contains("(20 code)"), "Should show Go code count");
}

#[test]
fn text_formatter_directory_breakdown_shows_progress_bar() {
    let files = vec![
        file_stats("src/main.rs", 100, 75, 15, 10, "Rust"),
        file_stats("tests/test.rs", 50, 25, 15, 10, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_directory_breakdown();
    let output = formatter().format(&stats).unwrap();

    // Progress bar characters should be present
    assert!(output.contains('█'));
    assert!(output.contains('░'));
    // src has 75% of code (75/100)
    assert!(output.contains("75.0%"));
    // tests has 25% of code (25/100)
    assert!(output.contains("25.0%"));
}

#[test]
fn text_formatter_progress_bar_with_zero_code() {
    let files = vec![file_stats("readme.md", 50, 0, 0, 50, "Markdown")];

    let stats = ProjectStatistics::new(files).with_language_breakdown();
    let output = formatter().format(&stats).unwrap();

    // Should handle zero code gracefully (no division by zero)
    assert!(output.contains("0.0%") || output.contains("░░░░░"));
}

// ============================================================================
// render_progress_bar unit tests
// ============================================================================

#[test]
fn render_progress_bar_zero_ratio_all_empty() {
    let bar = render_progress_bar(0.0, 10);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 0);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_EMPTY).count(), 10);
}

#[test]
fn render_progress_bar_full_ratio_all_filled() {
    let bar = render_progress_bar(1.0, 10);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 10);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_EMPTY).count(), 0);
}

#[test]
fn render_progress_bar_half_ratio() {
    let bar = render_progress_bar(0.5, 10);
    // 0.5 * 10 = 5.0, rounds to 5
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 5);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_EMPTY).count(), 5);
}

#[test]
fn render_progress_bar_negative_ratio_clamped_to_zero() {
    let bar = render_progress_bar(-0.5, 10);
    // Negative values should be clamped to 0
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 0);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_EMPTY).count(), 10);
}

#[test]
fn render_progress_bar_ratio_above_one_clamped() {
    let bar = render_progress_bar(1.5, 10);
    // Values > 1.0 should be clamped to 1.0
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 10);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_EMPTY).count(), 0);
}

#[test]
fn render_progress_bar_rounding_boundary() {
    // 0.44 * 10 = 4.4, rounds to 4
    let bar = render_progress_bar(0.44, 10);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 4);

    // 0.45 * 10 = 4.5, rounds to 5 (round half to even, but .round() uses away from zero)
    let bar = render_progress_bar(0.45, 10);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 5);

    // 0.46 * 10 = 4.6, rounds to 5
    let bar = render_progress_bar(0.46, 10);
    assert_eq!(bar.chars().filter(|&c| c == PROGRESS_FILLED).count(), 5);
}

#[test]
fn render_progress_bar_width_zero() {
    let bar = render_progress_bar(0.5, 0);
    assert!(bar.is_empty());
}

#[test]
fn render_progress_bar_preserves_total_width() {
    // Ensure the bar always has exactly the specified width
    for ratio in [0.0, 0.1, 0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1.0] {
        let bar = render_progress_bar(ratio, 25);
        let total_chars = bar
            .chars()
            .filter(|&c| c == PROGRESS_FILLED || c == PROGRESS_EMPTY)
            .count();
        assert_eq!(
            total_chars, 25,
            "Width should always be 25 for ratio {ratio}"
        );
    }
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

// ============================================================================
// SummaryOnly mode tests
// ============================================================================

#[test]
fn text_formatter_summary_only_shows_summary_section() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("lib.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_summary_only();
    let output = formatter().format(&stats).unwrap();

    // Summary section should be present
    assert!(output.contains("Summary:"));
    assert!(output.contains("Files: 2"));
    assert!(output.contains("Total lines: 150"));
    assert!(output.contains("Code: 120"));
    assert!(output.contains("Average code lines: 60.0"));
}

#[test]
fn text_formatter_summary_only_skips_file_details() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("lib.rs", 50, 40, 5, 5, "Rust"),
    ];

    let stats = ProjectStatistics::new(files).with_summary_only();
    let output = formatter().format(&stats).unwrap();

    // File details should NOT be present
    assert!(
        !output.contains("main.rs"),
        "File names should not appear in summary-only mode"
    );
    assert!(
        !output.contains("lib.rs"),
        "File names should not appear in summary-only mode"
    );
}

#[test]
fn text_formatter_summary_only_skips_top_files() {
    let files = vec![
        file_stats("large.rs", 200, 150, 30, 20, "Rust"),
        file_stats("small.rs", 50, 30, 10, 10, "Rust"),
    ];

    // Add top_files before calling with_summary_only (which clears it)
    let stats = ProjectStatistics::new(files)
        .with_top_files(5)
        .with_summary_only();
    let output = formatter().format(&stats).unwrap();

    // Top files section should NOT be present
    assert!(
        !output.contains("Largest Files"),
        "Top files section should not appear in summary-only mode"
    );
    assert!(
        !output.contains("large.rs"),
        "File names should not appear in summary-only mode"
    );
}

#[test]
fn text_formatter_summary_only_skips_language_breakdown() {
    let files = vec![
        file_stats("main.rs", 100, 80, 15, 5, "Rust"),
        file_stats("main.go", 50, 40, 5, 5, "Go"),
    ];

    let stats = ProjectStatistics::new(files)
        .with_language_breakdown()
        .with_summary_only();
    let output = formatter().format(&stats).unwrap();

    // Language breakdown should NOT be present
    assert!(
        !output.contains("By Language"),
        "Language breakdown should not appear in summary-only mode"
    );
    assert!(
        !output.contains("Rust ("),
        "Language names should not appear in summary-only mode"
    );
}

#[test]
fn text_formatter_summary_only_preserves_trend() {
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

    let stats = ProjectStatistics::new(files)
        .with_trend(trend)
        .with_summary_only();
    let output = formatter().format(&stats).unwrap();

    // Trend section should still be present
    assert!(
        output.contains("Changes from previous run"),
        "Trend should be preserved in summary-only mode"
    );
    assert!(output.contains("+1"));
}
