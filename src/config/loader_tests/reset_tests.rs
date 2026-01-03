//! Tests for config extends $reset marker functionality (clearing parent arrays).

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

#[test]
fn extends_appends_content_rules() {
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

    // Arrays are appended: parent + child
    assert_eq!(result.config.content.rules.len(), 3);
    assert_eq!(result.config.content.rules[0].pattern, "**/*.rs");
    assert_eq!(result.config.content.rules[1].pattern, "**/*.py");
    assert_eq!(result.config.content.rules[2].pattern, "**/*.go");
}

#[test]
fn extends_reset_clears_parent_rules() {
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
pattern = "$reset"
max_lines = 0

[[content.rules]]
pattern = "**/*.go"
max_lines = 600
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // $reset clears parent, only child rules remain (without the reset marker)
    assert_eq!(result.config.content.rules.len(), 1);
    assert_eq!(result.config.content.rules[0].pattern, "**/*.go");
    assert_eq!(result.config.content.rules[0].max_lines, 600);
}

#[test]
fn extends_reset_clears_scanner_exclude() {
    let base_content = r#"
version = "2"

[scanner]
exclude = ["**/vendor/**", "**/node_modules/**"]
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[scanner]
exclude = ["$reset", "**/build/**"]
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // $reset clears parent, only child patterns remain
    assert_eq!(result.config.scanner.exclude, vec!["**/build/**"]);
}

#[test]
fn extends_appends_scanner_exclude() {
    let base_content = r#"
version = "2"

[scanner]
exclude = ["**/vendor/**"]
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[scanner]
exclude = ["**/build/**"]
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Arrays are appended by default
    assert_eq!(
        result.config.scanner.exclude,
        vec!["**/vendor/**", "**/build/**"]
    );
}

#[test]
fn extends_reset_in_wrong_position_fails() {
    let base_content = r#"
version = "2"

[scanner]
exclude = ["**/vendor/**"]
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[scanner]
exclude = ["**/build/**", "$reset"]
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, SlocGuardError::Config(msg) if msg.contains("$reset") && msg.contains("first element"))
    );
}

#[test]
fn reset_stripped_when_no_extends() {
    // When a config has $reset but no parent, the marker should be stripped
    let content = r#"
version = "2"

[scanner]
exclude = ["$reset", "**/build/**"]
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/project/.sloc-guard.toml"))
        .unwrap();

    // $reset should be stripped, only actual patterns remain
    assert_eq!(result.config.scanner.exclude, vec!["**/build/**"]);
}

#[test]
fn extends_reset_clears_structure_rules() {
    let base_content = r#"
version = "2"

[[structure.rules]]
scope = "src/**"
max_files = 10
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[[structure.rules]]
scope = "$reset"

[[structure.rules]]
scope = "tests/**"
max_files = 20
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // $reset clears parent, only child rules remain
    assert_eq!(result.config.structure.rules.len(), 1);
    assert_eq!(result.config.structure.rules[0].scope, "tests/**");
    assert_eq!(result.config.structure.rules[0].max_files, Some(20));
}

#[test]
fn extends_appends_structure_rules() {
    let base_content = r#"
version = "2"

[[structure.rules]]
scope = "src/**"
max_files = 10
"#;
    let child_content = r#"
version = "2"
extends = "/base.toml"

[[structure.rules]]
scope = "tests/**"
max_files = 20
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Arrays are appended
    assert_eq!(result.config.structure.rules.len(), 2);
    assert_eq!(result.config.structure.rules[0].scope, "src/**");
    assert_eq!(result.config.structure.rules[1].scope, "tests/**");
}
