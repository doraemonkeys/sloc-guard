//! Tests for primary output format handling (--format with --output)

use std::path::PathBuf;

use tempfile::TempDir;

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
fn run_check_impl_with_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.json");

    let args = CheckArgs {
        format: OutputFormat::Json,
        output: Some(output_path.clone()),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, false, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("summary"));
}

#[test]
fn run_check_impl_with_sarif_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.sarif");

    let args = CheckArgs {
        format: OutputFormat::Sarif,
        output: Some(output_path.clone()),
        ..default_check_args()
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
        format: OutputFormat::Markdown,
        output: Some(output_path.clone()),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, false, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("SLOC Guard Results"));
}
