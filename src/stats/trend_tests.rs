use std::path::PathBuf;

use super::*;
use crate::counter::LineStats;
use crate::output::{FileStatistics, ProjectStatistics};

fn sample_project_stats(total_files: usize, total_code: usize) -> ProjectStatistics {
    let files: Vec<FileStatistics> = (0..total_files)
        .map(|i| FileStatistics {
            path: PathBuf::from(format!("file{i}.rs")),
            stats: LineStats {
                total: total_code / total_files + 20,
                code: total_code / total_files,
                comment: 10,
                blank: 10,
                ignored: 0,
            },
            language: "Rust".to_string(),
        })
        .collect();
    ProjectStatistics::new(files)
}

#[test]
fn test_trend_entry_new() {
    let stats = sample_project_stats(5, 100);
    let entry = TrendEntry::new(&stats);

    assert_eq!(entry.total_files, 5);
    assert_eq!(entry.code, 100);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_trend_entry_with_timestamp() {
    let stats = sample_project_stats(5, 100);
    let entry = TrendEntry::new(&stats).with_timestamp(12345);

    assert_eq!(entry.timestamp, 12345);
}

#[test]
fn test_trend_delta_compute_increase() {
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
fn test_trend_delta_compute_decrease() {
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
fn test_trend_delta_no_changes() {
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
fn test_trend_history_new() {
    let history = TrendHistory::new();

    assert!(history.is_empty());
    assert_eq!(history.len(), 0);
    assert_eq!(history.version(), 1);
}

#[test]
fn test_trend_history_add() {
    let mut history = TrendHistory::new();
    let stats = sample_project_stats(5, 100);

    history.add(&stats);

    assert!(!history.is_empty());
    assert_eq!(history.len(), 1);
    assert!(history.latest().is_some());
}

#[test]
fn test_trend_history_add_entry() {
    let mut history = TrendHistory::new();
    let entry = TrendEntry {
        timestamp: 12345,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    };

    history.add_entry(entry);

    assert_eq!(history.len(), 1);
    assert_eq!(history.latest().unwrap().timestamp, 12345);
}

#[test]
fn test_trend_history_compute_delta_empty() {
    let history = TrendHistory::new();
    let stats = sample_project_stats(5, 100);

    let delta = history.compute_delta(&stats);

    assert!(delta.is_none());
}

#[test]
fn test_trend_history_compute_delta_with_entry() {
    let mut history = TrendHistory::new();
    let entry = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    };
    history.add_entry(entry);

    let current = sample_project_stats(15, 600);
    let delta = history.compute_delta(&current);

    assert!(delta.is_some());
    let delta = delta.unwrap();
    assert_eq!(delta.files_delta, 5);
    assert_eq!(delta.code_delta, 100);
}

#[test]
fn test_trend_history_entries() {
    let mut history = TrendHistory::new();

    history.add_entry(TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    });
    history.add_entry(TrendEntry {
        timestamp: 2000,
        total_files: 15,
        total_lines: 150,
        code: 75,
        comment: 45,
        blank: 30,
    });

    let entries = history.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].timestamp, 1000);
    assert_eq!(entries[1].timestamp, 2000);
}

#[test]
fn test_trend_history_save_and_load() {
    let temp_dir = std::env::temp_dir();
    let history_path = temp_dir.join("test_sloc_guard_history.json");

    // Create and save history
    let mut history = TrendHistory::new();
    history.add_entry(TrendEntry {
        timestamp: 12345,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    });
    history.save(&history_path).unwrap();

    // Load and verify
    let loaded = TrendHistory::load(&history_path).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.latest().unwrap().timestamp, 12345);
    assert_eq!(loaded.latest().unwrap().code, 500);

    // Cleanup
    std::fs::remove_file(&history_path).ok();
}

#[test]
fn test_trend_history_load_or_default_nonexistent() {
    let history = TrendHistory::load_or_default(Path::new("nonexistent_file.json"));
    assert!(history.is_empty());
}

#[test]
fn test_trend_delta_default() {
    let delta = TrendDelta::default();

    assert_eq!(delta.files_delta, 0);
    assert_eq!(delta.lines_delta, 0);
    assert_eq!(delta.code_delta, 0);
    assert_eq!(delta.comment_delta, 0);
    assert_eq!(delta.blank_delta, 0);
    assert!(delta.previous_timestamp.is_none());
    assert!(!delta.has_changes());
}

#[test]
fn test_trend_history_load_nonexistent_returns_error() {
    let result = TrendHistory::load(Path::new("this_file_does_not_exist_12345.json"));
    assert!(result.is_err());
}

#[test]
fn test_trend_history_load_or_default_invalid_json() {
    use std::io::Write;

    let temp_dir = std::env::temp_dir();
    let invalid_path = temp_dir.join("test_invalid_history.json");

    // Write invalid JSON to the file
    let mut file = std::fs::File::create(&invalid_path).unwrap();
    file.write_all(b"{ invalid json }").unwrap();
    drop(file);

    // load_or_default should return default (empty) history when file is invalid
    let history = TrendHistory::load_or_default(&invalid_path);
    assert!(history.is_empty());

    // Cleanup
    std::fs::remove_file(&invalid_path).ok();
}
