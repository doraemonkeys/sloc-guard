use std::path::PathBuf;

use sloc_guard::cli::{CheckArgs, Cli, ColorChoice, Commands, GroupBy, InitArgs, StatsArgs};
use sloc_guard::output::OutputFormat;
use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};
use tempfile::TempDir;

use crate::{run_check, run_check_impl, run_stats, run_stats_impl};

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
fn run_check_impl_with_valid_directory() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec!["**/target/**".to_string()],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: true,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Json,
        output: Some(output_path.clone()),
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: Some(0.8),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    let cli = make_cli_for_check(ColorChoice::Always, 1, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_check_impl_with_no_skip_flags() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(5000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: true,
        no_skip_blank: true,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
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
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
}

// Strict mode tests

#[test]
fn run_check_impl_strict_mode_fails_on_warnings() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(10000),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: Some(0.001),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: true,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: Some(0.001),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: true,
        diff: None,
        strict: true,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

// Error handling tests

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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: true,
        fix: false,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);
    let exit_code = run_check(&args, &cli);
    assert_eq!(exit_code, EXIT_CONFIG_ERROR);
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
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, false);
    let exit_code = run_stats(&args, &cli);
    assert_eq!(exit_code, EXIT_CONFIG_ERROR);
}

// Output format tests

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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Sarif,
        output: Some(output_path.clone()),
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Markdown,
        output: Some(output_path.clone()),
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, false, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("SLOC Guard Results"));
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
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
}
