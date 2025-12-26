use crate::cli::InitArgs;
use tempfile::TempDir;

use super::{generate_config_template, run_init, run_init_impl, run_init_with_cwd};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

#[test]
fn generate_config_template_contains_content_section() {
    let template = generate_config_template();
    assert!(template.contains("version = \"2\""));
    assert!(template.contains("[content]"));
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
fn generate_config_template_contains_scanner_section() {
    let template = generate_config_template();
    assert!(template.contains("[scanner]"));
    assert!(template.contains("gitignore = true"));
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
        detect: false,
    };

    let result = run_init_impl(&args);
    assert!(result.is_ok());
    assert!(config_path.exists());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[content]"));
    assert!(content.contains("version = \"2\""));
}

#[test]
fn run_init_fails_if_file_exists_without_force() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    std::fs::write(&config_path, "existing content").unwrap();

    let args = InitArgs {
        output: config_path,
        force: false,
        detect: false,
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
        detect: false,
    };

    let result = run_init_impl(&args);
    assert!(result.is_ok());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[content]"));
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
        detect: false,
    };

    let result = run_init_impl(&args);
    assert!(result.is_ok());
    assert!(config_path.exists());
}

#[test]
fn generate_config_template_contains_structure_section() {
    let template = generate_config_template();
    assert!(template.contains("[structure]"));
    assert!(template.contains("max_files = 30"));
    assert!(template.contains("max_dirs = 10"));
    assert!(template.contains("max_depth = 8"));
}

#[test]
fn run_init_returns_success_exit_code() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    let args = InitArgs {
        output: config_path,
        force: false,
        detect: false,
    };

    let exit_code = run_init(&args);
    assert_eq!(exit_code, EXIT_SUCCESS);
}

#[test]
fn run_init_returns_error_exit_code_when_file_exists() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    std::fs::write(&config_path, "existing").unwrap();

    let args = InitArgs {
        output: config_path,
        force: false,
        detect: false,
    };

    let exit_code = run_init(&args);
    assert_eq!(exit_code, EXIT_CONFIG_ERROR);
}

#[test]
fn run_init_with_detect_creates_rust_config() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"",
    )
    .unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");

    let args = InitArgs {
        output: config_path.clone(),
        force: false,
        detect: true,
    };

    let result = run_init_with_cwd(&args, temp_dir.path());

    assert!(result.is_ok());
    assert!(config_path.exists());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("version = \"2\""));
    assert!(content.contains("\"rs\""));
    assert!(content.contains("Detected"));
}

#[test]
fn run_init_with_detect_handles_unknown_project() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    let args = InitArgs {
        output: config_path.clone(),
        force: false,
        detect: true,
    };

    let result = run_init_with_cwd(&args, temp_dir.path());

    assert!(result.is_ok());
    assert!(config_path.exists());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("version = \"2\""));
    assert!(content.contains("Unknown project type"));
}
