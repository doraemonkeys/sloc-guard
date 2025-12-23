use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::checker::ViolationType;
use crate::scanner::TestConfigParams;

struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
}

// =============================================================================
// Global Deny Extensions & Patterns
// =============================================================================

#[test]
fn global_deny_extensions_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("script.exe"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_extensions: vec![".exe".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("script.exe"));
    matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedFile { .. }
    );
}

#[test]
fn global_deny_patterns_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("file.bak"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["*.bak".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("file.bak"));
}

#[test]
fn structure_scan_config_file_matches_global_deny_extension() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_extensions: vec![".exe".to_string(), ".dll".to_string()],
        ..Default::default()
    })
    .unwrap();

    assert!(
        config
            .file_matches_global_deny(Path::new("app.exe"))
            .is_some()
    );
    assert!(
        config
            .file_matches_global_deny(Path::new("lib.dll"))
            .is_some()
    );
    assert!(
        config
            .file_matches_global_deny(Path::new("main.rs"))
            .is_none()
    );
}

#[test]
fn structure_scan_config_file_matches_global_deny_pattern() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["*.bak".to_string(), "temp_*".to_string()],
        ..Default::default()
    })
    .unwrap();

    assert!(
        config
            .file_matches_global_deny(Path::new("backup.bak"))
            .is_some()
    );
    assert!(
        config
            .file_matches_global_deny(Path::new("temp_data.txt"))
            .is_some()
    );
    assert!(
        config
            .file_matches_global_deny(Path::new("main.rs"))
            .is_none()
    );
}

// =============================================================================
// Directory-Only Deny Patterns (trailing `/`)
// =============================================================================

#[test]
fn deny_pattern_with_trailing_slash_only_matches_directories() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let node_modules_dir = src_dir.join("node_modules");
    std::fs::create_dir(&node_modules_dir).unwrap();
    std::fs::write(node_modules_dir.join("package.json"), "").unwrap();

    std::fs::write(src_dir.join("node_modules_legacy"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["**/node_modules/".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("node_modules")
    );
    assert!(matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedDirectory { .. }
    ));
}

#[test]
fn deny_pattern_without_trailing_slash_only_matches_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    std::fs::write(src_dir.join("temp_data.txt"), "").unwrap();

    let temp_dir_child = src_dir.join("temp_backup");
    std::fs::create_dir(&temp_dir_child).unwrap();
    std::fs::write(temp_dir_child.join("file.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["temp_*".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("temp_data.txt")
    );
    assert!(matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedFile { .. }
    ));
}

#[test]
fn structure_scan_config_separates_dir_and_file_patterns() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec![
            "**/node_modules/".to_string(),
            "*.bak".to_string(),
            "**/mocks/".to_string(),
        ],
        ..Default::default()
    })
    .unwrap();

    assert_eq!(config.global_deny_dir_pattern_strs.len(), 2);
    assert!(
        config
            .global_deny_dir_pattern_strs
            .contains(&"**/node_modules/".to_string())
    );
    assert!(
        config
            .global_deny_dir_pattern_strs
            .contains(&"**/mocks/".to_string())
    );
}

#[test]
fn dir_matches_global_deny_returns_original_pattern() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["**/node_modules/".to_string()],
        ..Default::default()
    })
    .unwrap();

    let result = config.dir_matches_global_deny(Path::new("project/src/node_modules"));
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "**/node_modules/");

    let result = config.dir_matches_global_deny(Path::new("project/src/utils"));
    assert!(result.is_none());
}

#[test]
fn multiple_directory_only_deny_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let node_modules = src_dir.join("node_modules");
    std::fs::create_dir(&node_modules).unwrap();

    let mocks = src_dir.join("mocks");
    std::fs::create_dir(&mocks).unwrap();

    let valid_dir = src_dir.join("utils");
    std::fs::create_dir(&valid_dir).unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["**/node_modules/".to_string(), "**/mocks/".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 2);
    let violation_paths: Vec<_> = result
        .allowlist_violations
        .iter()
        .map(|v| v.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(violation_paths.contains(&"node_modules".to_string()));
    assert!(violation_paths.contains(&"mocks".to_string()));
}

#[test]
fn deny_pattern_trailing_slash_with_simple_name() {
    let temp_dir = TempDir::new().unwrap();

    let temp_stuff = temp_dir.path().join("temp_stuff");
    std::fs::create_dir(&temp_stuff).unwrap();
    std::fs::write(temp_stuff.join("data.txt"), "").unwrap();

    std::fs::write(temp_dir.path().join("temp_file.txt"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["temp_*/".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("temp_stuff"));
    assert!(matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedDirectory { pattern } if pattern == "temp_*/"
    ));
}

// =============================================================================
// Deny Dirs (directory basename matching)
// =============================================================================

#[test]
fn global_deny_dirs_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    let pycache = src_dir.join("__pycache__");
    std::fs::create_dir(&pycache).unwrap();
    std::fs::write(pycache.join("module.pyc"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_dirs: vec!["__pycache__".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("__pycache__"));
    assert!(matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedDirectory { .. }
    ));
}

#[test]
fn deny_dirs_only_matches_directories_not_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Create a subdirectory first
    let sub_dir = src_dir.join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();

    // Create a file with the same name as the deny pattern in the subdirectory
    std::fs::write(sub_dir.join("node_modules"), "fake file").unwrap();

    // Create a real directory with the same name in src
    let real_node_modules = src_dir.join("node_modules");
    std::fs::create_dir(&real_node_modules).unwrap();
    std::fs::write(real_node_modules.join("package.json"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_dirs: vec!["node_modules".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Only the directory should trigger a violation, not the file
    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("node_modules")
    );
    assert!(matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedDirectory { .. }
    ));
}

// =============================================================================
// Deny File Patterns (filename-only matching)
// =============================================================================

#[test]
fn global_deny_file_patterns_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("secrets.json"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_files: vec!["secrets.*".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("secrets.json")
    );
}

#[test]
fn deny_file_patterns_only_matches_filename_not_path() {
    let temp_dir = TempDir::new().unwrap();
    // Create a directory structure where the pattern could match a path component
    let secrets_dir = temp_dir.path().join("secrets");
    std::fs::create_dir_all(&secrets_dir).unwrap();
    std::fs::write(secrets_dir.join("config.json"), "").unwrap();

    // This should NOT match because "secrets.*" only matches filenames
    // and "config.json" doesn't match "secrets.*"
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_files: vec!["secrets.*".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // No violations because the file is config.json, not secrets.*
    assert!(result.allowlist_violations.is_empty());
}

#[test]
fn structure_scan_config_file_matches_global_deny_file_pattern() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_files: vec!["temp_*".to_string(), "secrets.*".to_string()],
        ..Default::default()
    })
    .unwrap();

    // Should match filename patterns
    assert!(
        config
            .file_matches_global_deny(Path::new("temp_cache.txt"))
            .is_some()
    );
    assert!(
        config
            .file_matches_global_deny(Path::new("secrets.json"))
            .is_some()
    );

    // Should NOT match because the pattern is only matched against filename
    // even if the full path contains a matching pattern
    assert!(
        config
            .file_matches_global_deny(Path::new("main.rs"))
            .is_none()
    );
}
