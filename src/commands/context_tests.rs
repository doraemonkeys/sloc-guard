use crate::cli::ColorChoice;
use crate::config::Config;
use crate::output::ColorMode;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};
use std::path::PathBuf;
use tempfile::TempDir;

use super::*;

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

    let config: Config = toml::from_str(content).unwrap();
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

#[test]
fn resolve_scan_paths_uses_include_override() {
    let config = Config::default();
    let paths = vec![PathBuf::from(".")];
    let include = vec!["src".to_string(), "lib".to_string()];

    let result = resolve_scan_paths(&paths, &include, &config);
    assert_eq!(result, vec![PathBuf::from("src"), PathBuf::from("lib")]);
}

#[test]
fn resolve_scan_paths_uses_cli_paths() {
    let config = Config::default();
    let paths = vec![PathBuf::from("src"), PathBuf::from("tests")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include, &config);
    assert_eq!(result, vec![PathBuf::from("src"), PathBuf::from("tests")]);
}

#[test]
fn resolve_scan_paths_uses_config_include_paths() {
    let mut config = Config::default();
    config.default.include_paths = vec!["src".to_string()];

    let paths = vec![PathBuf::from(".")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include, &config);
    assert_eq!(result, vec![PathBuf::from("src")]);
}

#[test]
fn resolve_scan_paths_defaults_to_current_dir() {
    let config = Config::default();
    let paths = vec![PathBuf::from(".")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include, &config);
    assert_eq!(result, vec![PathBuf::from(".")]);
}

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

// =============================================================================
// FileReader Tests
// =============================================================================

#[test]
fn real_file_reader_reads_file_contents() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"hello world").unwrap();

    let reader = RealFileReader;
    let content = reader.read(&file_path).unwrap();

    assert_eq!(content, b"hello world");
}

#[test]
fn real_file_reader_returns_error_for_nonexistent_file() {
    let reader = RealFileReader;
    let result = reader.read(std::path::Path::new("nonexistent_file.txt"));

    assert!(result.is_err());
}

#[test]
fn real_file_reader_metadata_returns_size() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"hello").unwrap();

    let reader = RealFileReader;
    let (_, size) = reader.metadata(&file_path).unwrap();

    assert_eq!(size, 5); // "hello" is 5 bytes
}

#[test]
fn read_file_with_hash_returns_content_and_hash() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"test content").unwrap();

    let reader = RealFileReader;
    let result = read_file_with_hash(&reader, &file_path);

    assert!(result.is_some());
    let (hash, content) = result.unwrap();
    assert_eq!(content, b"test content");
    assert!(!hash.is_empty());
}

#[test]
fn read_file_with_hash_returns_none_for_nonexistent() {
    let reader = RealFileReader;
    let result = read_file_with_hash(&reader, std::path::Path::new("nonexistent.txt"));

    assert!(result.is_none());
}
