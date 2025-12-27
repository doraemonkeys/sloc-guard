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
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert_eq!(config.content.max_lines, 600);
    assert!(config.content.extensions.contains(&"rs".to_string()));
    assert!(config.scanner.exclude.iter().any(|p| p == "target/**"));
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
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Child config overrides preset's max_lines
    assert_eq!(config.content.max_lines, 800);
    // Preset's other values are preserved
    assert!(config.content.extensions.contains(&"rs".to_string()));
    assert!(config.scanner.exclude.iter().any(|p| p == "target/**"));
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
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Child adds rules on top of preset
    assert!(
        config
            .content
            .rules
            .iter()
            .any(|r| r.pattern == "src/legacy/**")
    );
    assert_eq!(config.content.max_lines, 600); // From node-strict
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
