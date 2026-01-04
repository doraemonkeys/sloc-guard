//! Tests for source tracking functionality (`load_with_sources`, `load_from_path_with_sources`).
//!
//! These tests verify the source chain is correctly built during config loading,
//! enabling the `explain --sources` feature.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;

use super::mock_fs::MockFileSystem;

// ============================================================================
// Basic source tracking tests
// ============================================================================

#[test]
fn load_with_sources_no_config_returns_empty_chain() {
    // No config files exist
    let fs = MockFileSystem::new().with_config_dir(None);
    let loader = FileConfigLoader::with_fs(fs);

    let result = loader.load_with_sources().unwrap();

    assert!(result.source_chain.is_empty());
    assert!(result.preset_used.is_none());
}

#[test]
fn load_with_sources_local_config_only() {
    let config_content = r#"
version = "2"

[content]
max_lines = 400
"#;

    let fs = MockFileSystem::new()
        .with_file("/project/.sloc-guard.toml", config_content)
        .with_current_dir("/project");

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_with_sources().unwrap();

    assert_eq!(result.source_chain.len(), 1);
    assert!(
        result.source_chain[0]
            .source
            .to_string()
            .contains(".sloc-guard.toml")
    );
    assert_eq!(result.config.content.max_lines, 400);
    assert!(result.preset_used.is_none());
}

#[test]
fn load_with_sources_user_config_fallback() {
    let config_content = r#"
version = "2"

[content]
max_lines = 600
"#;

    let fs = MockFileSystem::new()
        .with_file("/home/user/.config/sloc-guard/config.toml", config_content)
        .with_config_dir(Some("/home/user/.config/sloc-guard".into()))
        .with_current_dir("/project");

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_with_sources().unwrap();

    assert_eq!(result.source_chain.len(), 1);
    assert!(
        result.source_chain[0]
            .source
            .to_string()
            .contains("config.toml")
    );
    assert_eq!(result.config.content.max_lines, 600);
}

