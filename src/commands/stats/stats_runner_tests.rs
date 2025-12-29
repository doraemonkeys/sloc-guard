use std::path::PathBuf;
use tempfile::TempDir;

use crate::cli::{
    BreakdownArgs, BreakdownBy, Cli, ColorChoice, Commands, CommonStatsArgs, FileSortOrder,
    FilesArgs, InitArgs, StatsAction, StatsArgs, StatsOutputFormat, SummaryArgs, TrendArgs,
};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

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

#[test]
fn run_stats_breakdown_depth_zero_shows_warning() {
    // depth = 0 is meaningless (behaves same as None), should warn
    let args = StatsArgs {
        action: StatsAction::Breakdown(BreakdownArgs {
            common: make_common_args(vec![PathBuf::from("src")], Some(vec!["rs".to_string()])),
            by: BreakdownBy::Dir,
            depth: Some(0), // Should trigger warning
            format: StatsOutputFormat::Text,
        }),
    };

    // quiet = false to allow warnings
    let cli = make_cli_for_stats(ColorChoice::Never, 0, false, true);
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
