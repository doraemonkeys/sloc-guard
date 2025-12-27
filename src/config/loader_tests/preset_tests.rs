//! Tests for preset:* extends functionality.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::config::presets;

use super::mock_fs::MockFileSystem;

#[test]
fn extends_preset_loads_rust_strict() {
    let config_content = r#"
extends = "preset:rust-strict"
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert_eq!(result.config.content.max_lines, 600);
    assert!(result.config.content.extensions.contains(&"rs".to_string()));
    assert!(
        result
            .config
            .scanner
            .exclude
            .iter()
            .any(|p| p == "target/**")
    );
    // Verify preset_used is populated
    assert_eq!(result.preset_used, Some("rust-strict".to_string()));
}

#[test]
fn extends_preset_child_overrides_preset_values() {
    let config_content = r#"
extends = "preset:rust-strict"

[content]
max_lines = 800
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Child config overrides preset's max_lines
    assert_eq!(result.config.content.max_lines, 800);
    // Preset's other values are preserved
    assert!(result.config.content.extensions.contains(&"rs".to_string()));
    assert!(
        result
            .config
            .scanner
            .exclude
            .iter()
            .any(|p| p == "target/**")
    );
    // Preset used should still be tracked
    assert_eq!(result.preset_used, Some("rust-strict".to_string()));
}

#[test]
fn extends_preset_child_adds_rules() {
    let config_content = r#"
extends = "preset:node-strict"

[[content.rules]]
pattern = "src/legacy/**"
max_lines = 1000
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Child adds rules on top of preset
    assert!(
        result
            .config
            .content
            .rules
            .iter()
            .any(|r| r.pattern == "src/legacy/**")
    );
    assert_eq!(result.config.content.max_lines, 600); // From node-strict
    assert_eq!(result.preset_used, Some("node-strict".to_string()));
}

#[test]
fn extends_unknown_preset_returns_error() {
    let config_content = r#"
extends = "preset:nonexistent"
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml"));

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Unknown preset"));
    assert!(err_msg.contains("nonexistent"));
}

#[test]
fn extends_preset_all_available_presets_work() {
    for preset_name in presets::AVAILABLE_PRESETS {
        let config_content = format!(r#"extends = "preset:{preset_name}""#);

        let fs = MockFileSystem::new().with_file("/config.toml", &config_content);
        let loader = FileConfigLoader::with_fs(fs);
        let result = loader.load_from_path(Path::new("/config.toml"));

        assert!(
            result.is_ok(),
            "Failed to load preset '{preset_name}' through extends: {:?}",
            result.err()
        );
    }
}

#[test]
fn all_presets_exclude_git_directory() {
    // All presets must exclude .git/** to prevent structure violations on git internals.
    // Without this, directories like .git/objects (with many subdirectories) would trigger
    // structure violations even when gitignore = true.
    for preset_name in presets::AVAILABLE_PRESETS {
        let config_content = format!(r#"extends = "preset:{preset_name}""#);

        let fs = MockFileSystem::new().with_file("/config.toml", &config_content);
        let loader = FileConfigLoader::with_fs(fs);
        let result = loader.load_from_path(Path::new("/config.toml")).unwrap();

        let has_git_exclude = result
            .config
            .scanner
            .exclude
            .iter()
            .any(|p| p.contains(".git"));

        assert!(
            has_git_exclude,
            "Preset '{preset_name}' must have .git exclusion in scanner.exclude, got: {:?}",
            result.config.scanner.exclude
        );
    }
}
