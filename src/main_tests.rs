use std::path::PathBuf;

use sloc_guard::checker::{CheckResult, CheckStatus, ThresholdChecker};
use sloc_guard::cli::{CheckArgs, ColorChoice, InitArgs, StatsArgs};
use sloc_guard::config::Config;
use sloc_guard::counter::LineStats;
use sloc_guard::language::LanguageRegistry;
use sloc_guard::output::{ColorMode, OutputFormat};
use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};
use tempfile::TempDir;

use crate::{
    apply_cli_overrides, collect_file_stats, color_choice_to_mode, compute_effective_stats,
    format_output, format_stats_output, get_scan_paths, get_stats_scan_paths, load_config,
    process_file, run_check_impl, run_stats_impl, write_output,
};
use sloc_guard::cli::Cli;

#[test]
fn exit_codes_documented() {
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(EXIT_THRESHOLD_EXCEEDED, 1);
    assert_eq!(EXIT_CONFIG_ERROR, 2);
}

#[test]
fn load_config_no_config_returns_default() {
    let config = load_config(None, true).unwrap();
    assert_eq!(config.default.max_lines, 500);
}

#[test]
fn load_config_with_nonexistent_path_returns_error() {
    let result = load_config(Some(std::path::Path::new("nonexistent.toml")), false);
    assert!(result.is_err());
}

#[test]
fn load_config_without_no_config_searches_defaults() {
    // This test will return default config if no config file exists
    let config = load_config(None, false).unwrap();
    assert!(config.default.max_lines > 0);
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
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.default.max_lines, 100);
}

#[test]
fn apply_cli_overrides_no_skip_comments() {
    let mut config = Config::default();
    assert!(config.default.skip_comments);

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
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.default.skip_comments);
}

#[test]
fn apply_cli_overrides_no_skip_blank() {
    let mut config = Config::default();
    assert!(config.default.skip_blank);

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
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.default.skip_blank);
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
    };

    apply_cli_overrides(&mut config, &args);
    assert!((config.default.warn_threshold - 0.8).abs() < f64::EPSILON);
}

#[test]
fn get_scan_paths_uses_include_override() {
    let config = Config::default();
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec!["src".to_string(), "lib".to_string()],
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
    };

    let paths = get_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("lib")]);
}

#[test]
fn get_scan_paths_uses_cli_paths() {
    let config = Config::default();
    let args = CheckArgs {
        paths: vec![PathBuf::from("src"), PathBuf::from("tests")],
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
    };

    let paths = get_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("tests")]);
}

#[test]
fn get_scan_paths_uses_config_include_paths() {
    let mut config = Config::default();
    config.default.include_paths = vec!["src".to_string()];

    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
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
    };

    let paths = get_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src")]);
}

#[test]
fn get_scan_paths_defaults_to_current_dir() {
    let config = Config::default();
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
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
    };

    let paths = get_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from(".")]);
}

#[test]
fn compute_effective_stats_skip_both() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
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
    let path = PathBuf::from("nonexistent_file.rs");

    let result = process_file(&path, &registry, &checker, true, true);
    assert!(result.is_none());
}

#[test]
fn process_file_unknown_extension_returns_none() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let path = PathBuf::from("Cargo.toml");

    let result = process_file(&path, &registry, &checker, true, true);
    assert!(result.is_none());
}

#[test]
fn process_file_valid_rust_file() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let path = PathBuf::from("src/lib.rs");

    let result = process_file(&path, &registry, &checker, true, true);
    assert!(result.is_some());
    let check_result = result.unwrap();
    assert_eq!(check_result.status, CheckStatus::Passed);
}

