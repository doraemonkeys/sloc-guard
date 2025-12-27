//! Dot-prefixed path scope matching tests (regression tests for scope matching bug).
//!
//! These tests verify that paths starting with "./" are normalized correctly
//! for scope pattern matching.
//!
//! Note: The core path normalization logic is tested in:
//! - `allowlist::strip_dot_prefix_tests` (unit tests for the strip function)
//! - `allowlist_tests::allowlist_rule_matches_directory_with_dot_prefix` (integration tests)
//!
//! The following tests verify the full end-to-end behavior but require
//! changing the current directory, which causes race conditions in parallel
//! test execution. They are run serially by using a mutex.

use std::path::Path;
use std::sync::Mutex;

use tempfile::TempDir;

use super::*;
use crate::scanner::TestConfigParams;

/// Global mutex to serialize tests that change current directory.
static CWD_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn scan_with_dot_prefix_detects_deny_dirs_violation() {
    // Regression test: When scanning from ".", the scope pattern should match
    // directories like "./src" against scope "{src,src/**}"

    // Acquire lock to prevent parallel execution with other cwd-changing tests
    let _lock = CWD_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();

    // Save current dir and change to temp dir
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create structure: src/stats (a denied directory)
    let src_dir = Path::new("src");
    let stats_dir = src_dir.join("stats");
    std::fs::create_dir_all(&stats_dir).unwrap();
    std::fs::write(stats_dir.join("mod.rs"), "").unwrap();

    // Rule: deny "stats" directory in src scope
    let allowlist_rule = AllowlistRuleBuilder::new("{src,src/**}".to_string())
        .with_deny_dirs(vec!["stats".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();

    // Scan from "." (the key part of the regression)
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(Path::new("."), Some(&config))
        .unwrap();

    // Restore original directory before any assertions to ensure cleanup on panic
    std::env::set_current_dir(&original_dir).unwrap();

    // Should detect the denied directory
    let denied_dirs: Vec<_> = result
        .allowlist_violations
        .iter()
        .filter(|v| {
            matches!(
                v.violation_type,
                crate::checker::ViolationType::DeniedDirectory { .. }
            )
        })
        .collect();

    assert_eq!(
        denied_dirs.len(),
        1,
        "Expected 1 denied directory violation for 'stats'. Got: {:?}",
        denied_dirs.iter().map(|v| &v.path).collect::<Vec<_>>()
    );
    assert!(
        denied_dirs[0].path.ends_with("stats"),
        "Expected violation path to end with 'stats', got: {:?}",
        denied_dirs[0].path
    );
}

#[test]
fn scan_with_dot_prefix_detects_deny_files_violation() {
    // Regression test: When scanning from ".", per-rule deny_files should work

    // Acquire lock to prevent parallel execution with other cwd-changing tests
    let _lock = CWD_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();

    // Save current dir and change to temp dir
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create structure: src/utils.rs (a denied file)
    let src_dir = Path::new("src");
    std::fs::create_dir_all(src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("utils.rs"), "").unwrap(); // This should be denied

    // Rule: deny "utils.rs" file in src scope
    let allowlist_rule = AllowlistRuleBuilder::new("{src,src/**}".to_string())
        .with_deny_files(vec!["utils.rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();

    // Scan from "." (the key part of the regression)
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(Path::new("."), Some(&config))
        .unwrap();

    // Restore original directory before any assertions to ensure cleanup on panic
    std::env::set_current_dir(&original_dir).unwrap();

    // Should detect the denied file
    let denied_files: Vec<_> = result
        .allowlist_violations
        .iter()
        .filter(|v| {
            matches!(
                v.violation_type,
                crate::checker::ViolationType::DeniedFile { .. }
            )
        })
        .collect();

    assert_eq!(
        denied_files.len(),
        1,
        "Expected 1 denied file violation for 'utils.rs'. Got: {:?}",
        denied_files.iter().map(|v| &v.path).collect::<Vec<_>>()
    );
    assert!(
        denied_files[0].path.ends_with("utils.rs"),
        "Expected violation path to end with 'utils.rs', got: {:?}",
        denied_files[0].path
    );
}

#[test]
fn scan_from_subdirectory_still_detects_violations() {
    // Test that scanning from a subdirectory (e.g., "src") still works correctly
    // This test uses **/src pattern to match absolute paths from temp directories
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let stats_dir = src_dir.join("stats");
    std::fs::create_dir_all(&stats_dir).unwrap();
    std::fs::write(stats_dir.join("mod.rs"), "").unwrap();

    // Rule: deny "stats" directory in any src scope (using **/ to match absolute paths)
    let allowlist_rule = AllowlistRuleBuilder::new("{**/src,**/src/**}".to_string())
        .with_deny_dirs(vec!["stats".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();

    // Scan from "src" directly (absolute path)
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(&src_dir, Some(&config))
        .unwrap();

    // Should detect the denied directory
    let denied_dirs: Vec<_> = result
        .allowlist_violations
        .iter()
        .filter(|v| {
            matches!(
                v.violation_type,
                crate::checker::ViolationType::DeniedDirectory { .. }
            )
        })
        .collect();

    assert_eq!(
        denied_dirs.len(),
        1,
        "Expected 1 denied directory violation for 'stats'. Got: {:?}",
        denied_dirs.iter().map(|v| &v.path).collect::<Vec<_>>()
    );
}

