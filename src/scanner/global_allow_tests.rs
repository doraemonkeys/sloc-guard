use std::path::Path;

use super::*;

#[test]
fn has_global_file_allowlist_with_extensions() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![".rs".to_string(), ".toml".to_string()],
        &[],
        &[],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(config.has_global_file_allowlist());
}

#[test]
fn has_global_file_allowlist_with_files() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &["Cargo.toml".to_string()],
        &[],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(config.has_global_file_allowlist());
}

#[test]
fn has_global_file_allowlist_empty() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(!config.has_global_file_allowlist());
}

#[test]
fn has_global_dir_allowlist_with_dirs() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &["src".to_string(), "tests".to_string()],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(config.has_global_dir_allowlist());
}

#[test]
fn has_global_dir_allowlist_empty() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(!config.has_global_dir_allowlist());
}

#[test]
fn file_matches_global_allow_extension() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![".rs".to_string(), ".toml".to_string()],
        &[],
        &[],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(config.file_matches_global_allow(Path::new("main.rs")));
    assert!(config.file_matches_global_allow(Path::new("Cargo.toml")));
    assert!(!config.file_matches_global_allow(Path::new("readme.md")));
}

#[test]
fn file_matches_global_allow_pattern() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &["Cargo.*".to_string(), "README*".to_string()],
        &[],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(config.file_matches_global_allow(Path::new("Cargo.toml")));
    assert!(config.file_matches_global_allow(Path::new("Cargo.lock")));
    assert!(config.file_matches_global_allow(Path::new("README.md")));
    assert!(!config.file_matches_global_allow(Path::new("main.rs")));
}

#[test]
fn dir_matches_global_allow() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &["src".to_string(), "test*".to_string()],
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(config.dir_matches_global_allow(Path::new("src")));
    assert!(config.dir_matches_global_allow(Path::new("tests")));
    assert!(config.dir_matches_global_allow(Path::new("test_utils")));
    assert!(!config.dir_matches_global_allow(Path::new("vendor")));
}