#[test]
fn format_output_text() {
    let results: Vec<CheckResult> = vec![];
    let output = format_output(OutputFormat::Text, &results, ColorMode::Never, 0).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_output_json() {
    let results: Vec<CheckResult> = vec![];
    let output = format_output(OutputFormat::Json, &results, ColorMode::Never, 0).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_output_sarif_works() {
    let results: Vec<CheckResult> = vec![];
    let result = format_output(OutputFormat::Sarif, &results, ColorMode::Never, 0);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("$schema"));
    assert!(output.contains("2.1.0"));
}

#[test]
fn format_output_markdown_not_implemented() {
    let results: Vec<CheckResult> = vec![];
    let result = format_output(OutputFormat::Markdown, &results, ColorMode::Never, 0);
    assert!(result.is_err());
}

// Stats command tests

#[test]
fn get_stats_scan_paths_uses_include_override() {
    let config = Config::default();
    let args = StatsArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        ext: None,
        exclude: vec![],
        include: vec!["src".to_string(), "lib".to_string()],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
    };

    let paths = get_stats_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("lib")]);
}

#[test]
fn get_stats_scan_paths_uses_cli_paths() {
    let config = Config::default();
    let args = StatsArgs {
        paths: vec![PathBuf::from("src"), PathBuf::from("tests")],
        config: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
    };

    let paths = get_stats_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("tests")]);
}

#[test]
fn get_stats_scan_paths_uses_config_include_paths() {
    let mut config = Config::default();
    config.default.include_paths = vec!["src".to_string()];

    let args = StatsArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
    };

    let paths = get_stats_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src")]);
}

#[test]
fn get_stats_scan_paths_defaults_to_current_dir() {
    let config = Config::default();
    let args = StatsArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        format: OutputFormat::Text,
        output: None,
        no_cache: true,
    };

    let paths = get_stats_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from(".")]);
}

#[test]
fn collect_file_stats_nonexistent_returns_none() {
    let registry = LanguageRegistry::default();
    let path = PathBuf::from("nonexistent_file.rs");

    let result = collect_file_stats(&path, &registry);
    assert!(result.is_none());
}

#[test]
fn collect_file_stats_unknown_extension_returns_none() {
    let registry = LanguageRegistry::default();
    let path = PathBuf::from("Cargo.toml");

    let result = collect_file_stats(&path, &registry);
    assert!(result.is_none());
}

#[test]
fn collect_file_stats_valid_rust_file() {
    let registry = LanguageRegistry::default();
    let path = PathBuf::from("src/lib.rs");

    let result = collect_file_stats(&path, &registry);
    assert!(result.is_some());
    let file_stats = result.unwrap();
    assert_eq!(file_stats.path, path);
    assert!(file_stats.stats.total > 0);
}

#[test]
fn format_stats_output_text() {
    let stats = sloc_guard::output::ProjectStatistics::new(vec![]);
    let output = format_stats_output(OutputFormat::Text, &stats).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_stats_output_json() {
    let stats = sloc_guard::output::ProjectStatistics::new(vec![]);
    let output = format_stats_output(OutputFormat::Json, &stats).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_stats_output_sarif_not_implemented() {
    let stats = sloc_guard::output::ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Sarif, &stats);
    assert!(result.is_err());
}

#[test]
fn format_stats_output_markdown_not_implemented() {
    let stats = sloc_guard::output::ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Markdown, &stats);
    assert!(result.is_err());
}

// color_choice_to_mode tests

#[test]
fn color_choice_to_mode_auto() {
    assert_eq!(color_choice_to_mode(ColorChoice::Auto), ColorMode::Auto);
}

#[test]
fn color_choice_to_mode_always() {
    assert_eq!(color_choice_to_mode(ColorChoice::Always), ColorMode::Always);
}

#[test]
fn color_choice_to_mode_never() {
    assert_eq!(color_choice_to_mode(ColorChoice::Never), ColorMode::Never);
}

// write_output tests

#[test]
fn write_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.txt");

    let result = write_output(Some(&output_path), "test content", false);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn write_output_quiet_mode() {
    // In quiet mode, nothing should be written to stdout (no error)
    let result = write_output(None, "test content", true);
    assert!(result.is_ok());
}

#[test]
fn write_output_normal_mode() {
    // Normal mode should succeed (writes to stdout)
    let result = write_output(None, "", false);
    assert!(result.is_ok());
}

// Integration tests for run_check_impl

fn make_cli_for_check(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: sloc_guard::cli::Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
        }),
        verbose,
        quiet,
        color,
        no_config,
    }
}

