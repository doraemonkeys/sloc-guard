use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

use crate::baseline::Baseline;
use crate::cache::Cache;
use crate::checker::{CheckResult, ThresholdChecker};
use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, InitArgs};
use crate::config::Config;
use crate::counter::LineStats;
use crate::language::LanguageRegistry;
use crate::output::{ColorMode, OutputFormat};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::*;

#[test]
fn compute_effective_stats_skip_both() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, true, true);
    assert_eq!(effective.code, 80);
    assert_eq!(effective.comment, 15);
    assert_eq!(effective.blank, 5);
}

#[test]
fn compute_effective_stats_include_comments() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, false, true);
    assert_eq!(effective.code, 95);
    assert_eq!(effective.comment, 0);
    assert_eq!(effective.blank, 5);
}

#[test]
fn compute_effective_stats_include_blanks() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, true, false);
    assert_eq!(effective.code, 85);
    assert_eq!(effective.comment, 15);
    assert_eq!(effective.blank, 0);
}

#[test]
fn compute_effective_stats_include_both() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, false, false);
    assert_eq!(effective.code, 100);
    assert_eq!(effective.comment, 0);
    assert_eq!(effective.blank, 0);
}

#[test]
fn process_file_nonexistent_returns_none() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let cache = Mutex::new(Cache::new(String::new()));
    let path = PathBuf::from("nonexistent_file.rs");

    let result = process_file_for_check(&path, &registry, &checker, &cache);
    assert!(result.is_none());
}

#[test]
fn process_file_unknown_extension_returns_none() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let cache = Mutex::new(Cache::new(String::new()));
    let path = PathBuf::from("Cargo.toml");

    let result = process_file_for_check(&path, &registry, &checker, &cache);
    assert!(result.is_none());
}

#[test]
fn process_file_valid_rust_file() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let cache = Mutex::new(Cache::new(String::new()));
    let path = PathBuf::from("src/lib.rs");

    let result = process_file_for_check(&path, &registry, &checker, &cache);
    assert!(result.is_some());
    let check_result = result.unwrap();
    assert!(check_result.is_passed());
}

#[test]
fn format_output_text() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let output = format_output(OutputFormat::Text, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_output_json() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let output = format_output(OutputFormat::Json, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_output_sarif_works() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let result = format_output(OutputFormat::Sarif, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("$schema"));
    assert!(output.contains("2.1.0"));
}

#[test]
fn format_output_markdown_works() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let result = format_output(OutputFormat::Markdown, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Guard Results"));
    assert!(output.contains("| Total Files | 0 |"));
}

// Integration tests moved from main_integration_tests.rs

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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
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
        max_files: None,
        max_dirs: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, false, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("SLOC Guard Results"));
}

#[test]
fn apply_cli_overrides_max_lines() {
    let mut config = Config::default();
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: Some(100),
        ext: None,
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
        max_files: None,
        max_dirs: None,
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.content.max_lines, 100);
}

#[test]
fn apply_cli_overrides_no_skip_comments() {
    let mut config = Config::default();
    assert!(config.content.skip_comments);

    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: true,
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
        max_files: None,
        max_dirs: None,
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.content.skip_comments);
}

#[test]
fn apply_cli_overrides_no_skip_blank() {
    let mut config = Config::default();
    assert!(config.content.skip_blank);

    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
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
        max_files: None,
        max_dirs: None,
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.content.skip_blank);
}

#[test]
fn apply_cli_overrides_warn_threshold() {
    let mut config = Config::default();
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: None,
        max_dirs: None,
    };

    apply_cli_overrides(&mut config, &args);
    assert!((config.content.warn_threshold - 0.8).abs() < f64::EPSILON);
}

// Baseline comparison tests moved from main_baseline_tests.rs

