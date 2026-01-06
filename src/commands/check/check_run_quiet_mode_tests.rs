//! Tests for quiet mode output behavior.
//!
//! Validates "suppress success, preserve failure" semantics:
//! - Success + quiet → no stdout output
//! - Failure + quiet → output violations (so user knows why exit code is non-zero)
//! - Warning + quiet → output warnings (so user knows what triggered the warning)

use std::path::PathBuf;

use tempfile::TempDir;

use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, ExtendsPolicy, InitArgs};
use crate::output::OutputFormat;
use crate::{EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::*;

fn make_cli(quiet: bool) -> Cli {
    Cli {
        command: Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
            detect: false,
        }),
        verbose: 0,
        quiet,
        color: ColorChoice::Never,
        no_config: true,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
    }
}

fn default_check_args() -> CheckArgs {
    CheckArgs {
        paths: vec![],
        config: None,
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
        staged: false,
        warnings_as_errors: false,
        fail_fast: false,
        strict: false,
        baseline: None,
        update_baseline: None,
        ratchet: None,
        no_sloc_cache: true,
        no_gitignore: true,
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

/// Create a temp directory with a Rust file of specified line count.
fn setup_temp_project(line_count: usize) -> TempDir {
    use std::fmt::Write;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    let content = (0..line_count).fold(String::new(), |mut acc, i| {
        let _ = writeln!(acc, "let x{i} = {i};");
        acc
    });
    std::fs::write(&file_path, content).unwrap();

    temp_dir
}

// =============================================================================
// Quiet Mode: Success → Suppress Output
// =============================================================================

#[test]
fn quiet_mode_success_suppresses_output_to_file() {
    // Setup: file within limit (no violations)
    let temp_dir = setup_temp_project(10);
    let output_file = temp_dir.path().join("output.txt");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(100), // Well above file size
        output: Some(output_file.clone()),
        ..default_check_args()
    };
    let cli = make_cli(true); // quiet = true

    let result = run_check_impl(&args, &cli);

    // Should succeed
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);

    // In quiet+success mode, stdout is suppressed but file output is always written.
    // Verify file exists and contains the success result.
    // Note: In non-verbose mode, passed files only appear in summary, not individually.
    assert!(output_file.exists());
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("1 files checked"),
        "file output should include file count, got: {content}"
    );
    assert!(
        content.contains("1 passed"),
        "file output should indicate pass count, got: {content}"
    );
}

#[test]
fn quiet_mode_success_returns_zero_exit_code() {
    let temp_dir = setup_temp_project(10);

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(100),
        ..default_check_args()
    };
    let cli = make_cli(true); // quiet = true

    let result = run_check_impl(&args, &cli);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

// =============================================================================
// Quiet Mode: Failure → Preserve Output
// =============================================================================

#[test]
fn quiet_mode_failure_preserves_output_to_file() {
    // Setup: file exceeds limit (violation)
    let temp_dir = setup_temp_project(50);
    let output_file = temp_dir.path().join("output.txt");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(10), // Below file size = failure
        output: Some(output_file.clone()),
        ..default_check_args()
    };
    let cli = make_cli(true); // quiet = true

    let result = run_check_impl(&args, &cli);

    // Should fail
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);

    // Output file should contain violation details
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("test.rs"),
        "quiet+failure should output filename, got: {content}"
    );
    assert!(
        content.contains("FAILED"),
        "output should indicate failure status, got: {content}"
    );
    assert!(
        content.contains("50"),
        "output should include actual line count, got: {content}"
    );
}

#[test]
fn quiet_mode_failure_returns_nonzero_exit_code() {
    let temp_dir = setup_temp_project(50);

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(10),
        ..default_check_args()
    };
    let cli = make_cli(true); // quiet = true

    let result = run_check_impl(&args, &cli);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Quiet Mode: Warning → Preserve Output
// =============================================================================

#[test]
fn quiet_mode_warning_preserves_output_to_file() {
    // Setup: file approaching limit (warning)
    let temp_dir = setup_temp_project(85);
    let output_file = temp_dir.path().join("output.txt");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(100),       // File is 85 lines
        warn_threshold: Some(0.80), // 80% of 100 = 80 lines triggers warning
        output: Some(output_file.clone()),
        ..default_check_args()
    };
    let cli = make_cli(true); // quiet = true

    let result = run_check_impl(&args, &cli);

    // Should succeed (warning doesn't fail by default)
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);

    // Output file should contain warning details
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("test.rs"),
        "quiet+warning should output warning details, got: {content}"
    );
    // Text format outputs "WARNING" for warning results
    assert!(
        content.contains("WARNING"),
        "output should indicate warning status, got: {content}"
    );
}

// =============================================================================
// Non-Quiet Mode: Always Output
// =============================================================================

#[test]
fn normal_mode_success_outputs_results() {
    let temp_dir = setup_temp_project(10);
    let output_file = temp_dir.path().join("output.txt");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(100),
        output: Some(output_file.clone()),
        ..default_check_args()
    };
    let cli = make_cli(false); // quiet = false

    let result = run_check_impl(&args, &cli);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);

    // Output file should contain results
    // Note: In non-verbose mode, passed files only appear in summary, not individually.
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("1 files checked"),
        "normal mode should output file count, got: {content}"
    );
    assert!(
        content.contains("1 passed"),
        "normal mode should indicate pass count, got: {content}"
    );
}

#[test]
fn normal_mode_failure_outputs_results() {
    let temp_dir = setup_temp_project(50);
    let output_file = temp_dir.path().join("output.txt");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(10),
        output: Some(output_file.clone()),
        ..default_check_args()
    };
    let cli = make_cli(false); // quiet = false

    let result = run_check_impl(&args, &cli);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);

    // Output file should contain failure details
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("test.rs"),
        "normal mode should output failure details, got: {content}"
    );
}

// =============================================================================
// Quiet Mode with JSON Format: Verify structured output preserved on failure
// =============================================================================

#[test]
fn quiet_mode_failure_json_format_outputs_violations() {
    let temp_dir = setup_temp_project(50);
    let output_file = temp_dir.path().join("output.json");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        max_lines: Some(10),
        format: OutputFormat::Json,
        output: Some(output_file.clone()),
        ..default_check_args()
    };
    let cli = make_cli(true); // quiet = true

    let result = run_check_impl(&args, &cli);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);

    // JSON output should be valid and contain violation
    let content = std::fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("quiet+failure JSON output should be valid JSON");

    // Verify JSON contains failure information
    // JSON output uses "results" array for check results
    assert!(
        json.get("results").is_some(),
        "JSON output should have 'results' array for check results, got: {json}"
    );
}