#[test]
fn load_from_path_with_sources_single_file() {
    let config_content = r#"
version = "2"

[content]
max_lines = 350
extensions = ["rs", "go"]
"#;

    let fs = MockFileSystem::new().with_file("/project/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path_with_sources(Path::new("/project/config.toml"))
        .unwrap();

    assert_eq!(result.source_chain.len(), 1);
    assert!(
        result.source_chain[0]
            .source
            .to_string()
            .contains("config.toml")
    );
    assert_eq!(result.config.content.max_lines, 350);
    assert_eq!(result.config.content.extensions, vec!["rs", "go"]);
}

// ============================================================================
// Inheritance chain source tracking
// ============================================================================

#[test]
fn load_with_sources_extends_chain_tracks_all_sources() {
    let base_content = r#"
version = "2"

[content]
max_lines = 200
skip_comments = true
"#;

    let child_content = r#"
version = "2"
extends = "/base/config.toml"

[content]
max_lines = 400
"#;

    let fs = MockFileSystem::new()
        .with_file("/base/config.toml", base_content)
        .with_file("/project/.sloc-guard.toml", child_content)
        .with_current_dir("/project");

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_with_sources().unwrap();

    // Source chain should have both configs (base first, child second)
    assert_eq!(result.source_chain.len(), 2);
    assert!(
        result.source_chain[0]
            .source
            .to_string()
            .contains("/base/config.toml")
    );
    assert!(
        result.source_chain[1]
            .source
            .to_string()
            .contains(".sloc-guard.toml")
    );

    // Child overrides max_lines, inherits skip_comments
    assert_eq!(result.config.content.max_lines, 400);
    assert!(result.config.content.skip_comments);
}

#[test]
fn load_from_path_with_sources_extends_preserves_order() {
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
        .load_from_path_with_sources(Path::new("/configs/child.toml"))
        .unwrap();

    // Source chain: grandparent -> parent -> child (base to leaf order)
    assert_eq!(result.source_chain.len(), 3);
    assert!(
        result.source_chain[0]
            .source
            .to_string()
            .contains("grandparent.toml")
    );
    assert!(
        result.source_chain[1]
            .source
            .to_string()
            .contains("parent.toml")
    );
    assert!(
        result.source_chain[2]
            .source
            .to_string()
            .contains("child.toml")
    );

    // Final config has child's max_lines, grandparent's exclude
    assert_eq!(result.config.content.max_lines, 300);
    assert_eq!(result.config.scanner.exclude, vec!["**/vendor/**"]);
}

// ============================================================================
// Preset source tracking
// ============================================================================

#[test]
fn load_with_sources_preset_tracks_preset_source() {
    let child_content = r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 250
"#;

    let fs = MockFileSystem::new()
        .with_file("/project/.sloc-guard.toml", child_content)
        .with_current_dir("/project");

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_with_sources().unwrap();

    // Source chain should have preset first, then child
    assert_eq!(result.source_chain.len(), 2);
    assert!(
        result.source_chain[0]
            .source
            .to_string()
            .contains("rust-strict")
    );
    assert!(
        result.source_chain[1]
            .source
            .to_string()
            .contains(".sloc-guard.toml")
    );

    // Preset should be tracked
    assert_eq!(result.preset_used, Some("rust-strict".to_string()));
}

#[test]
fn load_from_path_with_sources_preset_chain() {
    let middle_content = r#"
version = "2"
extends = "preset:rust-strict"

[content]
skip_blank = true
"#;

    let child_content = r#"
version = "2"
extends = "/middle/config.toml"

[content]
max_lines = 500
"#;

    let fs = MockFileSystem::new()
        .with_file("/middle/config.toml", middle_content)
        .with_file("/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path_with_sources(Path::new("/project/config.toml"))
        .unwrap();

    // Source chain: preset -> middle -> child
    assert_eq!(result.source_chain.len(), 3);
    assert!(
        result.source_chain[0]
            .source
            .to_string()
            .contains("rust-strict")
    );
    assert!(result.source_chain[1].source.to_string().contains("middle"));
    assert!(
        result.source_chain[2]
            .source
            .to_string()
            .contains("project")
    );

    // Preset was used
    assert_eq!(result.preset_used, Some("rust-strict".to_string()));
}

// ============================================================================
// Source chain values
// ============================================================================

#[test]
fn source_chain_contains_raw_toml_values() {
    let config_content = r#"
version = "2"

[content]
max_lines = 123
extensions = ["py", "rb"]
"#;

    let fs = MockFileSystem::new().with_file("/project/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path_with_sources(Path::new("/project/config.toml"))
        .unwrap();

    // The source chain should have the raw TOML value
    let source = &result.source_chain[0];
    let content_table = source.value.get("content").unwrap();
    assert_eq!(
        content_table.get("max_lines").unwrap().as_integer(),
        Some(123)
    );

    let extensions = content_table.get("extensions").unwrap().as_array().unwrap();
    assert_eq!(extensions.len(), 2);
}

#[test]
fn source_chain_each_source_has_own_values() {
    let base_content = r#"
version = "2"

[content]
max_lines = 100
skip_comments = true
"#;

    let child_content = r#"
version = "2"
extends = "/base/config.toml"

[content]
max_lines = 200
"#;

    let fs = MockFileSystem::new()
        .with_file("/base/config.toml", base_content)
        .with_file("/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path_with_sources(Path::new("/project/config.toml"))
        .unwrap();

    // Base should have its original values
    let base = &result.source_chain[0];
    let base_content_table = base.value.get("content").unwrap();
    assert_eq!(
        base_content_table.get("max_lines").unwrap().as_integer(),
        Some(100)
    );
    assert_eq!(
        base_content_table.get("skip_comments").unwrap().as_bool(),
        Some(true)
    );

    // Child should have its original values (not merged)
    let child = &result.source_chain[1];
    let child_content_table = child.value.get("content").unwrap();
    assert_eq!(
        child_content_table.get("max_lines").unwrap().as_integer(),
        Some(200)
    );
    // Child doesn't have skip_comments in its raw value
    assert!(child_content_table.get("skip_comments").is_none());

    // But the merged config has both
    assert_eq!(result.config.content.max_lines, 200);
    assert!(result.config.content.skip_comments);
}

// ============================================================================
// Error cases
// ============================================================================

#[test]
fn load_from_path_with_sources_missing_file_returns_error() {
    let fs = MockFileSystem::new();
    let loader = FileConfigLoader::with_fs(fs);

    let result = loader.load_from_path_with_sources(Path::new("/nonexistent/config.toml"));

    assert!(result.is_err());
}

#[test]
fn load_from_path_with_sources_cycle_detection() {
    let a_content = r#"
version = "2"
extends = "/b.toml"

[content]
max_lines = 100
"#;

    let b_content = r#"
version = "2"
extends = "/a.toml"

[content]
max_lines = 200
"#;

    let fs = MockFileSystem::new()
        .with_file("/a.toml", a_content)
        .with_file("/b.toml", b_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path_with_sources(Path::new("/a.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    // Use match to verify it's a CircularExtends error
    assert!(
        matches!(&err, crate::error::SlocGuardError::CircularExtends { .. }),
        "Expected CircularExtends error, got: {err:?}"
    );
}

#[test]
fn load_from_path_with_sources_depth_limit() {
    // Create a chain that exceeds depth limit
    let mut fs = MockFileSystem::new();

    // Create 12 configs in a chain (exceeds MAX_EXTENDS_DEPTH of 10)
    for i in 0..12 {
        let content = if i == 0 {
            r#"
version = "2"

[content]
max_lines = 100
"#
            .to_string()
        } else {
            format!(
                r#"
version = "2"
extends = "/config_{}.toml"

[content]
max_lines = {}
"#,
                i - 1,
                100 + i
            )
        };
        fs = fs.with_file(format!("/config_{i}.toml"), &content);
    }

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path_with_sources(Path::new("/config_11.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("depth") || err.to_string().contains("deep"));
}

// ============================================================================
// Relative path resolution in source tracking
// ============================================================================

#[test]
fn load_from_path_with_sources_relative_extends() {
    let base_content = r#"
version = "2"

[content]
max_lines = 150
"#;

    let child_content = r#"
version = "2"
extends = "../base/config.toml"

[content]
skip_blank = false
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/base/config.toml", base_content)
        .with_file("/configs/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path_with_sources(Path::new("/configs/project/config.toml"))
        .unwrap();

    // Should have both sources
    assert_eq!(result.source_chain.len(), 2);
    assert!(result.source_chain[0].source.to_string().contains("base"));
    assert!(
        result.source_chain[1]
            .source
            .to_string()
            .contains("project")
    );

    // Values should be merged
    assert_eq!(result.config.content.max_lines, 150);
    assert!(!result.config.content.skip_blank);
}

// ============================================================================
// Remote config source tracking
// ============================================================================

/// Integration test for remote config source tracking is not included because:
/// - The loader uses `fetch_remote_config()` directly without HTTP client injection
/// - Adding DI for HTTP clients would require significant architecture changes
/// - The source tracking logic for remote configs follows the same pattern as
///   file/preset tracking, which is thoroughly tested above
///
/// The `remote_tests/` module tests HTTP fetching, caching, and policies.
/// This structural test validates that `SourcedConfig` with `ConfigSource::Remote`
/// works correctly, ensuring the source chain can represent remote configs.

#[test]
fn sourced_config_remote_variant_works_in_chain() {
    use crate::config::SourcedConfig;
    use crate::error::ConfigSource;

    // Create a mock source chain simulating: preset -> remote -> local
    let preset_value: toml::Value = toml::from_str(
        r#"
version = "2"

[content]
max_lines = 100
"#,
    )
    .unwrap();

    let remote_value: toml::Value = toml::from_str(
        r#"
version = "2"

[content]
max_lines = 200
"#,
    )
    .unwrap();

    let local_value: toml::Value = toml::from_str(
        r#"
version = "2"

[content]
max_lines = 300
"#,
    )
    .unwrap();

    let source_chain = [
        SourcedConfig {
            source: ConfigSource::preset("rust-strict"),
            value: preset_value,
        },
        SourcedConfig {
            source: ConfigSource::remote("https://example.com/base-config.toml"),
            value: remote_value,
        },
        SourcedConfig {
            source: ConfigSource::file("/project/.sloc-guard.toml"),
            value: local_value,
        },
    ];

    // Verify chain ordering and source types
    assert_eq!(source_chain.len(), 3);
    assert!(source_chain[0].source.to_string().contains("rust-strict"));
    assert!(
        source_chain[1]
            .source
            .to_string()
            .contains("https://example.com/base-config.toml")
    );
    assert!(
        source_chain[2]
            .source
            .to_string()
            .contains(".sloc-guard.toml")
    );

    // Verify values are accessible
    assert_eq!(
        source_chain[1]
            .value
            .get("content")
            .and_then(|c| c.get("max_lines"))
            .and_then(toml::Value::as_integer),
        Some(200)
    );
}
