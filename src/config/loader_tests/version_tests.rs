//! Tests for config version field validation.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

#[test]
fn config_with_valid_version_loads_successfully() {
    let config_content = r#"
version = "2"

[content]
max_lines = 300
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert_eq!(config.version, Some("2".to_string()));
    assert_eq!(config.content.max_lines, 300);
}

#[test]
fn config_without_version_loads_successfully() {
    let config_content = r"
[content]
max_lines = 400
";

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert!(config.version.is_none());
    assert_eq!(config.content.max_lines, 400);
}

#[test]
fn config_with_unsupported_version_returns_error() {
    let config_content = r#"
version = "99"

[content]
max_lines = 300
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, SlocGuardError::Config(msg) if msg.contains("Unsupported config version"))
    );
}

#[test]
fn config_with_v1_version_returns_error() {
    let config_content = r#"
version = "1"

[content]
max_lines = 300
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml"));

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Unsupported config version"));
    assert!(err_msg.contains("'1'")); // Should mention the unsupported version
}
