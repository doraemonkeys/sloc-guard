use std::path::PathBuf;

use crate::cli::{Cli, ColorChoice, Commands, ConfigOutputFormat, InitArgs};
use crate::config::{Config, ContentConfig, ContentRule};
use tempfile::TempDir;

use super::*;

fn make_cli() -> Cli {
    Cli {
        command: Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
            detect: false,
        }),
        verbose: 0,
        quiet: false,
        color: ColorChoice::Never,
        no_config: false,
        no_extends: false,
        offline: false,
    }
}

#[test]
fn validate_config_nonexistent_file_returns_error() {
    let path = std::path::Path::new("nonexistent_config.toml");
    let result = run_config_validate_impl(path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn validate_config_invalid_toml_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");
    std::fs::write(&config_path, "this is not valid { toml }").unwrap();

    let result = run_config_validate_impl(&config_path);
    assert!(result.is_err());
}

#[test]
fn validate_config_valid_minimal_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("minimal.toml");
    std::fs::write(&config_path, "# minimal valid config\n").unwrap();

    let result = run_config_validate_impl(&config_path);
    assert!(result.is_ok());
}

#[test]
fn validate_config_valid_full_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("full.toml");
    let content = r#"
version = "2"

[scanner]
gitignore = true
exclude = ["**/target/**"]

[content]
max_lines = 500
extensions = ["rs", "go"]
skip_comments = true
skip_blank = true
warn_threshold = 0.9

[[content.rules]]
pattern = "src/legacy.rs"
max_lines = 800
reason = "Legacy code"
"#;
    std::fs::write(&config_path, content).unwrap();

    let result = run_config_validate_impl(&config_path);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_invalid_warn_threshold_too_high() {
    let mut config = Config::default();
    config.content.warn_threshold = 1.5;

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("warn_threshold"));
}

#[test]
fn validate_config_semantics_invalid_warn_threshold_negative() {
    let mut config = Config::default();
    config.content.warn_threshold = -0.1;

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("warn_threshold"));
}

#[test]
fn validate_config_semantics_valid_warn_threshold_boundaries() {
    let mut config = Config::default();

    config.content.warn_threshold = 0.0;
    assert!(validate_config_semantics(&config).is_ok());

    config.content.warn_threshold = 1.0;
    assert!(validate_config_semantics(&config).is_ok());
}

#[test]
fn validate_config_semantics_invalid_scanner_exclude_pattern() {
    let mut config = Config::default();
    config.scanner.exclude = vec!["[invalid".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid glob"));
}

#[test]
fn validate_config_semantics_invalid_content_exclude_pattern() {
    let mut config = Config::default();
    config.content.exclude = vec!["[invalid".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid glob"));
}

#[test]
fn config_show_default_returns_text() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    std::fs::write(&config_path, "# empty config uses defaults\n").unwrap();

    let cli = make_cli();
    let result = run_config_show_impl(Some(&config_path), ConfigOutputFormat::Text, &cli);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Effective Configuration"));
    assert!(output.contains("[scanner]"));
    assert!(output.contains("[content]"));
    assert!(output.contains("max_lines = 600"));
}

#[test]
fn config_show_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    std::fs::write(&config_path, "# empty config uses defaults\n").unwrap();

    let cli = make_cli();
    let result = run_config_show_impl(Some(&config_path), ConfigOutputFormat::Json, &cli);
    assert!(result.is_ok());
    let output = result.unwrap();
    // V2 schema has scanner, content, structure sections
    assert!(output.contains("\"scanner\""));
    assert!(output.contains("\"content\""));
    assert!(output.contains("\"max_lines\""));
}

#[test]
fn config_show_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    let content = r#"
version = "2"

[content]
max_lines = 300

[scanner]
exclude = ["**/vendor/**"]
"#;
    std::fs::write(&config_path, content).unwrap();

    let cli = make_cli();
    let result = run_config_show_impl(Some(&config_path), ConfigOutputFormat::Text, &cli);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("max_lines = 300"));
    assert!(output.contains("vendor"));
}

#[test]
fn config_show_nonexistent_file_returns_error() {
    let path = std::path::Path::new("nonexistent_config.toml");
    let cli = make_cli();
    let result = run_config_show_impl(Some(path), ConfigOutputFormat::Text, &cli);
    assert!(result.is_err());
}

#[test]
fn format_config_text_includes_all_sections() {
    let mut config = Config::default();
    config.content.rules.push(ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 300,
        warn_threshold: Some(0.85),
        warn_at: None,
        skip_comments: Some(true),
        skip_blank: None,
        reason: Some("Rust files".to_string()),
        expires: None,
    });
    config.scanner.exclude = vec!["**/target/**".to_string()];

    let output = format_config_text(&config);

    assert!(output.contains("[scanner]"));
    assert!(output.contains("[content]"));
    assert!(output.contains("[[content.rules]]"));
    assert!(output.contains("**/*.rs"));
    assert!(output.contains("Rust files"));
}

