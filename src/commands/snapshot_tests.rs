//! Tests for the snapshot command.

use super::*;
use crate::counter::LineStats;
use tempfile::TempDir;

fn create_test_stats() -> ProjectStatistics {
    let file_stats = vec![FileStatistics {
        path: std::path::PathBuf::from("test.rs"),
        stats: LineStats {
            total: 100,
            code: 80,
            comment: 15,
            blank: 5,
            ignored: 0,
        },
        language: "rust".to_string(),
    }];
    ProjectStatistics::new(file_stats)
}

#[test]
fn test_print_dry_run_output_would_add() {
    let stats = create_test_stats();

    // This just verifies the function runs without panicking
    print_dry_run_output(&stats, None, true);
}

#[test]
fn test_print_dry_run_output_would_skip() {
    let stats = create_test_stats();

    print_dry_run_output(&stats, None, false);
}

#[test]
fn test_print_dry_run_output_with_git_context() {
    let stats = create_test_stats();
    let git_context = GitContext {
        commit: "abc1234".to_string(),
        branch: Some("main".to_string()),
    };

    print_dry_run_output(&stats, Some(&git_context), true);
}

#[test]
fn test_print_snapshot_summary() {
    let stats = create_test_stats();
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history.json");

    // This just verifies the function runs without panicking
    print_snapshot_summary(&stats, None, &history_path);
}

#[test]
fn test_print_snapshot_summary_with_git_context() {
    let stats = create_test_stats();
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history.json");
    let git_context = GitContext {
        commit: "def5678".to_string(),
        branch: Some("feature".to_string()),
    };

    print_snapshot_summary(&stats, Some(&git_context), &history_path);
}

