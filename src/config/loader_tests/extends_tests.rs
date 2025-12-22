//! Tests for config extends/inheritance functionality (chain loading, cycle detection, merging).

use std::path::{Path, PathBuf};

use crate::config::loader::ConfigLoader;
use crate::config::{Config, FileConfigLoader};
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

#[test]
fn extends_loads_base_config() {
    let base_content = r#"
[default]
max_lines = 300
extensions = ["rs", "go"]
"#;
    let child_content = r#"
extends = "/base/config.toml"

[default]
max_lines = 500
"#;

    let fs = MockFileSystem::new()
        .with_file("/base/config.toml", base_content)
        .with_file("/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/project/config.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 500);
    assert_eq!(config.default.extensions, vec!["rs", "go"]);
    assert!(config.extends.is_none());
}

#[test]
fn extends_with_relative_path() {
    let base_content = r"
[default]
max_lines = 200
";
    let child_content = r#"
extends = "../base/config.toml"

[default]
skip_comments = false
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/base/config.toml", base_content)
        .with_file("/configs/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/configs/project/config.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 200);
    assert!(!config.default.skip_comments);
}

#[test]
fn extends_chain_works() {
    let grandparent_content = r#"
[default]
max_lines = 100

[exclude]
patterns = ["**/vendor/**"]
"#;
    let parent_content = r#"
extends = "/configs/grandparent.toml"

[default]
max_lines = 200
"#;
    let child_content = r#"
extends = "/configs/parent.toml"

[default]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/grandparent.toml", grandparent_content)
        .with_file("/configs/parent.toml", parent_content)
        .with_file("/configs/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/configs/child.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 300);
    assert_eq!(config.exclude.patterns, vec!["**/vendor/**"]);
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

[default]
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
fn extends_merges_rules() {
    let base_content = r#"
[rules.rust]
extensions = ["rs"]
max_lines = 300

[rules.python]
extensions = ["py"]
max_lines = 400
"#;
    let child_content = r#"
extends = "/base.toml"

[rules.rust]
max_lines = 500

[rules.go]
extensions = ["go"]
max_lines = 600
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Child overrides max_lines but inherits extensions from base
    let rust_rule = config.rules.get("rust").unwrap();
    assert_eq!(rust_rule.max_lines, Some(500));
    assert_eq!(rust_rule.extensions, vec!["rs"]);

    // Python rule inherited entirely from base
    let python_rule = config.rules.get("python").unwrap();
    assert_eq!(python_rule.max_lines, Some(400));
    assert_eq!(python_rule.extensions, vec!["py"]);

    // Go rule defined only in child
    let go_rule = config.rules.get("go").unwrap();
    assert_eq!(go_rule.max_lines, Some(600));
    assert_eq!(go_rule.extensions, vec!["go"]);
}

#[test]
fn extends_error_on_missing_base() {
    let child_content = r#"
extends = "/nonexistent/base.toml"

[default]
max_lines = 100
"#;

    let fs = MockFileSystem::new().with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml"));

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SlocGuardError::FileRead { .. }
    ));
}

#[test]
fn load_without_extends_ignores_extends_field() {
    let base_content = r"
[default]
max_lines = 100
";
    let child_content = r#"
extends = "/base.toml"

[default]
max_lines = 200
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/project/.sloc-guard.toml", child_content)
        .with_current_dir("/project");

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_without_extends().unwrap();

    // Should have max_lines from child only, not merged with base
    assert_eq!(config.default.max_lines, 200);
    // Extends field should be preserved in the config
    assert_eq!(config.extends, Some("/base.toml".to_string()));
}

#[test]
fn load_from_path_without_extends_ignores_extends() {
    let base_content = r#"
[default]
max_lines = 100
extensions = ["rs", "go"]
"#;
    let child_content = r#"
extends = "/base.toml"

[default]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path_without_extends(Path::new("/child.toml"))
        .unwrap();

    // Should have only child's max_lines, not merged
    assert_eq!(config.default.max_lines, 300);
    // Extensions should be default (not from base)
    assert_eq!(
        config.default.extensions,
        Config::default().default.extensions
    );
    // Extends field should be preserved
    assert_eq!(config.extends, Some("/base.toml".to_string()));
}

#[test]
fn load_without_extends_falls_back_to_user_config() {
    let user_content = r#"
extends = "https://example.com/base.toml"

[default]
max_lines = 400
"#;

    let fs = MockFileSystem::new()
        .with_home_dir(Some(PathBuf::from("/home/user")))
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_without_extends().unwrap();

    assert_eq!(config.default.max_lines, 400);
    assert_eq!(
        config.extends,
        Some("https://example.com/base.toml".to_string())
    );
}
