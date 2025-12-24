//! Tests for `TrendHistory` basic operations: add, entries, save/load.

use std::path::Path;

use super::*;

#[test]
fn test_new() {
    let history = TrendHistory::new();

    assert!(history.is_empty());
    assert_eq!(history.len(), 0);
    assert_eq!(history.version(), 1);
}

#[test]
fn test_add() {
    let mut history = TrendHistory::new();
    let stats = sample_project_stats(5, 100);

    history.add(&stats);

    assert!(!history.is_empty());
    assert_eq!(history.len(), 1);
    assert!(history.latest().is_some());
}

#[test]
fn test_add_entry() {
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
fn test_compute_delta_empty() {
    let history = TrendHistory::new();
    let stats = sample_project_stats(5, 100);

    let delta = history.compute_delta(&stats);

    assert!(delta.is_none());
}

#[test]
fn test_compute_delta_with_entry() {
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
fn test_entries() {
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
fn test_save_and_load() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let history_path = temp_dir.path().join("history.json");

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
}

#[test]
fn test_load_or_default_nonexistent() {
    let history = TrendHistory::load_or_default(Path::new("nonexistent_file.json"));
    assert!(history.is_empty());
}

#[test]
fn test_load_nonexistent_returns_error() {
    let result = TrendHistory::load(Path::new("this_file_does_not_exist_12345.json"));
    assert!(result.is_err());
}

#[test]
fn test_load_or_default_invalid_json() {
    use std::io::Write;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let invalid_path = temp_dir.path().join("invalid_history.json");

    // Write invalid JSON to the file
    let mut file = std::fs::File::create(&invalid_path).unwrap();
    file.write_all(b"{ invalid json }").unwrap();
    drop(file);

    // load_or_default should return default (empty) history when file is invalid
    let history = TrendHistory::load_or_default(&invalid_path);
    assert!(history.is_empty());
}

#[test]
fn test_default_trait() {
    let history = TrendHistory::default();
    assert!(history.is_empty());
    assert_eq!(history.len(), 0);
    assert_eq!(history.version(), 1);
}

#[test]
fn test_equality() {
    let mut history1 = TrendHistory::new();
    let mut history2 = TrendHistory::new();

    assert_eq!(history1, history2);

    history1.add_entry(TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    });

    assert_ne!(history1, history2);

    history2.add_entry(TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    });

    assert_eq!(history1, history2);
}

#[test]
fn test_load_or_default_valid_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let history_path = temp_dir.path().join("history.json");

    // Create and save a valid history file
    let mut history = TrendHistory::new();
    history.add_entry(TrendEntry {
        timestamp: 5000,
        total_files: 20,
        total_lines: 2000,
        code: 1000,
        comment: 600,
        blank: 400,
    });
    history.save(&history_path).unwrap();

    // load_or_default should load the file successfully
    let loaded = TrendHistory::load_or_default(&history_path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.latest().unwrap().timestamp, 5000);
}
