use std::path::Path;

use super::*;
use crate::scanner::TestConfigParams;

#[test]
fn has_global_file_allowlist_with_extensions() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_extensions: vec![".rs".to_string(), ".toml".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.has_global_file_allowlist());
}

#[test]
fn has_global_file_allowlist_with_files() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_files: vec!["Cargo.toml".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.has_global_file_allowlist());
}

#[test]
fn has_global_file_allowlist_empty() {
    let config = StructureScanConfig::new(TestConfigParams::default()).unwrap();
    assert!(!config.has_global_file_allowlist());
}

#[test]
fn has_global_dir_allowlist_with_dirs() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_dirs: vec!["src".to_string(), "tests".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.has_global_dir_allowlist());
}

#[test]
fn has_global_dir_allowlist_empty() {
    let config = StructureScanConfig::new(TestConfigParams::default()).unwrap();
    assert!(!config.has_global_dir_allowlist());
}

#[test]
fn file_matches_global_allow_extension() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_extensions: vec![".rs".to_string(), ".toml".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.file_matches_global_allow(Path::new("main.rs")));
    assert!(config.file_matches_global_allow(Path::new("Cargo.toml")));
    assert!(!config.file_matches_global_allow(Path::new("readme.md")));
}

#[test]
fn file_matches_global_allow_pattern() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_files: vec!["Cargo.*".to_string(), "README*".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.file_matches_global_allow(Path::new("Cargo.toml")));
    assert!(config.file_matches_global_allow(Path::new("Cargo.lock")));
    assert!(config.file_matches_global_allow(Path::new("README.md")));
    assert!(!config.file_matches_global_allow(Path::new("main.rs")));
}

#[test]
fn dir_matches_global_allow() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_dirs: vec!["src".to_string(), "test*".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.dir_matches_global_allow(Path::new("src")));
    assert!(config.dir_matches_global_allow(Path::new("tests")));
    assert!(config.dir_matches_global_allow(Path::new("test_utils")));
    assert!(!config.dir_matches_global_allow(Path::new("vendor")));
}
