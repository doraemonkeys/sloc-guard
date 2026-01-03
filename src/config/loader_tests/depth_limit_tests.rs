//! Tests for config extends depth limit enforcement (`MAX_EXTENDS_DEPTH`).

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::{ConfigLoader, MAX_EXTENDS_DEPTH};
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

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
    // Create a chain of configs where the deepest config reaches depth = MAX_EXTENDS_DEPTH.
    // Since the check is `depth > MAX_EXTENDS_DEPTH`, depth 10 is allowed (passes).
    let mut fs = MockFileSystem::new();

    // Build chain: config_0 (depth 0) -> config_1 (depth 1) -> ... -> config_10 (depth 10)
    // This creates 11 config files, with the terminal config at depth = MAX_EXTENDS_DEPTH.
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

    // Should succeed - the deepest config is at depth 10, which equals MAX_EXTENDS_DEPTH.
    // The check `depth > MAX_EXTENDS_DEPTH` passes because 10 > 10 is false.
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
