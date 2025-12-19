use std::path::PathBuf;
use tempfile::TempDir;

use crate::cli::{Cli, ColorChoice, Commands, GroupBy, InitArgs, StatsArgs};
use crate::output::{OutputFormat, ProjectStatistics};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::*;

#[test]
fn format_stats_output_text() {
    let stats = ProjectStatistics::new(vec![]);
    let output = format_stats_output(OutputFormat::Text, &stats).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_stats_output_json() {
    let stats = ProjectStatistics::new(vec![]);
    let output = format_stats_output(OutputFormat::Json, &stats).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_stats_output_sarif_not_implemented() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Sarif, &stats);
    assert!(result.is_err());
}

#[test]
fn format_stats_output_markdown_works() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Markdown, &stats);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Statistics"));
    assert!(output.contains("| Total Files | 0 |"));
}

// Integration tests moved from main_integration_tests.rs

fn make_cli_for_stats(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
        }),
        verbose,
        quiet,
        color,
        no_config,
        no_extends: false,
    }
}

#[test]
fn run_stats_impl_with_valid_directory() {
    let args = StatsArgs {
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
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_stats_impl_with_top_files() {
    let args = StatsArgs {
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
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(history_path.exists(), "Custom history file should be created");

    // Verify the history file contains valid JSON
    let content = std::fs::read_to_string(&history_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.get("version").is_some());
    assert!(parsed.get("entries").is_some());
}
