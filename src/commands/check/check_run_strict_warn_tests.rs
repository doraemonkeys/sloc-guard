//! Tests for strict mode and `warn_only` flag behavior.

use std::path::PathBuf;

use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, InitArgs};
use crate::output::OutputFormat;
use crate::{EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

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
        offline: false,
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
        no_cache: true,
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
fn run_check_impl_with_warn_only() {
    let args = CheckArgs {
        max_lines: Some(1),
        warn_only: true,
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_strict_mode_fails_on_warnings() {
    let args = CheckArgs {
        max_lines: Some(10000),
        warn_threshold: Some(0.001),
        strict: true,
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_impl_strict_mode_disabled_warnings_pass() {
    let args = CheckArgs {
        max_lines: Some(10000),
        warn_threshold: Some(0.001),
        strict: false,
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_warn_only_overrides_strict() {
    let args = CheckArgs {
        max_lines: Some(1),
        warn_only: true,
        strict: true,
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}