#[test]
fn format_config_text_shows_check_section() {
    let mut config = Config::default();
    config.check.warnings_as_errors = true;
    config.check.fail_fast = true;

    let output = format_config_text(&config);
    assert!(output.contains("[check]"));
    assert!(output.contains("warnings_as_errors = true"));
    assert!(output.contains("fail_fast = true"));
}

#[test]
fn format_config_text_hides_default_check_section() {
    let config = Config::default();
    let output = format_config_text(&config);
    // Default check section (all false) should not be shown
    assert!(!output.contains("[check]"));
}

#[test]
fn validate_config_semantics_warn_at_greater_than_max_lines() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_at: Some(600), // warn_at > max_lines is invalid
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("content.warn_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn validate_config_semantics_warn_at_equal_to_max_lines() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_at: Some(500), // warn_at == max_lines is invalid
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("content.warn_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn validate_config_semantics_warn_at_less_than_max_lines_is_valid() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_at: Some(400), // warn_at < max_lines is valid
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_rule_warn_at_greater_than_rule_max_lines() {
    let config = Config {
        content: ContentConfig {
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_at: Some(400), // warn_at > max_lines is invalid
                warn_threshold: None,
                skip_comments: None,
                skip_blank: None,
                reason: None,
                expires: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("content.rules[0].warn_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn validate_config_semantics_rule_warn_at_less_than_rule_max_lines_is_valid() {
    let config = Config {
        content: ContentConfig {
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_at: Some(250), // warn_at < max_lines is valid
                warn_threshold: None,
                skip_comments: None,
                skip_blank: None,
                reason: None,
                expires: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

// ============================================================================
// Stats Report Config Validation Tests
// ============================================================================

#[test]
fn validate_config_semantics_stats_report_exclude_valid_values() {
    let mut config = Config::default();
    config.stats.report.exclude = vec![
        "summary".to_string(),
        "files".to_string(),
        "breakdown".to_string(),
        "trend".to_string(),
    ];

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_exclude_case_insensitive() {
    let mut config = Config::default();
    config.stats.report.exclude = vec!["SUMMARY".to_string(), "Trend".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_exclude_invalid_section() {
    let mut config = Config::default();
    config.stats.report.exclude = vec!["invalid_section".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.exclude"));
    assert!(err_msg.contains("invalid_section"));
    assert!(err_msg.contains("summary, files, breakdown, trend"));
}

#[test]
fn validate_config_semantics_stats_report_breakdown_by_valid_lang() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("lang".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_breakdown_by_valid_language() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("language".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_breakdown_by_valid_dir() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("dir".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_breakdown_by_valid_directory() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("directory".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_breakdown_by_case_insensitive() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("LANG".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_breakdown_by_invalid() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("invalid".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.breakdown_by"));
    assert!(err_msg.contains("invalid"));
}

#[test]
fn validate_config_semantics_stats_report_trend_since_valid() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("7d".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_trend_since_valid_week() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("1w".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_trend_since_valid_hours() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("12h".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn validate_config_semantics_stats_report_trend_since_invalid() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("invalid".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.trend_since"));
    assert!(err_msg.contains("invalid"));
}

#[test]
fn validate_config_semantics_stats_report_trend_since_missing_unit() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("30".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.trend_since"));
}

#[test]
fn format_config_text_shows_stats_report() {
    let mut config = Config::default();
    config.stats.report.top_count = Some(20);
    config.stats.report.breakdown_by = Some("dir".to_string());
    config.stats.report.exclude = vec!["trend".to_string()];
    config.stats.report.trend_since = Some("7d".to_string());

    let output = format_config_text(&config);
    assert!(output.contains("[stats.report]"));
    assert!(output.contains("top_count = 20"));
    assert!(output.contains("breakdown_by = \"dir\""));
    assert!(output.contains("trend"));
    assert!(output.contains("trend_since = \"7d\""));
}

#[test]
fn format_config_text_omits_empty_stats_report() {
    let config = Config::default();

    let output = format_config_text(&config);
    // Default config has no stats.report settings, so section should be omitted
    assert!(!output.contains("[stats.report]"));
}

#[test]
fn validate_config_valid_full_config_with_stats_report() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("full_stats.toml");
    let content = r#"
version = "2"

[content]
max_lines = 500

[stats.report]
exclude = ["trend"]
top_count = 15
breakdown_by = "lang"
trend_since = "30d"
"#;
    std::fs::write(&config_path, content).unwrap();

    let result = run_config_validate_impl(&config_path);
    assert!(result.is_ok());
}
