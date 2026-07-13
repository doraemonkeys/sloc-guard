//! Basic `scan_with_structure` tests: file collection, stats, depth, filtering.

use tempfile::TempDir;

use super::*;
use crate::scanner::TestConfigParams;

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
    assert_eq!(result.dir_stats[&src_dir].files.len(), 2);
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

    assert_eq!(result.dir_stats[&src_dir].dirs.len(), 2);
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
            .is_none_or(|s| s.dirs.len() <= 1)
    );
}

#[test]
fn scan_with_structure_records_raw_child_inventory() {
    // The scanner is rule-agnostic: it records every child by name, and the
    // structure checker applies count_exclude at check time.
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let sub_dir = src_dir.join("sub");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("generated.txt"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    let src_stats = &result.dir_stats[&src_dir];
    let mut file_names = src_stats.files.clone();
    file_names.sort_unstable();
    assert_eq!(file_names, vec!["generated.txt", "main.rs"]);
    assert_eq!(src_stats.dirs, vec!["sub"]);
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
    assert!(result.dir_stats[&empty_dir].files.is_empty());
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
    // But dir_stats still records both files (filter doesn't affect the inventory)
    assert_eq!(result.dir_stats[&src_dir].files.len(), 2);
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

    assert_eq!(result.dir_stats[temp_dir.path()].dirs.len(), 3);
}

#[test]
fn scan_with_structure_handles_files_at_root() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("b.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.files.len(), 2);
    assert_eq!(result.dir_stats[temp_dir.path()].files.len(), 2);
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
fn count_excluded_file_is_exempt_from_quota_but_not_from_policy() {
    // A count-excluded file no longer skips allowlist/deny checks: it is
    // exempt from quotas, not invisible to policy.
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("test.gen"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["*.gen".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Both files in file list and in the raw inventory
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.dir_stats[&src_dir].files.len(), 2);
    // The denied file is reported even though count_exclude drops it from counts
    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("test.gen"));

    let structure_config = crate::config::StructureConfig {
        max_files: Some(1),
        count_exclude: vec!["*.gen".to_string()],
        ..Default::default()
    };
    let checker = crate::checker::StructureChecker::new(&structure_config).unwrap();
    let violations = checker.check(&result.dir_stats);
    assert!(violations.is_empty());
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
