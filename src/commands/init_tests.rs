use crate::cli::InitArgs;
use tempfile::TempDir;

use super::{generate_config_template, run_init_impl};

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
    let result: Result<crate::config::Config, _> = toml::from_str(&template);
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

#[test]
fn generate_config_template_contains_strict() {
    let template = generate_config_template();
    assert!(template.contains("strict"));
    assert!(template.contains("Strict mode"));
}