#[test]
fn load_baseline_none_path_returns_none() {
    let result = load_baseline(None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn load_baseline_nonexistent_file_returns_error() {
    let result = load_baseline(Some(std::path::Path::new("nonexistent-baseline.json")));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn load_baseline_valid_file_returns_baseline() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let mut baseline = Baseline::new();
    baseline.set("test/file.rs", 100, "abc123".to_string());
    baseline.save(&baseline_path).unwrap();

    let result = load_baseline(Some(&baseline_path));
    assert!(result.is_ok());
    let loaded = result.unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.len(), 1);
    assert!(loaded.contains("test/file.rs"));
}

#[test]
fn apply_baseline_comparison_marks_failed_as_grandfathered() {
    let mut results = vec![
        CheckResult::Failed {
            path: PathBuf::from("src/file.rs"),
            stats: LineStats {
                total: 600,
                code: 600,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            limit: 500,
            override_reason: None,
            suggestions: None,
        },
        CheckResult::Passed {
            path: PathBuf::from("src/other.rs"),
            stats: LineStats {
                total: 100,
                code: 100,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            limit: 500,
            override_reason: None,
        },
    ];

    let mut baseline = Baseline::new();
    baseline.set("src/file.rs", 600, "hash123".to_string());

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_grandfathered());
    assert!(results[1].is_passed());
}

#[test]
fn apply_baseline_comparison_does_not_mark_new_violations() {
    let mut results = vec![CheckResult::Failed {
        path: PathBuf::from("src/new_file.rs"),
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
        suggestions: None,
    }];

    let baseline = Baseline::new();

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_failed());
}

#[test]
fn apply_baseline_comparison_handles_windows_paths() {
    let mut results = vec![CheckResult::Failed {
        path: PathBuf::from("src\\file.rs"),
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
        suggestions: None,
    }];

    let mut baseline = Baseline::new();
    baseline.set("src/file.rs", 600, "hash123".to_string());

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_grandfathered());
}

