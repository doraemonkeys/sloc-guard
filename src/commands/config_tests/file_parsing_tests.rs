use tempfile::TempDir;

use super::super::*;

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

#[test]
fn validate_config_valid_full_config_with_structure() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("full_structure.toml");
    let content = r#"
version = "2"

[content]
max_lines = 500

[structure]
max_files = 50
warn_threshold = 0.9
warn_files_at = 45
warn_files_threshold = 0.85

[[structure.rules]]
scope = "src/**"
max_files = 30
warn_files_at = 25
warn_threshold = 0.8
"#;
    std::fs::write(&config_path, content).unwrap();

    let result = run_config_validate_impl(&config_path);
    assert!(result.is_ok());
}
