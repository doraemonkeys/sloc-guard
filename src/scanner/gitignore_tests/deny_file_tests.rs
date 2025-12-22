//! Tests for `deny_file_patterns` functionality in `GitAwareScanner`.
//!
//! Covers: relative path matching, nested directories, global deny patterns,
//! glob patterns for file denial, and non-matching files.

use super::super::{FileScanner, GitAwareScanner, StructureScanConfig};
use super::fixtures::{AcceptAllFilter, init_git_repo};
use crate::scanner::AllowlistRuleBuilder;
use tempfile::TempDir;

#[test]
fn deny_file_patterns_with_relative_pattern() {
    // Test that deny_file_patterns works with patterns like "src/**" (not "**/src")
    // This tests the fix where relative paths must be used for pattern matching
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    let analyzer_dir = src_dir.join("analyzer");
    std::fs::create_dir_all(&analyzer_dir).unwrap();
    std::fs::write(analyzer_dir.join("mod.rs"), "").unwrap();
    std::fs::write(analyzer_dir.join("types.rs"), "").unwrap(); // Should be denied

    // Pattern "src/**" should match "src/analyzer" and deny "types.rs"
    let allowlist_rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_files(vec!["types.rs".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // types.rs should be a violation
    assert!(
        !result.allowlist_violations.is_empty(),
        "Expected violation for types.rs"
    );
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("types.rs")),
        "Expected types.rs to be denied"
    );
}

#[test]
fn deny_file_patterns_nested_directories() {
    // Test deny_file_patterns works in deeply nested directories
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let deep_dir = temp_dir.path().join("src").join("module").join("submodule");
    std::fs::create_dir_all(&deep_dir).unwrap();
    std::fs::write(deep_dir.join("mod.rs"), "").unwrap();
    std::fs::write(deep_dir.join("utils.rs"), "").unwrap(); // Should be denied

    let allowlist_rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_files(vec!["utils.rs".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("utils.rs")),
        "Expected utils.rs to be denied in nested directory"
    );
}

#[test]
fn global_deny_file_patterns() {
    // Test global deny_file_patterns (not tied to a specific rule)
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("secrets.json"), "").unwrap(); // Should be denied globally

    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &["secrets.json".to_string()],
        &[],
    )
    .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("secrets.json")),
        "Expected secrets.json to be globally denied"
    );
}

#[test]
fn deny_file_patterns_does_not_match_allowed_files() {
    // Ensure files not matching deny patterns are not flagged
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("mod.rs"), "").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_files(vec!["utils.rs".to_string(), "types.rs".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // No violations because mod.rs and lib.rs are not in deny list
    assert!(
        result.allowlist_violations.is_empty(),
        "Expected no violations for allowed files, got: {:?}",
        result.allowlist_violations
    );
}

#[test]
fn deny_file_patterns_with_glob() {
    // Test deny_file_patterns with glob patterns like "temp_*"
    // Note: Pattern "src/**" matches directories INSIDE src, not src itself.
    // So we need files in a subdirectory like src/module/
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let module_dir = temp_dir.path().join("src").join("module");
    std::fs::create_dir_all(&module_dir).unwrap();
    std::fs::write(module_dir.join("main.rs"), "").unwrap();
    std::fs::write(module_dir.join("temp_cache.txt"), "").unwrap(); // Should be denied
    std::fs::write(module_dir.join("temp_data.json"), "").unwrap(); // Should be denied

    let allowlist_rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_files(vec!["temp_*".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(
        result.allowlist_violations.len(),
        2,
        "Expected 2 violations for temp_* pattern"
    );
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("temp_cache.txt"))
    );
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("temp_data.json"))
    );
}
