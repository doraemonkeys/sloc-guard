//! Tests for config show command and `format_config_text`.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::cli::{Cli, ColorChoice, Commands, ConfigOutputFormat, ExtendsPolicy, InitArgs};
use crate::config::{Config, ContentRule};

use super::super::*;

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
        extends_policy: ExtendsPolicy::Normal,
    }
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
