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
    // Verify chain has correct order: a.toml was visited first, then b.toml, then a.toml again
    match &err {
        SlocGuardError::CircularExtends { chain } => {
            assert_eq!(chain.len(), 3, "Expected chain of length 3: [a, b, a]");
            assert!(chain[0].contains("a.toml"), "First should be a.toml");
            assert!(chain[1].contains("b.toml"), "Second should be b.toml");
            assert!(
                chain[2].contains("a.toml"),
                "Third (cycle) should be a.toml"
            );
        }
        other => panic!("Expected CircularExtends error, got: {other:?}"),
    }
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
    // Verify chain has correct order: self.toml visited, then self.toml again
    match &err {
        SlocGuardError::CircularExtends { chain } => {
            assert_eq!(chain.len(), 2, "Expected chain of length 2: [self, self]");
            assert!(chain[0].contains("self.toml"), "First should be self.toml");
            assert!(
                chain[1].contains("self.toml"),
                "Second (cycle) should be self.toml"
            );
        }
        other => panic!("Expected CircularExtends error, got: {other:?}"),
    }
}

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

// =============================================================================
// Extends Inheritance Order Tests
// =============================================================================
// These tests verify that rule ordering is preserved correctly during inheritance.
// When using extends, arrays are appended (parent + child), so child rules appear
// AFTER parent rules. Combined with "last match wins" semantics, this means child
// rules override parent rules when both match the same path.

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

// =============================================================================
// Extends Depth Limit Tests
// =============================================================================
// These tests verify that deeply nested extends chains are rejected.

use crate::config::loader::MAX_EXTENDS_DEPTH;

#[test]
fn extends_respects_max_depth_limit() {
    // Create a chain of configs that exceeds MAX_EXTENDS_DEPTH
    let mut fs = MockFileSystem::new();

    // Build chain: 0 -> 1 -> 2 -> ... -> MAX_EXTENDS_DEPTH + 1
    for i in 0..=MAX_EXTENDS_DEPTH + 1 {
        let content = if i == MAX_EXTENDS_DEPTH + 1 {
            // Terminal config (no extends)
            r#"
version = "2"

[content]
max_lines = 100
"#
            .to_string()
        } else {
            // Config that extends the next one
            format!(
                r#"
version = "2"
extends = "/config_{}.toml"

[content]
max_lines = {}
"#,
                i + 1,
                (i + 1) * 100
            )
        };
        fs = fs.with_file(format!("/config_{i}.toml"), &content);
    }

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config_0.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    // Verify chain preserves traversal order: config_0 -> config_1 -> ... -> config_N
    match &err {
        SlocGuardError::ExtendsTooDeep { max, chain, .. } => {
            assert_eq!(*max, MAX_EXTENDS_DEPTH);
            // Chain should contain configs in order of traversal
            for (i, entry) in chain.iter().enumerate() {
                assert!(
                    entry.contains(&format!("config_{i}.toml")),
                    "Chain entry {i} should be config_{i}.toml, got: {entry}"
                );
            }
        }
        other => {
            panic!("Expected ExtendsTooDeep error with max={MAX_EXTENDS_DEPTH}, got: {other:?}")
        }
    }
}

#[test]
fn extends_chain_at_max_depth_succeeds() {
    // Create a chain of configs exactly at MAX_EXTENDS_DEPTH (should succeed)
    let mut fs = MockFileSystem::new();

    // Build chain: 0 -> 1 -> 2 -> ... -> MAX_EXTENDS_DEPTH
    // Depth 0 (initial) extends depth 1, which extends depth 2, etc.
    // Total chain length = MAX_EXTENDS_DEPTH levels (not exceeding limit)
    for i in 0..=MAX_EXTENDS_DEPTH {
        let content = if i == MAX_EXTENDS_DEPTH {
            // Terminal config (no extends)
            r#"
version = "2"

[content]
max_lines = 100
"#
            .to_string()
        } else {
            // Config that extends the next one
            format!(
                r#"
version = "2"
extends = "/config_{}.toml"

[content]
max_lines = {}
"#,
                i + 1,
                (i + 1) * 100
            )
        };
        fs = fs.with_file(format!("/config_{i}.toml"), &content);
    }

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config_0.toml"));

    // Should succeed - chain is exactly at limit, not exceeding
    assert!(result.is_ok());
    // The innermost config's max_lines should be overridden by outer configs
    assert_eq!(result.unwrap().config.content.max_lines, 100);
}

#[test]
fn extends_depth_limit_error_message_is_informative() {
    // Verify error message contains useful information
    let mut fs = MockFileSystem::new();

    for i in 0..=MAX_EXTENDS_DEPTH + 1 {
        let content = if i == MAX_EXTENDS_DEPTH + 1 {
            "version = \"2\"\n".to_string()
        } else {
            format!("version = \"2\"\nextends = \"/config_{}.toml\"\n", i + 1)
        };
        fs = fs.with_file(format!("/config_{i}.toml"), &content);
    }

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config_0.toml"));

    let err = result.unwrap_err();
    let msg = format!("{err}");

    // Should mention depth and the max value
    assert!(
        msg.contains("too deep") || msg.contains("exceeds maximum"),
        "Error should mention depth limit: {msg}"
    );
    assert!(
        msg.contains(&MAX_EXTENDS_DEPTH.to_string()),
        "Error should mention max depth value: {msg}"
    );
}

#[test]
fn extends_preset_does_not_count_toward_depth() {
    // Presets are terminal - they don't have their own extends,
    // so a chain ending in a preset should work even at the boundary
    let mut fs = MockFileSystem::new();

    // Build chain up to MAX_EXTENDS_DEPTH - 1, then the last one extends a preset
    for i in 0..MAX_EXTENDS_DEPTH {
        let content = if i == MAX_EXTENDS_DEPTH - 1 {
            // Last config extends a preset (terminal)
            r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 600
"#
            .to_string()
        } else {
            format!(
                r#"
version = "2"
extends = "/config_{}.toml"
"#,
                i + 1
            )
        };
        fs = fs.with_file(format!("/config_{i}.toml"), &content);
    }

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config_0.toml"));

    // Should succeed - preset is terminal
    assert!(result.is_ok());
    let load_result = result.unwrap();
    assert_eq!(load_result.preset_used, Some("rust-strict".to_string()));
}
