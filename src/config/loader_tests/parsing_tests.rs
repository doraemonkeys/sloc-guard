//! Tests for config parsing, explicit path loading, and error handling.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

#[test]
fn load_from_explicit_path() {
    let config_content = r#"
version = "2"

[content]
max_lines = 700
extensions = ["rs", "py"]
"#;

    let fs = MockFileSystem::new().with_file("/custom/path/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/custom/path/config.toml"))
        .unwrap();

    assert_eq!(result.config.content.max_lines, 700);
    assert_eq!(result.config.content.extensions, vec!["rs", "py"]);
}

#[test]
fn returns_error_for_invalid_toml() {
    let invalid_content = "this is not valid toml [[[";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", invalid_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::TomlParse(_)));
}

#[test]
fn returns_error_for_nonexistent_explicit_path() {
    let fs = MockFileSystem::new();

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/does/not/exist.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::FileAccess { .. }));
}

#[test]
fn parses_full_v2_config() {
    let config_content = r#"
version = "2"

[scanner]
gitignore = true
exclude = ["**/target/**", "**/vendor/**"]

[content]
max_lines = 500
extensions = ["rs", "go"]
skip_comments = true
skip_blank = true
warn_threshold = 0.85

[[content.rules]]
pattern = "**/*.rs"
max_lines = 300
reason = "Rust files"

[[content.rules]]
pattern = "src/legacy.rs"
max_lines = 800
reason = "Legacy code"
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml")).unwrap();
    let config = result.config;

    assert_eq!(config.content.max_lines, 500);
    assert_eq!(config.content.extensions, vec!["rs", "go"]);
    assert!(config.content.skip_comments);
    assert!(config.content.skip_blank);
    assert!((config.content.warn_threshold - 0.85).abs() < f64::EPSILON);

    assert_eq!(config.scanner.exclude, vec!["**/target/**", "**/vendor/**"]);

    assert_eq!(config.content.rules.len(), 2);
    assert_eq!(config.content.rules[0].pattern, "**/*.rs");
    assert_eq!(config.content.rules[0].max_lines, 300);
    assert_eq!(config.content.rules[1].pattern, "src/legacy.rs");
    assert_eq!(config.content.rules[1].max_lines, 800);
    assert_eq!(
        config.content.rules[1].reason,
        Some("Legacy code".to_string())
    );
}
