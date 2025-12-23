use std::path::Path;

use super::*;
use crate::scanner::TestConfigParams;
use tempfile::TempDir;

struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
}

struct RustOnlyFilter;

impl FileFilter for RustOnlyFilter {
    fn should_include(&self, path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "rs")
    }
}

// =============================================================================
// scan_with_structure Tests
// =============================================================================

#[test]
fn scan_with_structure_collects_files_and_stats() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub fn foo() {}").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.files.len(), 2);
    assert!(result.dir_stats.contains_key(&src_dir));
    assert_eq!(result.dir_stats[&src_dir].file_count, 2);
}

#[test]
fn scan_with_structure_counts_subdirectories() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let sub1 = src_dir.join("sub1");
    let sub2 = src_dir.join("sub2");
    std::fs::create_dir_all(&sub1).unwrap();
    std::fs::create_dir_all(&sub2).unwrap();
    std::fs::write(sub1.join("a.rs"), "").unwrap();
    std::fs::write(sub2.join("b.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.dir_stats[&src_dir].dir_count, 2);
}

#[test]
fn scan_with_structure_tracks_depth() {
    let temp_dir = TempDir::new().unwrap();
    let deep = temp_dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("file.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.dir_stats[&deep].depth, 3);
}

#[test]
fn scan_with_structure_respects_scanner_exclude() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(target_dir.join("build.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["**/target/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.files.len(), 1);
    assert!(
        result
            .dir_stats
            .get(temp_dir.path())
            .is_none_or(|s| s.dir_count <= 1)
    );
}

#[test]
fn scan_with_structure_respects_count_exclude() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("generated.txt"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        count_exclude_patterns: vec!["*.txt".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // txt files should be found but not counted
    assert_eq!(result.files.len(), 2);
    // Only .rs file is counted (txt is excluded from count)
    assert_eq!(result.dir_stats[&src_dir].file_count, 1);
}

#[test]
fn scan_with_structure_detects_allowlist_violations() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
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

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("config.json"));
}

#[test]
fn scan_with_structure_no_violation_for_matching_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
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

    assert!(result.allowlist_violations.is_empty());
}

#[test]
fn scan_with_structure_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path().join("empty");
    std::fs::create_dir(&empty_dir).unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert!(result.dir_stats.contains_key(&empty_dir));
    assert_eq!(result.dir_stats[&empty_dir].file_count, 0);
}

#[test]
fn scan_with_structure_filter_excludes_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("test.txt"), "").unwrap();

    let scanner = DirectoryScanner::new(RustOnlyFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Only .rs file in files list
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
    // But dir_stats still counts both files (filter doesn't affect counting)
    assert_eq!(result.dir_stats[&src_dir].file_count, 2);
}

#[test]
fn scan_with_structure_depth_zero_at_root() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("root.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert!(result.dir_stats.contains_key(temp_dir.path()));
    assert_eq!(result.dir_stats[temp_dir.path()].depth, 0);
}

#[test]
fn scan_with_structure_multiple_dirs_at_same_level() {
    let temp_dir = TempDir::new().unwrap();
    let dir1 = temp_dir.path().join("dir1");
    let dir2 = temp_dir.path().join("dir2");
    let dir3 = temp_dir.path().join("dir3");
    std::fs::create_dir(&dir1).unwrap();
    std::fs::create_dir(&dir2).unwrap();
    std::fs::create_dir(&dir3).unwrap();
    std::fs::write(dir1.join("a.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.dir_stats[temp_dir.path()].dir_count, 3);
}

#[test]
fn scan_with_structure_handles_files_at_root() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("b.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.files.len(), 2);
    assert_eq!(result.dir_stats[temp_dir.path()].file_count, 2);
}

#[test]
fn scan_all_with_structure_merges_results() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let result = scanner.scan_all_with_structure(&paths, None).unwrap();

    assert_eq!(result.files.len(), 2);
    assert!(result.dir_stats.contains_key(temp_dir1.path()));
    assert!(result.dir_stats.contains_key(temp_dir2.path()));
}

#[test]
fn scan_with_structure_nested_directory_depth() {
    let temp_dir = TempDir::new().unwrap();
    let deep = temp_dir.path().join("a").join("b").join("c").join("d");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("file.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    let deep_stats = result.dir_stats.get(&deep);
    assert!(deep_stats.is_some());
    assert_eq!(deep_stats.unwrap().depth, 4);
}

#[test]
fn scan_with_structure_with_count_exclude_does_not_affect_file_list() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("test.gen"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        count_exclude_patterns: vec!["*.gen".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Both files in file list
    assert_eq!(result.files.len(), 2);
    // Only .rs counted
    assert_eq!(result.dir_stats[&src_dir].file_count, 1);
}

#[test]
fn scan_with_structure_with_scanner_exclude_skips_entirely() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    let vendor = temp_dir.path().join("vendor");
    std::fs::create_dir(&vendor).unwrap();
    std::fs::write(vendor.join("lib.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["**/vendor/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // vendor files completely excluded
    assert_eq!(result.files.len(), 1);
    assert!(
        !result
            .files
            .iter()
            .any(|f| f.to_string_lossy().contains("vendor"))
    );
}

#[test]
fn scan_with_structure_allowlist_violation_includes_rule_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert_eq!(
        result.allowlist_violations[0].triggering_rule_pattern,
        Some("**/src".to_string())
    );
}

#[test]
fn scan_with_structure_dir_excluded_by_name_match() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir(&target_dir).unwrap();
    std::fs::write(target_dir.join("build.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["**/target/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
}

// =============================================================================
// Global Allow Mode Tests
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
