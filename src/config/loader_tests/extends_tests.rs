//! Tests for config extends/inheritance functionality (chain loading, cycle detection, merging).

use std::path::{Path, PathBuf};

use crate::config::loader::ConfigLoader;
use crate::config::{Config, FileConfigLoader};
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

#[test]
fn extends_loads_base_config() {
    let base_content = r#"
version = "2"

[content]
max_lines = 300
extensions = ["rs", "go"]
"#;
    let child_content = r#"
version = "2"
extends = "/base/config.toml"

[content]
max_lines = 500
"#;

    let fs = MockFileSystem::new()
        .with_file("/base/config.toml", base_content)
        .with_file("/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/project/config.toml"))
        .unwrap();

    assert_eq!(result.config.content.max_lines, 500);
    assert_eq!(result.config.content.extensions, vec!["rs", "go"]);
    assert!(result.config.extends.is_none());
}

#[test]
fn extends_with_relative_path() {
    let base_content = r#"
version = "2"

[content]
max_lines = 200
"#;
    let child_content = r#"
version = "2"
extends = "../base/config.toml"

[content]
skip_comments = false
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/base/config.toml", base_content)
        .with_file("/configs/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/configs/project/config.toml"))
        .unwrap();

    assert_eq!(result.config.content.max_lines, 200);
    assert!(!result.config.content.skip_comments);
}

#[test]
fn extends_chain_works() {
    let grandparent_content = r#"
version = "2"

[content]
max_lines = 100

[scanner]
exclude = ["**/vendor/**"]
"#;
    let parent_content = r#"
version = "2"
extends = "/configs/grandparent.toml"

[content]
max_lines = 200
"#;
    let child_content = r#"
version = "2"
extends = "/configs/parent.toml"

[content]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/grandparent.toml", grandparent_content)
        .with_file("/configs/parent.toml", parent_content)
        .with_file("/configs/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/configs/child.toml"))
        .unwrap();

    assert_eq!(result.config.content.max_lines, 300);
    assert_eq!(result.config.scanner.exclude, vec!["**/vendor/**"]);
}

#[test]
fn extends_detects_direct_cycle() {
    let config_a = r#"
extends = "/configs/b.toml"
"#;
    let config_b = r#"
extends = "/configs/a.toml"
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/a.toml", config_a)
        .with_file("/configs/b.toml", config_b);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/configs/a.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::Config(msg) if msg.contains("Circular")));
}

#[test]
fn extends_detects_self_reference() {
    let config = r#"
extends = "/configs/self.toml"

[content]
max_lines = 100
"#;

    let fs = MockFileSystem::new().with_file("/configs/self.toml", config);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/configs/self.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::Config(msg) if msg.contains("Circular")));
}

#[test]
fn extends_merges_content_rules() {
    let base_content = r#"
version = "2"

[[content.rules]]
pattern = "**/*.rs"
max_lines = 300

[[content.rules]]
pattern = "**/*.py"
max_lines = 400
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[[content.rules]]
pattern = "**/*.go"
max_lines = 600
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Child's rules override base's rules (arrays are replaced, not merged)
    assert_eq!(result.config.content.rules.len(), 1);
    assert_eq!(result.config.content.rules[0].pattern, "**/*.go");
    assert_eq!(result.config.content.rules[0].max_lines, 600);
}

#[test]
fn extends_error_on_missing_base() {
    let child_content = r#"
extends = "/nonexistent/base.toml"

[content]
max_lines = 100
"#;

    let fs = MockFileSystem::new().with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml"));

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SlocGuardError::FileAccess { .. }
    ));
}

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
        .with_home_dir(Some(PathBuf::from("/home/user")))
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_without_extends().unwrap();

    assert_eq!(result.config.content.max_lines, 400);
    assert_eq!(
        result.config.extends,
        Some("https://example.com/base.toml".to_string())
    );
}
