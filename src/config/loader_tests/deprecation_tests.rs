//! Tests for deprecated config features detection and error handling.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;

use super::mock_fs::MockFileSystem;

#[test]
fn deprecated_path_rules_returns_error() {
    let config_content = r#"
[[path_rules]]
pattern = "src/generated/**"
max_lines = 1000
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml"));

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("path_rules"));
    assert!(err_msg.contains("no longer supported"));
    assert!(err_msg.contains("content.rules")); // Should mention the new format
}
