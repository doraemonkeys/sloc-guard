//! Tests for time-based queries: `find_entry_at_or_before`, `compute_delta_since`.

use super::*;

// ============================================================================
// find_entry_at_or_before Tests
// ============================================================================

#[test]
fn test_find_entry_at_or_before_empty_history() {
    let history = TrendHistory::new();
    assert!(history.find_entry_at_or_before(1000).is_none());
}

#[test]
fn test_find_entry_at_or_before_single_entry_before() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(500));

    let found = history.find_entry_at_or_before(1000);
    assert!(found.is_some());
    assert_eq!(found.unwrap().timestamp, 500);
}

#[test]
fn test_find_entry_at_or_before_single_entry_after() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(1500));

    let found = history.find_entry_at_or_before(1000);
    assert!(found.is_none());
}

#[test]
fn test_find_entry_at_or_before_single_entry_exact() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(1000));

    // Should find entry exactly AT the timestamp
    let found = history.find_entry_at_or_before(1000);
    assert!(found.is_some());
    assert_eq!(found.unwrap().timestamp, 1000);
}

#[test]
fn test_find_entry_at_or_before_multiple_entries_finds_nearest() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(100));
    history.add_entry(make_entry(500));
    history.add_entry(make_entry(800));
    history.add_entry(make_entry(1200));

    // Looking for entry at or before 1000, should find 800 (nearest <= 1000)
    let found = history.find_entry_at_or_before(1000);
    assert!(found.is_some());
    assert_eq!(found.unwrap().timestamp, 800);
}

#[test]
fn test_find_entry_at_or_before_all_entries_after() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(2000));
    history.add_entry(make_entry(3000));
    history.add_entry(make_entry(4000));

    let found = history.find_entry_at_or_before(1000);
    assert!(found.is_none());
}

#[test]
fn test_find_entry_at_or_before_all_entries_before() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(100));
    history.add_entry(make_entry(200));
    history.add_entry(make_entry(300));

    // Should find the nearest (300)
    let found = history.find_entry_at_or_before(1000);
    assert!(found.is_some());
    assert_eq!(found.unwrap().timestamp, 300);
}

// ============================================================================
// compute_delta_since Tests
// ============================================================================

#[test]
fn test_compute_delta_since_empty_history() {
    let history = TrendHistory::new();
    let stats = sample_project_stats(10, 500);

    let delta = history.compute_delta_since(86400, &stats, 100_000_000);
    assert!(delta.is_none());
}

#[test]
fn test_compute_delta_since_finds_entry() {
    let mut history = TrendHistory::new();
    let current_time = 100_000_000u64;

    // Add entry 10 days ago - this is BEFORE the 7-day target
    history.add_entry(TrendEntry {
        timestamp: current_time - 10 * SECONDS_PER_DAY,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
        git_ref: None,
        git_branch: None,
    });

    let current = sample_project_stats(15, 600);

    // Look back 7 days - should find the 10-day-old entry (nearest at or before 7 days ago)
    let delta = history.compute_delta_since(7 * SECONDS_PER_DAY, &current, current_time);

    assert!(delta.is_some());
    let delta = delta.unwrap();
    assert_eq!(delta.files_delta, 5);
    assert_eq!(delta.code_delta, 100);
}

#[test]
fn test_compute_delta_since_no_entry_old_enough() {
    let mut history = TrendHistory::new();
    let current_time = 100_000_000u64;

    // Add entry only 2 days ago - too recent for 7-day lookback
    history.add_entry(make_entry(current_time - 2 * SECONDS_PER_DAY));

    let current = sample_project_stats(15, 600);

    // Look back 7 days - no entry exists at or before 7 days ago
    let delta = history.compute_delta_since(7 * SECONDS_PER_DAY, &current, current_time);

    assert!(delta.is_none());
}

#[test]
fn test_compute_delta_since_picks_nearest_entry() {
    let mut history = TrendHistory::new();
    let current_time = 100_000_000u64;

    // Add entries at 30, 10, and 5 days ago
    history.add_entry(TrendEntry {
        timestamp: current_time - 30 * SECONDS_PER_DAY,
        total_files: 5,
        total_lines: 500,
        code: 250,
        comment: 150,
        blank: 100,
        git_ref: None,
        git_branch: None,
    });
    history.add_entry(TrendEntry {
        timestamp: current_time - 10 * SECONDS_PER_DAY,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
        git_ref: None,
        git_branch: None,
    });
    history.add_entry(TrendEntry {
        timestamp: current_time - 5 * SECONDS_PER_DAY,
        total_files: 12,
        total_lines: 1200,
        code: 600,
        comment: 360,
        blank: 240,
        git_ref: None,
        git_branch: None,
    });

    // 15 files with 750 / 15 = 50 code lines each = 750 total code
    let current = sample_project_stats(15, 750);

    // Look back 7 days - should find the 10-day-old entry (nearest at or before 7 days ago)
    let delta = history.compute_delta_since(7 * SECONDS_PER_DAY, &current, current_time);

    assert!(delta.is_some());
    let delta = delta.unwrap();
    // Comparing against 10-day-old entry (code: 500)
    assert_eq!(delta.files_delta, 5); // 15 - 10
    assert_eq!(delta.code_delta, 250); // 750 - 500
}

#[test]
fn test_compute_delta_since_exact_boundary() {
    let mut history = TrendHistory::new();
    let current_time = 100_000_000u64;

    // Add entry exactly 7 days ago
    history.add_entry(TrendEntry {
        timestamp: current_time - 7 * SECONDS_PER_DAY,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
        git_ref: None,
        git_branch: None,
    });

    let current = sample_project_stats(15, 600);

    // Look back 7 days - entry is at exact boundary, should be included
    let delta = history.compute_delta_since(7 * SECONDS_PER_DAY, &current, current_time);

    assert!(delta.is_some());
}
