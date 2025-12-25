use std::path::PathBuf;
use tempfile::TempDir;

use crate::cli::{
    Cli, ColorChoice, Commands, GroupBy, HistoryArgs, HistoryOutputFormat, InitArgs, StatsArgs,
};
use crate::output::{ColorMode, OutputFormat, ProjectStatistics};
use crate::stats::TrendEntry;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::*;

#[test]
fn format_stats_output_text() {
    let stats = ProjectStatistics::new(vec![]);
    let output =
        format_stats_output(OutputFormat::Text, &stats, ColorMode::Never, None, None).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_stats_output_json() {
    let stats = ProjectStatistics::new(vec![]);
    let output =
        format_stats_output(OutputFormat::Json, &stats, ColorMode::Never, None, None).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_stats_output_sarif_not_implemented() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Sarif, &stats, ColorMode::Never, None, None);
    assert!(result.is_err());
}

#[test]
fn format_stats_output_markdown_works() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Markdown, &stats, ColorMode::Never, None, None);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Statistics"));
    assert!(output.contains("| Total Files | 0 |"));
}

#[test]
fn format_stats_output_html_works() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Html, &stats, ColorMode::Never, None, None);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("<!DOCTYPE html>"));
    assert!(output.contains("Total Files"));
}

// Integration tests moved from main_integration_tests.rs

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
        offline: false,
    }
}

#[test]
fn run_stats_impl_with_valid_directory() {
    let args = StatsArgs {
        action: None,
        paths: vec![PathBuf::from("src")],
        config: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec!["**/target/**".to_string()],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
        group_by: GroupBy::None,
        top: None,
        no_gitignore: false,
        trend: false,
        history_file: None,
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_stats_impl_with_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("stats.json");

    let args = StatsArgs {
        action: None,
        paths: vec![PathBuf::from("src")],
        config: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Json,
        output: Some(output_path.clone()),
        no_cache: true,
        group_by: GroupBy::None,
        top: None,
        no_gitignore: false,
        trend: false,
        history_file: None,
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, false, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("summary"));
}

#[test]
fn run_stats_impl_with_include_paths() {
    let args = StatsArgs {
        action: None,
        paths: vec![PathBuf::from(".")],
        config: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec!["src".to_string()],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
        group_by: GroupBy::None,
        top: None,
        no_gitignore: false,
        trend: false,
        history_file: None,
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_stats_returns_config_error_on_invalid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");
    std::fs::write(&config_path, "invalid toml [[[[").unwrap();

    let args = StatsArgs {
        action: None,
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
        group_by: GroupBy::None,
        top: None,
        no_gitignore: true,
        trend: false,
        history_file: None,
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, false);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_CONFIG_ERROR);
}

#[test]
fn run_stats_impl_with_markdown_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("stats.md");

    let args = StatsArgs {
        action: None,
        paths: vec![PathBuf::from("src")],
        config: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Markdown,
        output: Some(output_path.clone()),
        no_cache: true,
        group_by: GroupBy::None,
        top: None,
        no_gitignore: false,
        trend: false,
        history_file: None,
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, false, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("SLOC Statistics"));
}

#[test]
fn run_stats_impl_with_group_by_lang() {
    let args = StatsArgs {
        action: None,
        paths: vec![PathBuf::from("src")],
        config: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
        group_by: GroupBy::Lang,
        top: None,
        no_gitignore: false,
        trend: false,
        history_file: None,
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_stats_impl_with_top_files() {
    let args = StatsArgs {
        action: None,
        paths: vec![PathBuf::from("src")],
        config: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
        group_by: GroupBy::None,
        top: Some(5),
        no_gitignore: false,
        trend: false,
        history_file: None,
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_stats_impl_with_custom_history_file() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("custom-history.json");

    let args = StatsArgs {
        action: None,
        paths: vec![PathBuf::from("src")],
        config: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
        group_by: GroupBy::None,
        top: None,
        no_gitignore: false,
        trend: true,
        history_file: Some(history_path.clone()),
        since: None,
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(
        history_path.exists(),
        "Custom history file should be created"
    );

    // Verify the history file contains valid JSON
    let content = std::fs::read_to_string(&history_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.get("version").is_some());
    assert!(parsed.get("entries").is_some());
}

// ============================================================================
// History Subcommand Tests
// ============================================================================

#[test]
fn format_history_text_empty() {
    let entries: Vec<&TrendEntry> = vec![];
    let output = format_history_text(&entries, 0);
    assert!(output.contains("No history entries found"));
    assert!(output.contains("sloc-guard stats --trend"));
}

#[test]
fn format_history_text_with_entries() {
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
fn format_history_text_no_git_context() {
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
fn format_history_json_empty() {
    let entries: Vec<&TrendEntry> = vec![];
    let output = format_history_json(&entries).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["count"], 0);
    assert!(parsed["entries"].as_array().unwrap().is_empty());
}

#[test]
fn format_history_json_with_entries() {
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

#[test]
fn format_timestamp_basic() {
    // 2024-12-24 14:00:00 UTC = 1_735_048_800
    let formatted = format_timestamp(1_735_048_800);
    assert_eq!(formatted, "2024-12-24 14:00:00");
}

#[test]
fn format_timestamp_epoch() {
    // 1970-01-01 00:00:00 UTC
    let formatted = format_timestamp(0);
    assert_eq!(formatted, "1970-01-01 00:00:00");
}

#[test]
fn format_timestamp_leap_year() {
    // 2024-02-29 12:00:00 UTC (leap year)
    let formatted = format_timestamp(1_709_208_000);
    assert_eq!(formatted, "2024-02-29 12:00:00");
}

#[test]
fn days_to_ymd_epoch() {
    assert_eq!(days_to_ymd(0), (1970, 1, 1));
}

#[test]
fn days_to_ymd_leap_year() {
    // 2024-02-29 is day 19782 since epoch (2024 is leap year)
    assert_eq!(days_to_ymd(19782), (2024, 2, 29));
}

#[test]
fn days_to_ymd_end_of_year() {
    // 2023-12-31 is day 19722 since epoch
    assert_eq!(days_to_ymd(19722), (2023, 12, 31));
}

#[test]
fn is_leap_year_true() {
    assert!(is_leap_year(2024));
    assert!(is_leap_year(2000));
    assert!(is_leap_year(2020));
}

#[test]
fn is_leap_year_false() {
    assert!(!is_leap_year(2023));
    assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
    assert!(!is_leap_year(2100));
}

#[test]
fn run_history_with_empty_history() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("empty-history.json");

    let args = HistoryArgs {
        limit: 10,
        format: HistoryOutputFormat::Text,
        history_file: Some(history_path),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_history(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_history_with_existing_history() {
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

    let args = HistoryArgs {
        limit: 10,
        format: HistoryOutputFormat::Text,
        history_file: Some(history_path),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_history(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_history_with_json_format() {
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

    let args = HistoryArgs {
        limit: 10,
        format: HistoryOutputFormat::Json,
        history_file: Some(history_path),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_history(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_history_respects_limit() {
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

    let args = HistoryArgs {
        limit: 2,
        format: HistoryOutputFormat::Text,
        history_file: Some(history_path),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_history(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}
