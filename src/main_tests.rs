use std::path::PathBuf;

use sloc_guard::checker::{CheckStatus, ThresholdChecker};
use sloc_guard::config::Config;
use sloc_guard::counter::LineStats;
use sloc_guard::language::LanguageRegistry;
use sloc_guard::output::{ColorMode, OutputFormat};
use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};
use tempfile::TempDir;

use crate::{
    compute_effective_stats, format_output, format_stats_output, load_config, process_file,
    write_output,
};

#[test]
fn exit_codes_documented() {
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(EXIT_THRESHOLD_EXCEEDED, 1);
    assert_eq!(EXIT_CONFIG_ERROR, 2);
}

#[test]
fn load_config_no_config_returns_default() {
    let config = load_config(None, true, false).unwrap();
    assert_eq!(config.default.max_lines, 500);
}

#[test]
fn load_config_with_nonexistent_path_returns_error() {
    let result = load_config(Some(std::path::Path::new("nonexistent.toml")), false, false);
    assert!(result.is_err());
}

#[test]
fn load_config_without_no_config_searches_defaults() {
    let config = load_config(None, false, false).unwrap();
    assert!(config.default.max_lines > 0);
}

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
    let results: Vec<sloc_guard::checker::CheckResult> = vec![];
    let output = format_output(OutputFormat::Text, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_output_json() {
    let results: Vec<sloc_guard::checker::CheckResult> = vec![];
    let output = format_output(OutputFormat::Json, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_output_sarif_works() {
    let results: Vec<sloc_guard::checker::CheckResult> = vec![];
    let result = format_output(OutputFormat::Sarif, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("$schema"));
    assert!(output.contains("2.1.0"));
}

#[test]
fn format_output_markdown_works() {
    let results: Vec<sloc_guard::checker::CheckResult> = vec![];
    let result = format_output(OutputFormat::Markdown, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Guard Results"));
    assert!(output.contains("| Total Files | 0 |"));
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
fn format_stats_output_markdown_works() {
    let stats = sloc_guard::output::ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Markdown, &stats);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Statistics"));
    assert!(output.contains("| Total Files | 0 |"));
}

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
    let result = write_output(None, "test content", true);
    assert!(result.is_ok());
}

#[test]
fn write_output_normal_mode() {
    let result = write_output(None, "", false);
    assert!(result.is_ok());
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

#[test]
fn load_config_with_no_extends_returns_config_without_merging() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("child.toml");
    let content = r#"
extends = "https://example.com/base.toml"

[default]
max_lines = 200
"#;
    std::fs::write(&config_path, content).unwrap();

    let config = load_config(Some(&config_path), false, true).unwrap();
    assert_eq!(config.default.max_lines, 200);
    assert_eq!(
        config.extends,
        Some("https://example.com/base.toml".to_string())
    );
}
