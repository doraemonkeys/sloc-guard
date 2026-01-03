//! Tests for config extends inheritance order (rule ordering, "last match wins" behavior).
//!
//! When using extends, arrays are appended (parent + child), so child rules appear
//! AFTER parent rules. Combined with "last match wins" semantics, this means child
//! rules override parent rules when both match the same path.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;

use super::mock_fs::MockFileSystem;

#[test]
fn extends_preserves_rule_order_for_last_match_wins() {
    // Parent defines a broad rule, child defines a more specific override.
    // Since arrays append and "last match wins", child's rule should take precedence.
    let parent_content = r#"
version = "2"

[[content.rules]]
pattern = "src/**"
max_lines = 500
"#;
    let child_content = r#"
version = "2"
extends = "/parent.toml"

[[content.rules]]
pattern = "src/**"
max_lines = 1000
"#;

    let fs = MockFileSystem::new()
        .with_file("/parent.toml", parent_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Order should be: [parent rule, child rule]
    assert_eq!(result.config.content.rules.len(), 2);
    assert_eq!(result.config.content.rules[0].max_lines, 500); // Parent rule first
    assert_eq!(result.config.content.rules[1].max_lines, 1000); // Child rule second (wins)
}

#[test]
fn extends_three_level_chain_preserves_rule_order() {
    // grandparent -> parent -> child
    // Final order should be: [grandparent rules, parent rules, child rules]
    let grandparent_content = r#"
version = "2"

[[content.rules]]
pattern = "**/*.rs"
max_lines = 300
reason = "grandparent rule"
"#;
    let parent_content = r#"
version = "2"
extends = "/grandparent.toml"

[[content.rules]]
pattern = "src/**"
max_lines = 500
reason = "parent rule"
"#;
    let child_content = r#"
version = "2"
extends = "/parent.toml"

[[content.rules]]
pattern = "src/generated/**"
max_lines = 1000
reason = "child rule"
"#;

    let fs = MockFileSystem::new()
        .with_file("/grandparent.toml", grandparent_content)
        .with_file("/parent.toml", parent_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Order: grandparent, parent, child
    assert_eq!(result.config.content.rules.len(), 3);
    assert_eq!(
        result.config.content.rules[0].reason,
        Some("grandparent rule".to_string())
    );
    assert_eq!(
        result.config.content.rules[1].reason,
        Some("parent rule".to_string())
    );
    assert_eq!(
        result.config.content.rules[2].reason,
        Some("child rule".to_string())
    );

    // max_lines should reflect the order for "last match wins" behavior
    assert_eq!(result.config.content.rules[0].max_lines, 300);
    assert_eq!(result.config.content.rules[1].max_lines, 500);
    assert_eq!(result.config.content.rules[2].max_lines, 1000);
}

#[test]
fn extends_structure_rules_preserve_order() {
    // Same test but for structure.rules
    let parent_content = r#"
version = "2"

[[structure.rules]]
scope = "src/**"
max_files = 50
reason = "parent structure rule"
"#;
    let child_content = r#"
version = "2"
extends = "/parent.toml"

[[structure.rules]]
scope = "src/generated/**"
max_files = 200
reason = "child structure rule"
"#;

    let fs = MockFileSystem::new()
        .with_file("/parent.toml", parent_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Order: parent, child
    assert_eq!(result.config.structure.rules.len(), 2);
    assert_eq!(
        result.config.structure.rules[0].reason,
        Some("parent structure rule".to_string())
    );
    assert_eq!(
        result.config.structure.rules[1].reason,
        Some("child structure rule".to_string())
    );
}

#[test]
fn extends_mixed_content_and_structure_rules_preserve_order() {
    // Parent has both content and structure rules
    // Child adds more rules to both
    let parent_content = r#"
version = "2"

[[content.rules]]
pattern = "**/*.rs"
max_lines = 400

[[structure.rules]]
scope = "src/**"
max_files = 30
"#;
    let child_content = r#"
version = "2"
extends = "/parent.toml"

[[content.rules]]
pattern = "tests/**"
max_lines = 600

[[structure.rules]]
scope = "tests/**"
max_files = 50
"#;

    let fs = MockFileSystem::new()
        .with_file("/parent.toml", parent_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Content rules: parent first, then child
    assert_eq!(result.config.content.rules.len(), 2);
    assert_eq!(result.config.content.rules[0].pattern, "**/*.rs");
    assert_eq!(result.config.content.rules[1].pattern, "tests/**");

    // Structure rules: parent first, then child
    assert_eq!(result.config.structure.rules.len(), 2);
    assert_eq!(result.config.structure.rules[0].scope, "src/**");
    assert_eq!(result.config.structure.rules[1].scope, "tests/**");
}

#[test]
fn extends_child_can_override_with_same_pattern() {
    // Parent defines rule for "src/**" with max_lines=500
    // Child redefines "src/**" with max_lines=800
    // Both rules should exist, with child's rule appearing last (and winning)
    let parent_content = r#"
version = "2"

[[content.rules]]
pattern = "src/**"
max_lines = 500
"#;
    let child_content = r#"
version = "2"
extends = "/parent.toml"

[[content.rules]]
pattern = "src/**"
max_lines = 800
"#;

    let fs = MockFileSystem::new()
        .with_file("/parent.toml", parent_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Both rules exist
    assert_eq!(result.config.content.rules.len(), 2);
    // Parent rule first with 500
    assert_eq!(result.config.content.rules[0].pattern, "src/**");
    assert_eq!(result.config.content.rules[0].max_lines, 500);
    // Child rule second with 800 (this will be used due to "last match wins")
    assert_eq!(result.config.content.rules[1].pattern, "src/**");
    assert_eq!(result.config.content.rules[1].max_lines, 800);
}

#[test]
fn extends_reset_then_add_preserves_child_order() {
    // Parent has rules [A, B], child uses $reset then adds [C, D]
    // Final should be [C, D] only, in that order
    let parent_content = r#"
version = "2"

[[content.rules]]
pattern = "**/*.rs"
max_lines = 300
reason = "A"

[[content.rules]]
pattern = "**/*.go"
max_lines = 400
reason = "B"
"#;
    let child_content = r#"
version = "2"
extends = "/parent.toml"

[[content.rules]]
pattern = "$reset"
max_lines = 0

[[content.rules]]
pattern = "src/**"
max_lines = 500
reason = "C"

[[content.rules]]
pattern = "tests/**"
max_lines = 600
reason = "D"
"#;

    let fs = MockFileSystem::new()
        .with_file("/parent.toml", parent_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Only child rules remain after $reset
    assert_eq!(result.config.content.rules.len(), 2);
    assert_eq!(result.config.content.rules[0].reason, Some("C".to_string()));
    assert_eq!(result.config.content.rules[1].reason, Some("D".to_string()));
}
