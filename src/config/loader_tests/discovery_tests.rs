//! Tests for config file discovery from various locations (current dir, user config, fallback).

use std::path::PathBuf;

use crate::config::loader::ConfigLoader;
use crate::config::{Config, FileConfigLoader};

use super::mock_fs::MockFileSystem;

#[test]
fn returns_default_when_no_config_found() {
    let fs = MockFileSystem::new();
    let loader = FileConfigLoader::with_fs(fs);

    let result = loader.load().unwrap();

    assert_eq!(result.config.content.max_lines, 600);
    assert!(result.config.content.skip_comments);
    assert!(result.config.content.skip_blank);
    assert!(result.preset_used.is_none());
}

#[test]
fn loads_local_config_from_current_directory() {
    let config_content = r#"
version = "2"

[content]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/my/project")
        .with_file("/my/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load().unwrap();

    assert_eq!(result.config.content.max_lines, 300);
}

#[test]
fn loads_user_config_as_fallback() {
    let config_content = r#"
version = "2"

[content]
max_lines = 400
"#;

    let fs = MockFileSystem::new()
        .with_config_dir(Some(PathBuf::from("/home/testuser/.config/sloc-guard")))
        .with_file(
            "/home/testuser/.config/sloc-guard/config.toml",
            config_content,
        );

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load().unwrap();

    assert_eq!(result.config.content.max_lines, 400);
}

#[test]
fn local_config_takes_priority_over_user_config() {
    let local_content = r#"
version = "2"

[content]
max_lines = 200
"#;
    let user_content = r#"
version = "2"

[content]
max_lines = 600
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/project")
        .with_config_dir(Some(PathBuf::from("/home/user/.config/sloc-guard")))
        .with_file("/project/.sloc-guard.toml", local_content)
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load().unwrap();

    assert_eq!(result.config.content.max_lines, 200);
}

#[test]
fn handles_missing_config_dir() {
    let fs = MockFileSystem::new().with_config_dir(None);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load().unwrap();

    assert_eq!(result.config, Config::default());
}

#[test]
fn default_loader_can_be_created() {
    let _loader = FileConfigLoader::new();
    let _loader_default = FileConfigLoader::default();
}