fn make_cli_for_stats(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: sloc_guard::cli::Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
        }),
        verbose,
        quiet,
        color,
        no_config,
    }
}

#[test]
fn run_check_impl_with_valid_directory() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(2000), // Increased to accommodate growing test file
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
        max_lines: Some(1), // Very low threshold to trigger failures
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: true, // Enable warn-only mode
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // With warn_only, should return SUCCESS even with failures
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_with_threshold_exceeded() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(1), // Very low threshold
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
        max_lines: Some(2000), // Increased to accommodate growing test file
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
        max_lines: Some(2000), // Increased to accommodate growing test file
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
        max_lines: Some(2000), // Increased to accommodate growing test file
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
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
}

// Integration tests for run_stats_impl

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
    };

    let cli = make_cli_for_stats(ColorChoice::Never, 0, true, true);

    let result = run_stats_impl(&args, &cli);
    assert!(result.is_ok());
}

// Strict mode tests

#[test]
fn run_check_impl_strict_mode_fails_on_warnings() {
    // Create a scenario that triggers warnings but not failures
    // Use a threshold that causes warnings (warn_threshold) but not failures
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(10000), // High enough to pass
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: Some(0.001), // Very low threshold to trigger warnings
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: true, // Enable strict mode
        baseline: None,
        no_cache: true,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // With strict mode, warnings should cause exit code 1
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_impl_strict_mode_disabled_warnings_pass() {
    // Same scenario but without strict mode
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(10000), // High enough to pass
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: Some(0.001), // Very low threshold to trigger warnings
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false, // Strict mode disabled
        baseline: None,
        no_cache: true,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Without strict mode, warnings should NOT cause exit code 1
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_warn_only_overrides_strict() {
    // When warn_only is set, strict mode should be ignored
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: Some(1), // Very low threshold to trigger failures
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: true, // Enable warn-only mode
        diff: None,
        strict: true, // Also enable strict mode
        baseline: None,
        no_cache: true,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // warn_only should take precedence, so return SUCCESS
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn config_strict_mode_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("strict.toml");
    let content = r"
[default]
max_lines = 500
strict = true
";
    std::fs::write(&config_path, content).unwrap();

    let config: sloc_guard::config::Config = toml::from_str(content).unwrap();
    assert!(config.default.strict);
}

#[test]
fn config_strict_mode_default_false() {
    let config = Config::default();
    assert!(!config.default.strict);
}

// Baseline update command tests

use crate::{get_baseline_scan_paths, run_baseline_update_impl};
use sloc_guard::baseline::Baseline;
use sloc_guard::cli::BaselineUpdateArgs;

#[test]
fn get_baseline_scan_paths_uses_include_override() {
    let config = Config::default();
    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec!["src".to_string(), "lib".to_string()],
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("lib")]);
}

#[test]
fn get_baseline_scan_paths_uses_cli_paths() {
    let config = Config::default();
    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from("src"), PathBuf::from("tests")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec![],
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("tests")]);
}

#[test]
fn get_baseline_scan_paths_uses_config_include_paths() {
    let mut config = Config::default();
    config.default.include_paths = vec!["src".to_string()];

    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec![],
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src")]);
}

#[test]
fn get_baseline_scan_paths_defaults_to_current_dir() {
    let config = Config::default();
    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec![],
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from(".")]);
}

fn make_cli_for_baseline(quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: sloc_guard::cli::Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
        }),
        verbose: 0,
        quiet,
        color: ColorChoice::Never,
        no_config,
    }
}

#[test]
fn run_baseline_update_creates_empty_baseline_when_no_violations() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    // Create a test file that will NOT exceed the threshold
    let test_file_path = temp_dir.path().join("small_file.rs");
    std::fs::write(&test_file_path, "fn main() {}\n").unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
    };

    let cli = make_cli_for_baseline(true, true);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());

    assert!(baseline_path.exists());
    let baseline = Baseline::load(&baseline_path).unwrap();
    assert!(baseline.is_empty());
}

#[test]
fn run_baseline_update_captures_violations() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    // Create a test file that will exceed the threshold
    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    // Create a temp config with a very low threshold
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
    };

    let cli = make_cli_for_baseline(true, false);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // 1 violation

    assert!(baseline_path.exists());
    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1);
}

