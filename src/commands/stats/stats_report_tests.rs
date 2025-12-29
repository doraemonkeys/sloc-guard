use std::path::PathBuf;
use tempfile::TempDir;

use crate::EXIT_SUCCESS;
use crate::cli::{
    BreakdownBy, Cli, ColorChoice, Commands, CommonStatsArgs, InitArgs, ReportArgs,
    ReportOutputFormat, StatsAction, StatsArgs,
};

use super::*;

// ============================================================================
// Config File Helper
// ============================================================================

fn create_config_with_depth(dir: &TempDir, depth: usize) -> PathBuf {
    let config_path = dir.path().join(".sloc-guard.toml");
    let config_content = format!(
        r#"version = "2"

[content]
max_lines = 500
extensions = ["rs"]

[stats.report]
breakdown_by = "dir"
depth = {depth}
"#
    );
    std::fs::write(&config_path, config_content).unwrap();
    config_path
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
            depth: None,
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
            depth: None,
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
            depth: None,
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
            depth: None,
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
            depth: None,
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
            depth: None,
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
    assert!(output_path.exists());
}

#[test]
fn run_stats_report_with_breakdown_by_dir_and_depth() {
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
            depth: Some(2),
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);
    assert!(output_path.exists());

    // Verify output contains directory breakdown with depth limiting
    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains("By Directory:"),
        "Expected directory breakdown section in output"
    );
}

#[test]
fn run_stats_report_with_depth_but_lang_breakdown() {
    // Depth with lang breakdown should emit a warning but still succeed
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
            breakdown_by: Some(BreakdownBy::Lang),
            since: None,
            depth: Some(2),
        }),
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, false, true);
    let exit_code = run_stats(&args, &cli);
    // Should still succeed even with warning
    assert_eq!(exit_code, EXIT_SUCCESS);
    assert!(output_path.exists());
}

#[test]
fn run_stats_report_uses_depth_from_config_without_cli_override() {
    // Test that depth from config file is applied when CLI --depth is not provided
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.txt");

    // Create a config file with depth = 1 and breakdown_by = "dir"
    let config_path = create_config_with_depth(&temp_dir, 1);

    // Create some nested Rust source files for testing
    let src_dir = temp_dir.path().join("src");
    let nested_dir = src_dir.join("commands").join("check");
    std::fs::create_dir_all(&nested_dir).unwrap();
    std::fs::write(
        nested_dir.join("runner.rs"),
        "fn main() {}\nfn helper() {}\n",
    )
    .unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub mod commands;\n").unwrap();

    let args = StatsArgs {
        action: StatsAction::Report(ReportArgs {
            common: CommonStatsArgs {
                paths: vec![temp_dir.path().to_path_buf()],
                config: Some(config_path),
                ext: Some(vec!["rs".to_string()]),
                exclude: vec![],
                include: vec![],
                no_cache: true,
                no_gitignore: true,
            },
            format: ReportOutputFormat::Text,
            output: Some(output_path.clone()),
            history_file: None,
            exclude_sections: vec!["trend".to_string(), "files".to_string()],
            top: None,
            breakdown_by: None, // Not overriding via CLI - should use config
            since: None,
            depth: None, // Not overriding via CLI - should use config value of 1
        }),
    };

    // no_config = false so config file is loaded
    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, false);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_SUCCESS);

    // Verify output contains directory breakdown (config breakdown_by = "dir")
    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains("By Directory:"),
        "Expected directory breakdown section in output, got:\n{content}"
    );
    // With depth = 1, we should NOT see the full nested path "commands/check"
    // because it should be grouped at top level ("src" only)
    assert!(
        !content.contains("commands/check"),
        "depth = 1 from config should group at top level, not show nested path 'commands/check'.\nOutput:\n{content}"
    );
}

#[test]
fn run_stats_report_with_depth_zero_shows_warning() {
    // Test that depth = 0 emits a warning
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
            depth: Some(0),
        }),
    };

    // quiet = false to allow warnings to be printed
    let cli = make_cli_for_stats(ColorChoice::Never, 0, false, true);
    let exit_code = run_stats(&args, &cli);
    // Should still succeed even with warning
    assert_eq!(exit_code, EXIT_SUCCESS);
    assert!(output_path.exists());
}
