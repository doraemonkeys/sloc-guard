//! Global and per-rule allow mode tests for `scan_with_structure`.
//!
//! Tests for `global_allow_extensions`, `global_allow_files`, `global_allow_dirs`,
//! and per-rule `allow_dirs`/`allow_files` behavior during structure scanning.

use tempfile::TempDir;

use super::*;
use crate::scanner::TestConfigParams;

// =============================================================================
// Global Allow Extensions
// =============================================================================

#[test]
fn scan_with_structure_global_allow_extensions_permits_matching_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_extensions: vec![".rs".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // No violations - all files are .rs
    assert!(result.allowlist_violations.is_empty());
}

#[test]
fn scan_with_structure_global_allow_extensions_rejects_non_matching() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_extensions: vec![".rs".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // config.json violates global allowlist
    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("config.json"));
}

// =============================================================================
// Global Allow Files
// =============================================================================

#[test]
fn scan_with_structure_global_allow_files_permits_matching() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Makefile"), "").unwrap();
    std::fs::write(src_dir.join("README.md"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_files: vec!["Makefile".to_string(), "README*".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // No violations - all files match patterns
    assert!(result.allowlist_violations.is_empty());
}

// =============================================================================
// Global Allow Dirs
// =============================================================================

#[test]
fn scan_with_structure_global_allow_dirs_permits_matching() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_dirs: vec!["src".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // src directory is allowed
    assert!(
        !result
            .allowlist_violations
            .iter()
            .any(|v| v.path.ends_with("src"))
    );
}

#[test]
fn scan_with_structure_global_allow_dirs_rejects_non_matching() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let vendor_dir = temp_dir.path().join("vendor");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&vendor_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(vendor_dir.join("lib.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_allow_dirs: vec!["src".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // vendor directory violates allowlist
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.ends_with("vendor"))
    );
}

// =============================================================================
// Per-Rule Allow Dirs
// =============================================================================

#[test]
fn scan_with_structure_per_rule_allow_dirs_permits_matching() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let utils_dir = src_dir.join("utils");
    std::fs::create_dir_all(&utils_dir).unwrap();
    std::fs::write(utils_dir.join("helper.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_allow_dirs(vec!["utils".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // utils directory is allowed - no dir violations
    assert!(
        !result
            .allowlist_violations
            .iter()
            .any(|v| v.path.ends_with("utils"))
    );
}

#[test]
fn scan_with_structure_per_rule_allow_dirs_rejects_non_matching() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let utils_dir = src_dir.join("utils");
    let vendor_dir = src_dir.join("vendor");
    std::fs::create_dir_all(&utils_dir).unwrap();
    std::fs::create_dir_all(&vendor_dir).unwrap();
    std::fs::write(utils_dir.join("helper.rs"), "").unwrap();
    std::fs::write(vendor_dir.join("lib.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_allow_dirs(vec!["utils".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // vendor directory violates per-rule allowlist
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.ends_with("vendor"))
    );
}

// =============================================================================
// Per-Rule Allow Files
// =============================================================================

#[test]
fn scan_with_structure_per_rule_allow_files_works() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Makefile"), "").unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_allow_files(vec!["Makefile".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // config.json violates per-rule allowlist
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.ends_with("config.json"))
    );
}

