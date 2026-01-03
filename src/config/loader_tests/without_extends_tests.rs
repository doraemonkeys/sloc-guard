//! Tests for `load_without_extends` functionality (loading config without resolving extends).

use std::path::{Path, PathBuf};

use crate::config::loader::ConfigLoader;
use crate::config::{Config, FileConfigLoader};

use super::mock_fs::MockFileSystem;

#[test]
fn load_without_extends_ignores_extends_field() {
    let base_content = r#"
version = "2"

[content]
max_lines = 100
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[content]
max_lines = 200
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/project/.sloc-guard.toml", child_content)
        .with_current_dir("/project");

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_without_extends().unwrap();

    // Should have max_lines from child only, not merged with base
    assert_eq!(result.config.content.max_lines, 200);
    // Extends field should be preserved in the config
    assert_eq!(result.config.extends, Some("/base.toml".to_string()));
    // preset_used should be None when not resolving extends
    assert!(result.preset_used.is_none());
}

#[test]
fn load_from_path_without_extends_ignores_extends() {
    let base_content = r#"
version = "2"

[content]
max_lines = 100
extensions = ["rs", "go"]
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[content]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path_without_extends(Path::new("/child.toml"))
        .unwrap();

    // Should have only child's max_lines, not merged
    assert_eq!(result.config.content.max_lines, 300);
    // Extensions should be default (not from base)
    assert_eq!(
        result.config.content.extensions,
        Config::default().content.extensions
    );
    // Extends field should be preserved
    assert_eq!(result.config.extends, Some("/base.toml".to_string()));
}

#[test]
fn load_without_extends_falls_back_to_user_config() {
    let user_content = r#"
version = "2"
extends = "https://example.com/base.toml"

[content]
max_lines = 400
"#;

    let fs = MockFileSystem::new()
        .with_config_dir(Some(PathBuf::from("/home/user/.config/sloc-guard")))
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_without_extends().unwrap();

    assert_eq!(result.config.content.max_lines, 400);
    assert_eq!(
        result.config.extends,
        Some("https://example.com/base.toml".to_string())
    );
}
