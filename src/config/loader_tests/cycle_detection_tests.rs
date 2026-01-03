//! Tests for config extends cycle detection (circular references, self-references).

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

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
