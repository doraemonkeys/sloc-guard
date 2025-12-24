//! Tests for `TrendEntry` creation, modification, and equality.

use super::*;

#[test]
fn test_new() {
    let stats = sample_project_stats(5, 100);
    let entry = TrendEntry::new(&stats);

    assert_eq!(entry.total_files, 5);
    assert_eq!(entry.code, 100);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_with_timestamp() {
    let stats = sample_project_stats(5, 100);
    let entry = TrendEntry::new(&stats).with_timestamp(12345);

    assert_eq!(entry.timestamp, 12345);
}

#[test]
fn test_equality() {
    let entry1 = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    };
    let entry2 = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    };
    let entry3 = TrendEntry {
        timestamp: 2000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    };

    assert_eq!(entry1, entry2);
    assert_ne!(entry1, entry3);
}