#[test]
fn run_baseline_update_with_exclude_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    // Create test directory structure
    let src_dir = temp_dir.path().join("src");
    let vendor_dir = temp_dir.path().join("vendor");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&vendor_dir).unwrap();

    // Create test files that would exceed threshold
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(src_dir.join("main.rs"), &large_content).unwrap();
    std::fs::write(vendor_dir.join("lib.rs"), &large_content).unwrap();

    // Create config with low threshold
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec!["**/vendor/**".to_string()],
        include: vec![],
    };

    let cli = make_cli_for_baseline(true, false);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // Only 1 violation (vendor excluded)

    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1);
    // Should only have the src file, not the vendor file
    let files: Vec<_> = baseline.files().keys().collect();
    assert!(files.iter().any(|f| f.contains("main.rs")));
    assert!(!files.iter().any(|f| f.contains("vendor")));
}

#[test]
fn baseline_file_contains_correct_hash() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    // Create a test file that will exceed the threshold
    let test_file_path = temp_dir.path().join("test.rs");
    let content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &content).unwrap();

    // Create config with low threshold
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
    };

    let cli = make_cli_for_baseline(true, false);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());

    let baseline = Baseline::load(&baseline_path).unwrap();
    let entry = baseline.files().values().next().unwrap();

    // Verify hash is a valid SHA-256 (64 hex characters)
    assert_eq!(entry.hash.len(), 64);
    assert!(entry.hash.chars().all(|c| c.is_ascii_hexdigit()));
}

// Baseline comparison tests

use crate::{apply_baseline_comparison, load_baseline};

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

    // Create a valid baseline file
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
        CheckResult {
            path: PathBuf::from("src/file.rs"),
            status: CheckStatus::Failed,
            stats: LineStats {
                total: 600,
                code: 600,
                comment: 0,
                blank: 0,
            },
            limit: 500,
        },
        CheckResult {
            path: PathBuf::from("src/other.rs"),
            status: CheckStatus::Passed,
            stats: LineStats {
                total: 100,
                code: 100,
                comment: 0,
                blank: 0,
            },
            limit: 500,
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
    let mut results = vec![CheckResult {
        path: PathBuf::from("src/new_file.rs"),
        status: CheckStatus::Failed,
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
        },
        limit: 500,
    }];

    let baseline = Baseline::new(); // Empty baseline

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_failed());
}

#[test]
fn apply_baseline_comparison_handles_windows_paths() {
    let mut results = vec![CheckResult {
        path: PathBuf::from("src\\file.rs"), // Windows path
        status: CheckStatus::Failed,
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
        },
        limit: 500,
    }];

    let mut baseline = Baseline::new();
    baseline.set("src/file.rs", 600, "hash123".to_string()); // Unix path in baseline

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_grandfathered());
}

#[test]
fn run_check_impl_with_baseline_grandfathers_violations() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file that will exceed the threshold
    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    // Create a baseline that includes this file
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut baseline = Baseline::new();
    let file_path_str = test_file_path.to_string_lossy().replace('\\', "/");
    baseline.set(&file_path_str, 102, "dummy_hash".to_string());
    baseline.save(&baseline_path).unwrap();

    // Create config with low threshold
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
        baseline: Some(baseline_path), // Use baseline
        no_cache: true,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Should return SUCCESS because the only failure is grandfathered
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_without_baseline_fails_on_violations() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file that will exceed the threshold
    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    // Create config with low threshold
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
        baseline: None, // No baseline
        no_cache: true,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Should return THRESHOLD_EXCEEDED because no baseline
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_impl_with_baseline_fails_on_new_violations() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file that will exceed the threshold
    let test_file_path = temp_dir.path().join("new_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    // Create an empty baseline (file not in baseline = new violation)
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let baseline = Baseline::new();
    baseline.save(&baseline_path).unwrap();

    // Create config with low threshold
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
        baseline: Some(baseline_path), // Baseline exists but doesn't include this file
        no_cache: true,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Should return THRESHOLD_EXCEEDED because file is not in baseline
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

