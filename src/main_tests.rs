use std::path::PathBuf;

use sloc_guard::checker::{CheckResult, CheckStatus, ThresholdChecker};
use sloc_guard::cli::{CheckArgs, InitArgs, StatsArgs};
use sloc_guard::config::Config;
use sloc_guard::counter::LineStats;
use sloc_guard::language::LanguageRegistry;
use sloc_guard::output::{ColorMode, OutputFormat};
use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};
use tempfile::TempDir;

use crate::{
    apply_cli_overrides, collect_file_stats, compute_effective_stats, format_output,
    format_stats_output, generate_config_template, get_scan_paths, get_stats_scan_paths,
    load_config, process_file, run_init_impl,
};

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
    let output = format_output(OutputFormat::Text, &results, ColorMode::Never).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_output_json() {
    let results: Vec<CheckResult> = vec![];
    let output = format_output(OutputFormat::Json, &results, ColorMode::Never).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_output_sarif_not_implemented() {
    let results: Vec<CheckResult> = vec![];
    let result = format_output(OutputFormat::Sarif, &results, ColorMode::Never);
    assert!(result.is_err());
}

#[test]
fn format_output_markdown_not_implemented() {
    let results: Vec<CheckResult> = vec![];
    let result = format_output(OutputFormat::Markdown, &results, ColorMode::Never);
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

// Init command tests

#[test]
fn generate_config_template_contains_default_section() {
    let template = generate_config_template();
    assert!(template.contains("[default]"));
    assert!(template.contains("max_lines = 500"));
    assert!(template.contains("skip_comments = true"));
    assert!(template.contains("skip_blank = true"));
    assert!(template.contains("warn_threshold = 0.9"));
}

#[test]
fn generate_config_template_contains_extensions() {
    let template = generate_config_template();
    assert!(template.contains(r#"extensions = ["rs", "go", "py", "js", "ts", "c", "cpp"]"#));
}

#[test]
fn generate_config_template_contains_exclude_section() {
    let template = generate_config_template();
    assert!(template.contains("[exclude]"));
    assert!(template.contains("**/target/**"));
    assert!(template.contains("**/node_modules/**"));
}

#[test]
fn generate_config_template_is_valid_toml() {
    let template = generate_config_template();
    let result: Result<sloc_guard::config::Config, _> = toml::from_str(&template);
    assert!(result.is_ok(), "Generated template should be valid TOML");
}

#[test]
fn run_init_creates_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    let args = InitArgs {
        output: config_path.clone(),
        force: false,
    };

    let result = run_init_impl(&args);
    assert!(result.is_ok());
    assert!(config_path.exists());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[default]"));
}

#[test]
fn run_init_fails_if_file_exists_without_force() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    std::fs::write(&config_path, "existing content").unwrap();

    let args = InitArgs {
        output: config_path,
        force: false,
    };

    let result = run_init_impl(&args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn run_init_overwrites_with_force() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    std::fs::write(&config_path, "old content").unwrap();

    let args = InitArgs {
        output: config_path.clone(),
        force: true,
    };

    let result = run_init_impl(&args);
    assert!(result.is_ok());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[default]"));
    assert!(!content.contains("old content"));
}

#[test]
fn run_init_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("subdir").join(".sloc-guard.toml");

    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();

    let args = InitArgs {
        output: config_path.clone(),
        force: false,
    };

    let result = run_init_impl(&args);
    assert!(result.is_ok());
    assert!(config_path.exists());
}

// Config validate command tests

use crate::{
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
    let mut config = Config::default();
    config.overrides = vec![sloc_guard::config::FileOverride {
        path: String::new(),
        max_lines: 500,
        reason: None,
    }];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("path cannot be empty"));
}

#[test]
fn validate_config_semantics_rule_without_extensions_or_max_lines() {
    let mut config = Config::default();
    config.rules.insert(
        "empty_rule".to_string(),
        sloc_guard::config::RuleConfig {
            extensions: vec![],
            max_lines: None,
            skip_comments: None,
            skip_blank: None,
        },
    );

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("must specify at least"));
}

// Config show command tests

#[test]
fn config_show_default_returns_text() {
    // Create a temp config file with known values
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    std::fs::write(&config_path, "# empty config uses defaults\n").unwrap();

    let result = run_config_show_impl(Some(&config_path), "text");
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Effective Configuration"));
    assert!(output.contains("[default]"));
    assert!(output.contains("max_lines = 500")); // default value
}

#[test]
fn config_show_json_format() {
    // Create a temp config file with known values
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
        sloc_guard::config::RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(300),
            skip_comments: Some(true),
            skip_blank: None,
        },
    );
    config.exclude.patterns = vec!["**/target/**".to_string()];
    config.overrides = vec![sloc_guard::config::FileOverride {
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
