//! Tests for config extends chain loading (basic loading, relative paths, chain traversal).

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
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
