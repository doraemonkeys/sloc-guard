//! Tests for config parsing, explicit path loading, and error handling.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

#[test]
fn load_from_explicit_path() {
    let config_content = r#"
[default]
max_lines = 700
extensions = ["rs", "py"]
"#;

    let fs = MockFileSystem::new().with_file("/custom/path/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/custom/path/config.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 700);
    assert_eq!(config.default.extensions, vec!["rs", "py"]);
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
    assert!(matches!(err, SlocGuardError::FileRead { .. }));
}

#[test]
fn parses_full_config_with_rules_and_overrides() {
    let config_content = r#"
[default]
max_lines = 500
extensions = ["rs", "go"]
include_paths = ["src", "lib"]
skip_comments = true
skip_blank = true
warn_threshold = 0.85

[rules.rust]
extensions = ["rs"]
max_lines = 300

[exclude]
patterns = ["**/target/**", "**/vendor/**"]

[[override]]
path = "src/legacy.rs"
max_lines = 800
reason = "Legacy code"
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert_eq!(config.default.max_lines, 500);
    assert_eq!(config.default.extensions, vec!["rs", "go"]);
    assert_eq!(config.default.include_paths, vec!["src", "lib"]);
    assert!(config.default.skip_comments);
    assert!(config.default.skip_blank);
    assert!((config.default.warn_threshold - 0.85).abs() < f64::EPSILON);

    let rust_rule = config.rules.get("rust").unwrap();
    assert_eq!(rust_rule.extensions, vec!["rs"]);
    assert_eq!(rust_rule.max_lines, Some(300));

    assert_eq!(
        config.exclude.patterns,
        vec!["**/target/**", "**/vendor/**"]
    );

    assert_eq!(config.overrides.len(), 1);
    assert_eq!(config.overrides[0].path, "src/legacy.rs");
    assert_eq!(config.overrides[0].max_lines, 800);
    assert_eq!(config.overrides[0].reason, Some("Legacy code".to_string()));
}
