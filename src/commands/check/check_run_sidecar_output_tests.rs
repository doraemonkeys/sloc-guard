//! Tests for sidecar output files (--report-json, --write-sarif, --write-json)

use std::path::PathBuf;

use tempfile::TempDir;

use crate::EXIT_THRESHOLD_EXCEEDED;
use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, InitArgs};
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
        offline: false,
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

// ============================================================================
// --report-json tests
// ============================================================================

#[test]
fn run_check_impl_with_report_json_creates_stats_file() {
    let temp_dir = TempDir::new().unwrap();
    let stats_output = temp_dir.path().join("stats.json");

    let args = CheckArgs {
        report_json: Some(stats_output.clone()),
        ..default_check_args()
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

    let args = default_check_args();

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
        max_lines: Some(1),
        report_json: Some(stats_output.clone()),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Should still return threshold exceeded
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);

    // Stats file should still be created even with failures
    assert!(stats_output.exists());
}

// ============================================================================
// --write-sarif tests
// ============================================================================

#[test]
fn run_check_impl_with_write_sarif_creates_sarif_file() {
    let temp_dir = TempDir::new().unwrap();
    let sarif_path = temp_dir.path().join("output.sarif");

    let args = CheckArgs {
        write_sarif: Some(sarif_path.clone()),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());

    // Verify SARIF file was created
    assert!(sarif_path.exists(), "SARIF file should be created");

    // Verify SARIF content structure
    let content = std::fs::read_to_string(&sarif_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["$schema"].is_string(), "SARIF should have $schema");
    assert!(json["version"].is_string(), "SARIF should have version");
    assert!(json["runs"].is_array(), "SARIF should have runs array");
}

// ============================================================================
// --write-json tests
// ============================================================================

#[test]
fn run_check_impl_with_write_json_creates_json_file() {
    let temp_dir = TempDir::new().unwrap();
    let json_path = temp_dir.path().join("output.json");

    let args = CheckArgs {
        write_json: Some(json_path.clone()),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());

    // Verify JSON file was created
    assert!(json_path.exists(), "JSON file should be created");

    // Verify JSON content structure (uses 'results' not 'files')
    let content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["summary"].is_object(), "JSON should have summary");
    assert!(json["results"].is_array(), "JSON should have results array");
}

// ============================================================================
// Combined sidecar output tests
// ============================================================================

#[test]
fn run_check_impl_with_both_write_sarif_and_write_json() {
    let temp_dir = TempDir::new().unwrap();
    let sarif_path = temp_dir.path().join("output.sarif");
    let json_path = temp_dir.path().join("output.json");

    let args = CheckArgs {
        write_sarif: Some(sarif_path.clone()),
        write_json: Some(json_path.clone()),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());

    // Verify both files were created
    assert!(sarif_path.exists(), "SARIF file should be created");
    assert!(json_path.exists(), "JSON file should be created");

    // Verify SARIF content
    let sarif_content = std::fs::read_to_string(&sarif_path).unwrap();
    let sarif_json: serde_json::Value = serde_json::from_str(&sarif_content).unwrap();
    assert!(sarif_json["$schema"].is_string());

    // Verify JSON content (uses 'results' not 'files')
    let json_content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_content).unwrap();
    assert!(json["summary"].is_object());
    assert!(json["results"].is_array());
}

#[test]
fn run_check_impl_write_formats_create_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    // Use nested directories that don't exist yet
    let sarif_path = temp_dir.path().join("nested/dir/output.sarif");
    let json_path = temp_dir.path().join("another/nested/output.json");

    let args = CheckArgs {
        write_sarif: Some(sarif_path.clone()),
        write_json: Some(json_path.clone()),
        ..default_check_args()
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());

    // Verify parent directories were created and files exist
    assert!(
        sarif_path.exists(),
        "SARIF file should be created in nested directory"
    );
    assert!(
        json_path.exists(),
        "JSON file should be created in nested directory"
    );
}
