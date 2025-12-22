use std::collections::HashMap;
use std::path::PathBuf;

use crate::checker::DirStats;
use crate::config::{ContentOverride, StructureOverride};
use crate::path_utils::path_matches_override;

use super::validate_override_paths;

#[test]
fn path_matches_override_exact_match() {
    let path = PathBuf::from("src/main.rs");
    assert!(path_matches_override(&path, "src/main.rs"));
    assert!(path_matches_override(&path, "main.rs"));
}

#[test]
fn path_matches_override_suffix_match() {
    let path = PathBuf::from("project/src/components/button.rs");
    assert!(path_matches_override(&path, "button.rs"));
    assert!(path_matches_override(&path, "components/button.rs"));
    assert!(path_matches_override(&path, "src/components/button.rs"));
    assert!(path_matches_override(
        &path,
        "project/src/components/button.rs"
    ));
}

#[test]
fn path_matches_override_no_match() {
    let path = PathBuf::from("src/main.rs");
    assert!(!path_matches_override(&path, "other.rs"));
    assert!(!path_matches_override(&path, "src/other.rs"));
    assert!(!path_matches_override(&path, "deep/nested/src/main.rs"));
}

#[test]
fn path_matches_override_partial_component_no_match() {
    let path = PathBuf::from("src/main.rs");
    // Should not match partial component names
    assert!(!path_matches_override(&path, "ain.rs"));
    assert!(!path_matches_override(&path, "rc/main.rs"));
}

#[test]
fn validate_override_paths_valid_content_override() {
    let content_overrides = vec![ContentOverride {
        path: "src/main.rs".to_string(),
        max_lines: 1000,
        reason: "Legacy file".to_string(),
    }];
    let structure_overrides: Vec<StructureOverride> = vec![];
    let files = vec![PathBuf::from("src/main.rs")];
    let directories: HashMap<PathBuf, DirStats> = HashMap::new();

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_ok());
}

#[test]
fn validate_override_paths_content_override_matches_directory() {
    let content_overrides = vec![ContentOverride {
        path: "src/components".to_string(),
        max_lines: 1000,
        reason: "Legacy file".to_string(),
    }];
    let structure_overrides: Vec<StructureOverride> = vec![];
    let files = vec![PathBuf::from("src/main.rs")];
    let mut directories: HashMap<PathBuf, DirStats> = HashMap::new();
    directories.insert(
        PathBuf::from("src/components"),
        DirStats {
            file_count: 5,
            dir_count: 0,
            depth: 1,
        },
    );

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("content.override[0]"));
    assert!(err.contains("matches directory"));
    assert!(err.contains("content overrides only apply to files"));
}

#[test]
fn validate_override_paths_valid_structure_override() {
    let content_overrides: Vec<ContentOverride> = vec![];
    let structure_overrides = vec![StructureOverride {
        path: "src/components".to_string(),
        max_files: Some(100),
        max_dirs: None,
        max_depth: None,
        reason: "Large component directory".to_string(),
    }];
    let files = vec![PathBuf::from("src/main.rs")];
    let mut directories: HashMap<PathBuf, DirStats> = HashMap::new();
    directories.insert(
        PathBuf::from("src/components"),
        DirStats {
            file_count: 50,
            dir_count: 2,
            depth: 1,
        },
    );

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_ok());
}

#[test]
fn validate_override_paths_structure_override_matches_file() {
    let content_overrides: Vec<ContentOverride> = vec![];
    let structure_overrides = vec![StructureOverride {
        path: "src/main.rs".to_string(),
        max_files: Some(100),
        max_dirs: None,
        max_depth: None,
        reason: "Misconfig".to_string(),
    }];
    let files = vec![PathBuf::from("src/main.rs")];
    let directories: HashMap<PathBuf, DirStats> = HashMap::new();

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("structure.override[0]"));
    assert!(err.contains("matches file"));
    assert!(err.contains("structure overrides only apply to directories"));
}

#[test]
fn validate_override_paths_suffix_matching() {
    // ContentOverride path "legacy" should match directory "project/src/legacy"
    let content_overrides = vec![ContentOverride {
        path: "legacy".to_string(),
        max_lines: 1000,
        reason: "Legacy".to_string(),
    }];
    let structure_overrides: Vec<StructureOverride> = vec![];
    let files: Vec<PathBuf> = vec![];
    let mut directories: HashMap<PathBuf, DirStats> = HashMap::new();
    directories.insert(
        PathBuf::from("project/src/legacy"),
        DirStats {
            file_count: 10,
            dir_count: 0,
            depth: 2,
        },
    );

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("legacy"));
}

