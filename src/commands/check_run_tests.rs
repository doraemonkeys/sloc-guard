use std::path::PathBuf;
use tempfile::TempDir;

use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, InitArgs};
use crate::output::OutputFormat;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::*;

fn make_cli_for_check(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
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
fn run_check_impl_with_valid_directory() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec!["**/target/**".to_string()],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_with_warn_only() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(1),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: true,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_with_threshold_exceeded() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(1),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_impl_with_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.json");

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Json,
        output: Some(output_path.clone()),
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, false, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("summary"));
}

#[test]
fn run_check_impl_with_verbose() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: Some(0.8),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Always, 1, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_check_impl_with_count_flags() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(5000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: true,
        count_blank: true,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_check_impl_with_include_paths() {
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec!["src".to_string()],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_check_impl_strict_mode_fails_on_warnings() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(10000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: Some(0.001),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: true,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_impl_strict_mode_disabled_warnings_pass() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(10000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: Some(0.001),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_warn_only_overrides_strict() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(1),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: true,
        diff: None,
        strict: true,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_returns_config_error_on_invalid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");
    std::fs::write(&config_path, "invalid toml [[[[").unwrap();

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: true,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);
    let exit_code = run_check(&args, &cli);
    assert_eq!(exit_code, EXIT_CONFIG_ERROR);
}

#[test]
fn run_check_impl_with_sarif_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.sarif");

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Sarif,
        output: Some(output_path.clone()),
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, false, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("$schema"));
}

#[test]
fn run_check_impl_with_markdown_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.md");

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Markdown,
        output: Some(output_path.clone()),
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, false, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("SLOC Guard Results"));
}

#[test]
fn run_check_impl_with_report_json_creates_stats_file() {
    let temp_dir = TempDir::new().unwrap();
    let stats_output = temp_dir.path().join("stats.json");

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: Some(stats_output.clone()),
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());

    // Verify stats file was created
    assert!(stats_output.exists(), "Stats JSON file should be created");

    // Verify JSON content structure
    let content = std::fs::read_to_string(&stats_output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Check summary fields exist
    assert!(json["summary"]["total_files"].is_number());
    assert!(json["summary"]["total_lines"].is_number());
    assert!(json["summary"]["code"].is_number());

    // Check files array exists and has entries
    assert!(json["files"].is_array());
    assert!(!json["files"].as_array().unwrap().is_empty());

    // Check language breakdown exists (included by default)
    assert!(json["by_language"].is_array());
}

#[test]
fn run_check_impl_without_report_json_does_not_create_file() {
    let temp_dir = TempDir::new().unwrap();
    let stats_output = temp_dir.path().join("stats.json");

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());

    // Verify stats file was NOT created
    assert!(!stats_output.exists());
}

#[test]
fn run_check_impl_report_json_does_not_affect_exit_code() {
    let temp_dir = TempDir::new().unwrap();
    let stats_output = temp_dir.path().join("stats.json");

    // Set very low limit to trigger failure
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(1),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: Some(stats_output.clone()),
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Should still return threshold exceeded
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);

    // Stats file should still be created even with failures
    assert!(stats_output.exists());
}

