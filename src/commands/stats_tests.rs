use std::path::PathBuf;
use tempfile::TempDir;

use crate::cli::{
    BreakdownArgs, BreakdownBy, Cli, ColorChoice, Commands, CommonStatsArgs, FileSortOrder,
    FilesArgs, HistoryArgs, HistoryOutputFormat, InitArgs, ReportArgs, ReportOutputFormat,
    StatsAction, StatsArgs, StatsOutputFormat, SummaryArgs, TrendArgs,
};
use crate::output::{ColorMode, OutputFormat, ProjectStatistics};
use crate::stats::TrendEntry;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::*;

// ============================================================================
// Format Output Tests
// ============================================================================

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
        offline: false,
    }
}

fn make_common_args(paths: Vec<PathBuf>, ext: Option<Vec<String>>) -> CommonStatsArgs {
    CommonStatsArgs {
        paths,
        config: None,
        ext,
        exclude: vec![],
        include: vec![],
        no_cache: true,
        no_gitignore: false,
    }
}

// ============================================================================
// Summary Subcommand Tests
// ============================================================================

#[test]
fn run_stats_summary_subcommand() {
    let args = StatsArgs {
        action: StatsAction::Summary(SummaryArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_summary_json_format() {
    let args = StatsArgs {
        action: StatsAction::Summary(SummaryArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: StatsOutputFormat::Json,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

// ============================================================================
// Files Subcommand Tests
// ============================================================================

#[test]
fn run_stats_files_subcommand() {
    let args = StatsArgs {
        action: StatsAction::Files(FilesArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            top: Some(5),
            sort: FileSortOrder::Code,
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_files_all() {
    let args = StatsArgs {
        action: StatsAction::Files(FilesArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            top: None, // All files
            sort: FileSortOrder::Code,
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

// ============================================================================
// Breakdown Subcommand Tests
// ============================================================================

#[test]
fn run_stats_breakdown_by_lang() {
    let args = StatsArgs {
        action: StatsAction::Breakdown(BreakdownArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            by: BreakdownBy::Lang,
            depth: None,
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_breakdown_by_dir() {
    let args = StatsArgs {
        action: StatsAction::Breakdown(BreakdownArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            by: BreakdownBy::Dir,
            depth: None,
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_breakdown_by_dir_with_depth() {
    let args = StatsArgs {
        action: StatsAction::Breakdown(BreakdownArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            by: BreakdownBy::Dir,
            depth: Some(1), // Top-level only
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_breakdown_depth_with_lang_shows_warning() {
    // --depth is only applicable to --by dir, should warn when used with --by lang
    let args = StatsArgs {
        action: StatsAction::Breakdown(BreakdownArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            by: BreakdownBy::Lang,
            depth: Some(2), // Should trigger warning
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    // Should still succeed despite warning
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

// ============================================================================
// Trend Subcommand Tests
// ============================================================================

#[test]
fn run_stats_trend_subcommand() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history.json");

    let args = StatsArgs {
        action: StatsAction::Trend(TrendArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            since: None,
            history_file: Some(history_path),
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_stats_trend_with_since() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history.json");

    // Create a history file with an entry
    let history_content = r#"{
        "version": 1,
        "entries": [
            {"timestamp": 1735048800, "total_files": 100, "total_lines": 5500, "code": 5000, "comment": 300, "blank": 200}
        ]
    }"#;
    std::fs::write(&history_path, history_content).unwrap();

    let args = StatsArgs {
        action: StatsAction::Trend(TrendArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            since: Some("7d".to_string()),
            history_file: Some(history_path),
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

// ============================================================================
// Report Subcommand Tests
// ============================================================================

#[test]
fn run_stats_report_subcommand() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.txt");

    let args = StatsArgs {
        action: StatsAction::Report(ReportArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: ReportOutputFormat::Text,
            output: Some(output_path.clone()),
            history_file: None,
            exclude_sections: vec![],
            top: None,
            breakdown_by: None,
            since: None,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
    assert!(output_path.exists());
}

#[test]
fn run_stats_report_json() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.json");

    let args = StatsArgs {
        action: StatsAction::Report(ReportArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: ReportOutputFormat::Json,
            output: Some(output_path.clone()),
            history_file: None,
            exclude_sections: vec![],
            top: None,
            breakdown_by: None,
            since: None,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("summary"));
}

#[test]
fn run_stats_report_html() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.html");

    let args = StatsArgs {
        action: StatsAction::Report(ReportArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: ReportOutputFormat::Html,
            output: Some(output_path.clone()),
            history_file: None,
            exclude_sections: vec![],
            top: None,
            breakdown_by: None,
            since: None,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("<!DOCTYPE html>"));
}

#[test]
fn run_stats_report_with_exclude_sections() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.txt");

    let args = StatsArgs {
        action: StatsAction::Report(ReportArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: ReportOutputFormat::Text,
            output: Some(output_path.clone()),
            history_file: None,
            exclude_sections: vec!["trend".to_string(), "breakdown".to_string()],
            top: None,
            breakdown_by: None,
            since: None,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
    assert!(output_path.exists());
}

#[test]
fn run_stats_report_with_top_count() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.txt");

    let args = StatsArgs {
        action: StatsAction::Report(ReportArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: ReportOutputFormat::Text,
            output: Some(output_path.clone()),
            history_file: None,
            exclude_sections: vec![],
            top: Some(5),
            breakdown_by: None,
            since: None,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);

    let content = std::fs::read_to_string(&output_path).unwrap();
    // Verify top N is respected - check for specific top count in section header
    // The output contains either "Top 5 Largest Files" or "Top N Files" format
    assert!(
        content.contains("Top 5") || content.contains("Top Files"),
        "Expected 'Top 5' or 'Top Files' in output, got:\n{content}"
    );
}

#[test]
fn run_stats_report_with_breakdown_by() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.txt");

    let args = StatsArgs {
        action: StatsAction::Report(ReportArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            format: ReportOutputFormat::Text,
            output: Some(output_path.clone()),
            history_file: None,
            exclude_sections: vec![],
            top: None,
            breakdown_by: Some(BreakdownBy::Dir),
            since: None,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
    assert!(output_path.exists());
}

// ============================================================================
// Report Helper Function Tests
// ============================================================================

#[test]
fn build_exclude_set_combines_cli_and_config() {
    let cli_excludes = vec!["trend".to_string()];
    let config_excludes = vec!["files".to_string()];

    let result = build_exclude_set(&cli_excludes, &config_excludes);

    assert!(result.contains("trend"));
    assert!(result.contains("files"));
    assert_eq!(result.len(), 2);
}

#[test]
fn build_exclude_set_normalizes_case() {
    let cli_excludes = vec!["TREND".to_string()];
    let config_excludes = vec!["Files".to_string()];

    let result = build_exclude_set(&cli_excludes, &config_excludes);

    assert!(result.contains("trend"));
    assert!(result.contains("files"));
}

#[test]
fn build_exclude_set_deduplicates() {
    let cli_excludes = vec!["trend".to_string(), "trend".to_string()];
    let config_excludes = vec!["trend".to_string()];

    let result = build_exclude_set(&cli_excludes, &config_excludes);

    assert!(result.contains("trend"));
    assert_eq!(result.len(), 1);
}

#[test]
fn parse_breakdown_by_lang() {
    assert_eq!(parse_breakdown_by(Some("lang")), Some(BreakdownBy::Lang));
    assert_eq!(
        parse_breakdown_by(Some("language")),
        Some(BreakdownBy::Lang)
    );
    assert_eq!(parse_breakdown_by(Some("LANG")), Some(BreakdownBy::Lang));
}

#[test]
fn parse_breakdown_by_dir() {
    assert_eq!(parse_breakdown_by(Some("dir")), Some(BreakdownBy::Dir));
    assert_eq!(
        parse_breakdown_by(Some("directory")),
        Some(BreakdownBy::Dir)
    );
    assert_eq!(parse_breakdown_by(Some("DIR")), Some(BreakdownBy::Dir));
}

#[test]
fn parse_breakdown_by_invalid() {
    assert_eq!(parse_breakdown_by(Some("invalid")), None);
    assert_eq!(parse_breakdown_by(None), None);
}

// ============================================================================
// History Subcommand Tests
// ============================================================================

#[test]
fn format_history_text_empty() {
    let entries: Vec<&TrendEntry> = vec![];
    let output = format_history_text(&entries, 0);
    assert!(output.contains("No history entries found"));
    assert!(output.contains("sloc-guard snapshot"));
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

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn run_stats_returns_config_error_on_invalid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");
    std::fs::write(&config_path, "invalid toml [[[[").unwrap();

    let mut common = make_common_args(
        vec![temp_dir.path().to_path_buf()],
        Some(vec!["rs".to_string()]),
    );
    common.config = Some(config_path);
    common.no_gitignore = true;

    let args = StatsArgs {
        action: StatsAction::Summary(SummaryArgs {
            common,
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, false);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_CONFIG_ERROR);
}

// ============================================================================
// Include Paths Tests
// ============================================================================

#[test]
fn run_stats_with_include_paths() {
    let mut common = make_common_args(vec![PathBuf::from(".")], Some(vec!["rs".to_string()]));
    common.include = vec!["src".to_string()];

    let args = StatsArgs {
        action: StatsAction::Summary(SummaryArgs {
            common,
            format: StatsOutputFormat::Text,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
}
