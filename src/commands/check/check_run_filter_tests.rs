//! Tests for input filtering and processing options (verbose, count flags, include paths)

use std::path::PathBuf;

use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, ExtendsPolicy, InitArgs};
use crate::output::OutputFormat;

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
fn run_check_impl_with_verbose() {
    let args = CheckArgs {
        warn_threshold: Some(0.8),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Always, 1, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_check_impl_with_count_flags() {
    let args = CheckArgs {
        max_lines: Some(5000),
        count_comments: true,
        count_blank: true,
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_check_impl_with_include_paths() {
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        include: vec!["src".to_string()],
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}
