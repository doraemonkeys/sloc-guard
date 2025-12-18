use crate::config::{Config, FileOverride, RuleConfig};
use tempfile::TempDir;

use super::{
    format_config_text, run_config_show_impl, run_config_validate_impl, validate_config_semantics,
};

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
[default]
max_lines = 500
extensions = ["rs", "go"]
skip_comments = true
skip_blank = true
warn_threshold = 0.9

[rules.rust]
extensions = ["rs"]
max_lines = 300

[exclude]
patterns = ["**/target/**"]

[[override]]
path = "src/legacy.rs"
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
    config.default.warn_threshold = 1.5;

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("warn_threshold"));
}

#[test]
fn validate_config_semantics_invalid_warn_threshold_negative() {
    let mut config = Config::default();
    config.default.warn_threshold = -0.1;

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("warn_threshold"));
}

#[test]
fn validate_config_semantics_valid_warn_threshold_boundaries() {
    let mut config = Config::default();

    config.default.warn_threshold = 0.0;
    assert!(validate_config_semantics(&config).is_ok());

    config.default.warn_threshold = 1.0;
    assert!(validate_config_semantics(&config).is_ok());
}

#[test]
fn validate_config_semantics_invalid_glob_pattern() {
    let mut config = Config::default();
    config.exclude.patterns = vec!["[invalid".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid glob"));
}

#[test]
fn validate_config_semantics_empty_override_path() {
    let config = Config {
        overrides: vec![FileOverride {
            path: String::new(),
            max_lines: 500,
            reason: None,
        }],
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("path cannot be empty"));
}

#[test]
fn validate_config_semantics_rule_without_extensions_or_max_lines() {
    let mut config = Config::default();
    config.rules.insert(
        "empty_rule".to_string(),
        RuleConfig {
            extensions: vec![],
            max_lines: None,
            skip_comments: None,
            skip_blank: None,
            warn_threshold: None,
        },
    );

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("must specify at least"));
}

#[test]
fn config_show_default_returns_text() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    std::fs::write(&config_path, "# empty config uses defaults\n").unwrap();

    let result = run_config_show_impl(Some(&config_path), "text");
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Effective Configuration"));
    assert!(output.contains("[default]"));
    assert!(output.contains("max_lines = 500"));
}

#[test]
fn config_show_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    std::fs::write(&config_path, "# empty config uses defaults\n").unwrap();

    let result = run_config_show_impl(Some(&config_path), "json");
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("\"default\""));
    assert!(output.contains("\"max_lines\""));
}

#[test]
fn config_show_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    let content = r#"
[default]
max_lines = 300

[exclude]
patterns = ["**/vendor/**"]
"#;
    std::fs::write(&config_path, content).unwrap();

    let result = run_config_show_impl(Some(&config_path), "text");
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("max_lines = 300"));
    assert!(output.contains("vendor"));
}

#[test]
fn config_show_nonexistent_file_returns_error() {
    let path = std::path::Path::new("nonexistent_config.toml");
    let result = run_config_show_impl(Some(path), "text");
    assert!(result.is_err());
}

#[test]
fn format_config_text_includes_all_sections() {
    let mut config = Config::default();
    config.rules.insert(
        "rust".to_string(),
        RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(300),
            skip_comments: Some(true),
            skip_blank: None,
            warn_threshold: None,
        },
    );
    config.exclude.patterns = vec!["**/target/**".to_string()];
    config.overrides = vec![FileOverride {
        path: "src/legacy.rs".to_string(),
        max_lines: 800,
        reason: Some("Legacy code".to_string()),
    }];

    let output = format_config_text(&config);

    assert!(output.contains("[default]"));
    assert!(output.contains("[rules.rust]"));
    assert!(output.contains("[exclude]"));
    assert!(output.contains("[[override]]"));
    assert!(output.contains("src/legacy.rs"));
    assert!(output.contains("Legacy code"));
}

#[test]
fn format_config_text_with_include_paths() {
    let mut config = Config::default();
    config.default.include_paths = vec!["src".to_string(), "lib".to_string()];

    let output = format_config_text(&config);
    assert!(output.contains("include_paths"));
    assert!(output.contains("src"));
    assert!(output.contains("lib"));
}

#[test]
fn format_config_text_with_rule_skip_blank() {
    let mut config = Config::default();
    config.rules.insert(
        "test".to_string(),
        RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(300),
            skip_comments: None,
            skip_blank: Some(false),
            warn_threshold: None,
        },
    );

    let output = format_config_text(&config);
    assert!(output.contains("[rules.test]"));
    assert!(output.contains("skip_blank = false"));
}

#[test]
fn format_config_text_shows_strict() {
    let mut config = Config::default();
    config.default.strict = true;

    let output = format_config_text(&config);
    assert!(output.contains("strict = true"));
}
