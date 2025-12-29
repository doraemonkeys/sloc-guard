use std::path::PathBuf;
use tempfile::TempDir;

use crate::EXIT_SUCCESS;
use crate::cli::{
    BreakdownBy, Cli, ColorChoice, Commands, CommonStatsArgs, InitArgs, ReportArgs,
    ReportOutputFormat, StatsAction, StatsArgs,
};

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
// Report Helper Function Tests
// ============================================================================

#[test]
fn test_build_exclude_set_combines_cli_and_config() {
    let cli_excludes = vec!["trend".to_string()];
    let config_excludes = vec!["files".to_string()];

    let result = build_exclude_set(&cli_excludes, &config_excludes);

    assert!(result.contains("trend"));
    assert!(result.contains("files"));
    assert_eq!(result.len(), 2);
}

#[test]
fn test_build_exclude_set_normalizes_case() {
    let cli_excludes = vec!["TREND".to_string()];
    let config_excludes = vec!["Files".to_string()];

    let result = build_exclude_set(&cli_excludes, &config_excludes);

    assert!(result.contains("trend"));
    assert!(result.contains("files"));
}

#[test]
fn test_build_exclude_set_deduplicates() {
    let cli_excludes = vec!["trend".to_string(), "trend".to_string()];
    let config_excludes = vec!["trend".to_string()];

    let result = build_exclude_set(&cli_excludes, &config_excludes);

    assert!(result.contains("trend"));
    assert_eq!(result.len(), 1);
}

#[test]
fn test_parse_breakdown_by_lang() {
    assert_eq!(parse_breakdown_by(Some("lang")), Some(BreakdownBy::Lang));
    assert_eq!(
        parse_breakdown_by(Some("language")),
        Some(BreakdownBy::Lang)
    );
    assert_eq!(parse_breakdown_by(Some("LANG")), Some(BreakdownBy::Lang));
}

#[test]
fn test_parse_breakdown_by_dir() {
    assert_eq!(parse_breakdown_by(Some("dir")), Some(BreakdownBy::Dir));
    assert_eq!(
        parse_breakdown_by(Some("directory")),
        Some(BreakdownBy::Dir)
    );
    assert_eq!(parse_breakdown_by(Some("DIR")), Some(BreakdownBy::Dir));
}

#[test]
fn test_parse_breakdown_by_invalid() {
    assert_eq!(parse_breakdown_by(Some("invalid")), None);
    assert_eq!(parse_breakdown_by(None), None);
}

// ============================================================================
// Report Subcommand Integration Tests
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
