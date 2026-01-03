use std::path::PathBuf;
use tempfile::TempDir;

use crate::EXIT_SUCCESS;
use crate::cli::{
    Cli, ColorChoice, Commands, ExtendsPolicy, HistoryArgs, HistoryOutputFormat, InitArgs,
    StatsAction, StatsArgs,
};
use crate::stats::TrendEntry;

use super::*;

// ============================================================================
// Test Helpers
// ============================================================================

fn make_cli_for_stats(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
            detect: false,
        }),
        verbose,
        quiet,
        color,
        no_config,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
    }
}

// ============================================================================
// History Formatting Tests
// ============================================================================

#[test]
fn test_format_history_text_empty() {
    let entries: Vec<&TrendEntry> = vec![];
    let output = format_history_text(&entries, 0);
    assert!(output.contains("No history entries found"));
    assert!(output.contains("sloc-guard snapshot"));
}

#[test]
fn test_format_history_text_with_entries() {
    let entry1 = TrendEntry {
        timestamp: 1_735_048_800, // 2024-12-24 14:00:00 UTC
        total_files: 100,
        total_lines: 5500,
        code: 5000,
        comment: 300,
        blank: 200,
        git_ref: Some("a1b2c3d".to_string()),
        git_branch: Some("main".to_string()),
    };
    let entry2 = TrendEntry {
        timestamp: 1_734_962_400, // 2024-12-23 14:00:00 UTC
        total_files: 98,
        total_lines: 5400,
        code: 4900,
        comment: 290,
        blank: 210,
        git_ref: Some("e4f5g6h".to_string()),
        git_branch: None,
    };

    let entries: Vec<&TrendEntry> = vec![&entry1, &entry2];
    let output = format_history_text(&entries, 5);

    assert!(output.contains("History (2 of 5 entries)"));
    assert!(output.contains("1. 2024-12-24"));
    assert!(output.contains("a1b2c3d (main)"));
    assert!(output.contains("Files: 100"));
    assert!(output.contains("Total: 5500"));
    assert!(output.contains("Code: 5000"));
    assert!(output.contains("2. 2024-12-23"));
    assert!(output.contains("e4f5g6h"));
}

#[test]
fn test_format_history_text_no_git_context() {
    let entry = TrendEntry {
        timestamp: 1_735_048_800,
        total_files: 50,
        total_lines: 2500,
        code: 2000,
        comment: 300,
        blank: 200,
        git_ref: None,
        git_branch: None,
    };

    let entries: Vec<&TrendEntry> = vec![&entry];
    let output = format_history_text(&entries, 1);

    assert!(output.contains("History (1 of 1 entries)"));
    assert!(output.contains("2024-12-24"));
    assert!(!output.contains(" - ")); // No git ref separator
}

#[test]
fn test_format_history_json_empty() {
    let entries: Vec<&TrendEntry> = vec![];
    let output = format_history_json(&entries).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["count"], 0);
    assert!(parsed["entries"].as_array().unwrap().is_empty());
}

#[test]
fn test_format_history_json_with_entries() {
    let entry = TrendEntry {
        timestamp: 1_735_048_800,
        total_files: 100,
        total_lines: 5500,
        code: 5000,
        comment: 300,
        blank: 200,
        git_ref: Some("a1b2c3d".to_string()),
        git_branch: Some("main".to_string()),
    };

    let entries: Vec<&TrendEntry> = vec![&entry];
    let output = format_history_json(&entries).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["count"], 1);

    let first_entry = &parsed["entries"][0];
    assert_eq!(first_entry["timestamp"], 1_735_048_800);
    assert_eq!(first_entry["total_files"], 100);
    assert_eq!(first_entry["code"], 5000);
    assert_eq!(first_entry["git_ref"], "a1b2c3d");
    assert_eq!(first_entry["git_branch"], "main");
}

// ============================================================================
// Timestamp Formatting Tests
// ============================================================================

