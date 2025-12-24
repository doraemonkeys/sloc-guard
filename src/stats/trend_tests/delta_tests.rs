//! Tests for `TrendDelta` computation and basic behavior.

use std::path::PathBuf;

use super::*;

#[test]
fn test_compute_increase() {
    let prev = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    };
    let current = sample_project_stats(15, 600);

    let delta = TrendDelta::compute(&prev, &current);

    assert_eq!(delta.files_delta, 5);
    assert_eq!(delta.code_delta, 100);
    assert!(delta.has_changes());
    assert_eq!(delta.previous_timestamp, Some(1000));
}

#[test]
fn test_compute_decrease() {
    let prev = TrendEntry {
        timestamp: 1000,
        total_files: 20,
        total_lines: 2000,
        code: 1000,
        comment: 600,
        blank: 400,
    };
    let current = sample_project_stats(10, 500);

    let delta = TrendDelta::compute(&prev, &current);

    assert_eq!(delta.files_delta, -10);
    assert_eq!(delta.code_delta, -500);
    assert!(delta.has_changes());
}

#[test]
fn test_no_changes() {
    let prev = TrendEntry {
        timestamp: 1000,
        total_files: 5,
        total_lines: 150,
        code: 100,
        comment: 50,
        blank: 50,
    };

    // Create stats with same totals
    let files = vec![FileStatistics {
        path: PathBuf::from("file.rs"),
        stats: LineStats {
            total: 150,
            code: 100,
            comment: 50,
            blank: 50,
            ignored: 0,
        },
        language: "Rust".to_string(),
    }];

    // Manually create stats with 5 files
    let mut stats = ProjectStatistics::new(files);
    // Override to match previous
    stats.total_files = 5;
    stats.total_lines = 150;
    stats.total_code = 100;
    stats.total_comment = 50;
    stats.total_blank = 50;

    let delta = TrendDelta::compute(&prev, &stats);

    assert!(!delta.has_changes());
}

#[test]
fn test_default() {
    let delta = TrendDelta::default();

    assert_eq!(delta.files_delta, 0);
    assert_eq!(delta.lines_delta, 0);
    assert_eq!(delta.code_delta, 0);
    assert_eq!(delta.comment_delta, 0);
    assert_eq!(delta.blank_delta, 0);
    assert!(delta.previous_timestamp.is_none());
    assert!(!delta.has_changes());
}