#[test]
fn run_check_impl_with_baseline_grandfathers_violations() {
    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut baseline = Baseline::new();
    let file_path_str = test_file_path.to_string_lossy().replace('\\', "/");
    baseline.set(&file_path_str, 102, "dummy_hash".to_string());
    baseline.save(&baseline_path).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

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
        baseline: Some(baseline_path),
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: None,
        max_dirs: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_without_baseline_fails_on_violations() {
    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

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
        max_files: None,
        max_dirs: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_impl_with_baseline_fails_on_new_violations() {
    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("new_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let baseline = Baseline::new();
    baseline.save(&baseline_path).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

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
        baseline: Some(baseline_path),
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Context Injection Tests (Task 5.5.13: Testability Refactoring)
// =============================================================================

use crate::commands::context::CheckContext;

#[test]
fn check_context_from_config_creates_valid_context() {
    let config = Config::default();
    let ctx = CheckContext::from_config(&config, 0.9).unwrap();

    // Verify context contains expected components
    // Default config has no structure limits, so structure checker exists but is not enabled
    if let Some(ref sc) = ctx.structure_checker {
        // Structure checker is created but not enabled with default config (no limits set)
        assert!(!sc.is_enabled());
    }
}

#[test]
fn check_context_new_allows_custom_injection() {
    let config = Config::default();
    let registry = LanguageRegistry::default();
    let threshold_checker = ThresholdChecker::new(config).with_warning_threshold(0.5);

    // Create context with custom components (no structure checker)
    let ctx = CheckContext::new(registry, threshold_checker, None);

    // Context should have no structure checker
    assert!(ctx.structure_checker.is_none());
}

#[test]
fn run_check_with_context_uses_injected_threshold_checker() {
    // This test demonstrates that run_check_with_context uses the injected
    // threshold_checker instead of creating one internally.
    let temp_dir = TempDir::new().unwrap();

    // Create a small file that would pass with default limits
    let test_file = temp_dir.path().join("small.rs");
    std::fs::write(&test_file, "fn main() {}\n").unwrap();

    // Create context with very strict threshold (max_lines = 1)
    let mut config = Config::default();
    config.content.extensions = vec!["rs".to_string()];
    config.content.max_lines = 1; // Very strict

    let ctx = CheckContext::from_config(&config, 0.9).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: None,
        max_dirs: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);
    let paths = args.paths.clone();

    // The injected context's threshold_checker should detect the violation
    let result = run_check_with_context(&args, &cli, &paths, &config, &ctx, &cache, None);
    assert!(result.is_ok());
    // File has 1 line, limit is 1, so it should pass (equal to limit)
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_with_context_uses_injected_structure_checker() {
    // This test demonstrates structure checker injection
    let temp_dir = TempDir::new().unwrap();

    // Create directory with files
    let sub_dir = temp_dir.path().join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(sub_dir.join("b.rs"), "fn b() {}").unwrap();
    std::fs::write(sub_dir.join("c.rs"), "fn c() {}").unwrap();

    // Create config with structure limits
    let mut config = Config::default();
    config.content.extensions = vec!["rs".to_string()];
    config.structure.max_files = Some(2); // Limit to 2 files per directory

    let ctx = CheckContext::from_config(&config, 0.9).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));

    // Verify structure checker is enabled
    assert!(ctx.structure_checker.is_some());
    assert!(ctx.structure_checker.as_ref().unwrap().is_enabled());

    let args = CheckArgs {
        paths: vec![sub_dir.clone()],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: None,
        max_dirs: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);
    let paths = vec![sub_dir];

    // Structure checker should detect violation (3 files > 2 limit)
    let result = run_check_with_context(&args, &cli, &paths, &config, &ctx, &cache, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Task 6.1: CLI Structure Parameters Tests
// =============================================================================

#[test]
fn validate_and_resolve_paths_no_args_defaults_to_current_dir() {
    let args = CheckArgs {
        paths: vec![],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: None,
        max_dirs: None,
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_ok());
    let paths = result.unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], PathBuf::from("."));
}

#[test]
fn validate_and_resolve_paths_max_files_without_path_returns_error() {
    let args = CheckArgs {
        paths: vec![],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: Some(10),
        max_dirs: None,
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("--max-files/--max-dirs require a target <PATH>"));
}

#[test]
fn validate_and_resolve_paths_max_dirs_without_path_returns_error() {
    let args = CheckArgs {
        paths: vec![],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: None,
        max_dirs: Some(5),
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("--max-files/--max-dirs require a target <PATH>"));
}

#[test]
fn validate_and_resolve_paths_max_files_with_path_succeeds() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: Some(10),
        max_dirs: None,
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_ok());
    let paths = result.unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], PathBuf::from("src"));
}

#[test]
fn apply_cli_overrides_structure_max_files() {
    let mut config = Config::default();
    assert!(config.structure.max_files.is_none());

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: Some(10),
        max_dirs: None,
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.structure.max_files, Some(10));
}

#[test]
fn apply_cli_overrides_structure_max_dirs() {
    let mut config = Config::default();
    assert!(config.structure.max_dirs.is_none());

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: None,
        ext: None,
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
        max_files: None,
        max_dirs: Some(5),
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.structure.max_dirs, Some(5));
}

#[test]
fn run_check_with_cli_max_files_overrides_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create 3 files in directory
    std::fs::write(temp_dir.path().join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(temp_dir.path().join("b.rs"), "fn b() {}").unwrap();
    std::fs::write(temp_dir.path().join("c.rs"), "fn c() {}").unwrap();

    // Use --max-files=2 to trigger violation (3 files > 2 limit)
    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
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
        max_files: Some(2),
        max_dirs: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // 3 files > 2 limit should fail
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_with_cli_max_dirs_overrides_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create 3 subdirectories
    std::fs::create_dir(temp_dir.path().join("sub1")).unwrap();
    std::fs::create_dir(temp_dir.path().join("sub2")).unwrap();
    std::fs::create_dir(temp_dir.path().join("sub3")).unwrap();

    // Use --max-dirs=2 to trigger violation (3 dirs > 2 limit)
    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
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
        max_files: None,
        max_dirs: Some(2),
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // 3 dirs > 2 limit should fail
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}
