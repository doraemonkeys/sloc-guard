use std::path::Path;

use super::*;
use crate::checker::ViolationType;
use tempfile::TempDir;

struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
}

// =============================================================================
// Global Deny Pattern Tests
// =============================================================================

#[test]
fn global_deny_extensions_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("script.exe"), "").unwrap();

    let config =
        StructureScanConfig::new(&[], &[], Vec::new(), vec![".exe".to_string()], &[]).unwrap();
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

    let config =
        StructureScanConfig::new(&[], &[], Vec::new(), vec![], &["*.bak".to_string()]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("file.bak"));
}

#[test]
fn structure_scan_config_file_matches_global_deny_extension() {
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![".exe".to_string(), ".dll".to_string()],
        &[],
    )
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
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![],
        &["*.bak".to_string(), "temp_*".to_string()],
    )
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
// Per-Rule Deny Pattern Tests
// =============================================================================

#[test]
fn per_rule_deny_extensions_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.ts"), "").unwrap();
    std::fs::write(src_dir.join("legacy.js"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_extensions(vec![".js".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("legacy.js"));
}

#[test]
fn per_rule_deny_patterns_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("temp_cache.txt"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_patterns(vec!["temp_*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("temp_cache.txt")
    );
}

#[test]
fn deny_takes_precedence_over_allowlist() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("backup.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .with_deny_patterns(vec!["backup*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("backup.rs"));
    matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedFile { .. }
    );
}

#[test]
fn allowlist_rule_file_matches_deny_extension() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_extensions(vec![".exe".to_string(), ".dll".to_string()])
        .build()
        .unwrap();

    assert!(rule.file_matches_deny(Path::new("src/app.exe")).is_some());
    assert!(rule.file_matches_deny(Path::new("src/lib.dll")).is_some());
    assert!(rule.file_matches_deny(Path::new("src/main.rs")).is_none());
}

#[test]
fn allowlist_rule_file_matches_deny_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_patterns(vec!["*.bak".to_string(), "temp_*".to_string()])
        .build()
        .unwrap();

    assert!(rule.file_matches_deny(Path::new("src/file.bak")).is_some());
    assert!(
        rule.file_matches_deny(Path::new("src/temp_data.txt"))
            .is_some()
    );
    assert!(rule.file_matches_deny(Path::new("src/main.rs")).is_none());
}

// =============================================================================
// Directory-Only Deny Pattern Tests (trailing `/`)
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

    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![],
        &["**/node_modules/".to_string()],
    )
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

    let config =
        StructureScanConfig::new(&[], &[], Vec::new(), vec![], &["temp_*".to_string()]).unwrap();
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
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![],
        &[
            "**/node_modules/".to_string(),
            "*.bak".to_string(),
            "**/mocks/".to_string(),
        ],
    )
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
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![],
        &["**/node_modules/".to_string()],
    )
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

    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        vec![],
        &["**/node_modules/".to_string(), "**/mocks/".to_string()],
    )
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

    let config =
        StructureScanConfig::new(&[], &[], Vec::new(), vec![], &["temp_*/".to_string()]).unwrap();
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
