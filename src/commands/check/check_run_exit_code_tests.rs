//! Tests for exit code behavior in `run_check_impl`.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, ExtendsPolicy, InitArgs};
use crate::output::OutputFormat;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::*;

fn make_cli_for_check(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
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

fn default_check_args() -> CheckArgs {
    CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(3000),
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
        staged: false,
        warnings_as_errors: false,
        fail_fast: false,
        strict: false,
        baseline: None,
        update_baseline: None,
        ratchet: None,
        no_sloc_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
        write_sarif: None,
        write_json: None,
        files: vec![],
    }
}

#[test]
fn run_check_impl_with_valid_directory() {
    let args = CheckArgs {
        exclude: vec!["**/target/**".to_string()],
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_with_threshold_exceeded() {
    let args = CheckArgs {
        max_lines: Some(1),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
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
        no_gitignore: true,
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);
    let exit_code = run_check(&args, &cli);
    assert_eq!(exit_code, EXIT_CONFIG_ERROR);
}