#[test]
fn test_format_timestamp_basic() {
    // 2024-12-24 14:00:00 UTC = 1_735_048_800
    let formatted = format_timestamp(1_735_048_800);
    assert_eq!(formatted, "2024-12-24 14:00:00");
}

#[test]
fn test_format_timestamp_epoch() {
    // 1970-01-01 00:00:00 UTC
    let formatted = format_timestamp(0);
    assert_eq!(formatted, "1970-01-01 00:00:00");
}

#[test]
fn test_format_timestamp_leap_year() {
    // 2024-02-29 12:00:00 UTC (leap year)
    let formatted = format_timestamp(1_709_208_000);
    assert_eq!(formatted, "2024-02-29 12:00:00");
}

#[test]
fn test_days_to_ymd_epoch() {
    assert_eq!(days_to_ymd(0), (1970, 1, 1));
}

#[test]
fn test_days_to_ymd_leap_year() {
    // 2024-02-29 is day 19782 since epoch (2024 is leap year)
    assert_eq!(days_to_ymd(19782), (2024, 2, 29));
}

#[test]
fn test_days_to_ymd_end_of_year() {
    // 2023-12-31 is day 19722 since epoch
    assert_eq!(days_to_ymd(19722), (2023, 12, 31));
}

#[test]
fn test_is_leap_year_true() {
    assert!(is_leap_year(2024));
    assert!(is_leap_year(2000));
    assert!(is_leap_year(2020));
}

#[test]
fn test_is_leap_year_false() {
    assert!(!is_leap_year(2023));
    assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
    assert!(!is_leap_year(2100));
}

// ============================================================================
// History Subcommand Integration Tests
// ============================================================================

#[test]
fn run_stats_history_with_empty_history() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("empty-history.json");

    let args = StatsArgs {
        action: StatsAction::History(HistoryArgs {
            limit: 10,
            format: HistoryOutputFormat::Text,
            history_file: Some(history_path),
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_history_with_existing_history() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history.json");

    // Create a history file with some entries
    let history_content = r#"{
        "version": 1,
        "entries": [
            {"timestamp": 1735048800, "total_files": 100, "total_lines": 5500, "code": 5000, "comment": 300, "blank": 200}
        ]
    }"#;
    std::fs::write(&history_path, history_content).unwrap();

    let args = StatsArgs {
        action: StatsAction::History(HistoryArgs {
            limit: 10,
            format: HistoryOutputFormat::Text,
            history_file: Some(history_path),
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_history_with_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history.json");

    // Create a history file with entries
    let history_content = r#"{
        "version": 1,
        "entries": [
            {"timestamp": 1735048800, "total_files": 100, "total_lines": 5500, "code": 5000, "comment": 300, "blank": 200}
        ]
    }"#;
    std::fs::write(&history_path, history_content).unwrap();

    let args = StatsArgs {
        action: StatsAction::History(HistoryArgs {
            limit: 10,
            format: HistoryOutputFormat::Json,
            history_file: Some(history_path),
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_history_respects_limit() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history.json");

    // Create a history file with 5 entries
    let history_content = r#"{
        "version": 1,
        "entries": [
            {"timestamp": 1735000000, "total_files": 96, "total_lines": 5100, "code": 4600, "comment": 260, "blank": 180},
            {"timestamp": 1735010000, "total_files": 97, "total_lines": 5200, "code": 4700, "comment": 270, "blank": 190},
            {"timestamp": 1735020000, "total_files": 98, "total_lines": 5300, "code": 4800, "comment": 280, "blank": 195},
            {"timestamp": 1735030000, "total_files": 99, "total_lines": 5400, "code": 4900, "comment": 290, "blank": 198},
            {"timestamp": 1735040000, "total_files": 100, "total_lines": 5500, "code": 5000, "comment": 300, "blank": 200}
        ]
    }"#;
    std::fs::write(&history_path, history_content).unwrap();

    let args = StatsArgs {
        action: StatsAction::History(HistoryArgs {
            limit: 2,
            format: HistoryOutputFormat::Text,
            history_file: Some(history_path),
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}
