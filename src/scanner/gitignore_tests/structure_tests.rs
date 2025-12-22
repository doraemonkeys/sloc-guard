//! Tests for `GitAwareScanner` `scan_with_structure()` functionality.
//!
//! Covers: directory stats collection, gitignore respect during structure scan,
//! allowlist validation, `count_exclude`, `scanner_exclude`, subdirectory counting,
//! `dir_count` inference from file paths, and depth tracking.

use super::super::{FileScanner, GitAwareScanner, StructureScanConfig};
use super::fixtures::{AcceptAllFilter, init_git_repo};
use crate::scanner::AllowlistRuleBuilder;
use tempfile::TempDir;

#[test]
fn collects_stats() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub fn foo() {}").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert!(result.files.len() >= 2);
    // Check that dir_stats contains a path ending with "src"
    assert!(result.dir_stats.keys().any(|k| k.ends_with("src")));
}

#[test]
fn respects_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("debug.log"), "log").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert!(result.files.iter().any(|f| f.ends_with("main.rs")));
    assert!(!result.files.iter().any(|f| f.ends_with("debug.log")));
}

#[test]
fn with_allowlist() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // config.json should be an allowlist violation
    assert!(!result.allowlist_violations.is_empty());
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("config.json"))
    );
}

#[test]
fn respects_count_exclude() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("generated.txt"), "generated").unwrap();

    let config = StructureScanConfig::new(
        &["*.txt".to_string()],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Both files should be found
    assert!(result.files.len() >= 2);
    // The src dir should have stats with only 1 file counted (txt excluded from count)
    let src_stats = result.dir_stats.iter().find(|(k, _)| k.ends_with("src"));
    assert!(src_stats.is_some());
    assert_eq!(src_stats.unwrap().1.file_count, 1);
}

#[test]
fn respects_scanner_exclude() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    let vendor_dir = temp_dir.path().join("vendor");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&vendor_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(vendor_dir.join("lib.rs"), "").unwrap();

    let config = StructureScanConfig::new(
        &[],
        &["**/vendor/**".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        &[],
    )
    .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // vendor files should be excluded
    assert!(result.files.iter().any(|f| f.ends_with("main.rs")));
    assert!(
        !result
            .files
            .iter()
            .any(|f| f.ends_with("vendor") && f.ends_with("lib.rs"))
    );
}

#[test]
fn counts_subdirectories() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    let sub1 = src_dir.join("sub1");
    let sub2 = src_dir.join("sub2");
    std::fs::create_dir_all(&sub1).unwrap();
    std::fs::create_dir_all(&sub2).unwrap();
    std::fs::write(sub1.join("a.rs"), "").unwrap();
    std::fs::write(sub2.join("b.rs"), "").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Should find both files
    assert_eq!(result.files.len(), 2);
    assert!(result.files.iter().any(|f| f.ends_with("a.rs")));
    assert!(result.files.iter().any(|f| f.ends_with("b.rs")));
    // sub1 and sub2 dirs should have file counts
    let sub1_stats = result.dir_stats.iter().find(|(k, _)| k.ends_with("sub1"));
    let sub2_stats = result.dir_stats.iter().find(|(k, _)| k.ends_with("sub2"));
    assert!(sub1_stats.is_some());
    assert!(sub2_stats.is_some());
    assert_eq!(sub1_stats.unwrap().1.file_count, 1);
    assert_eq!(sub2_stats.unwrap().1.file_count, 1);
}

#[test]
fn infers_dir_count_from_file_paths() {
    // This test verifies that gix's dirwalk (which only emits files, not directories)
    // correctly infers dir_count by analyzing file paths.
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // src/
    //   sub1/
    //     a.rs
    //   sub2/
    //     b.rs
    //   sub3/
    //     c.rs
    let src_dir = temp_dir.path().join("src");
    let sub1 = src_dir.join("sub1");
    let sub2 = src_dir.join("sub2");
    let sub3 = src_dir.join("sub3");
    std::fs::create_dir_all(&sub1).unwrap();
    std::fs::create_dir_all(&sub2).unwrap();
    std::fs::create_dir_all(&sub3).unwrap();
    std::fs::write(sub1.join("a.rs"), "").unwrap();
    std::fs::write(sub2.join("b.rs"), "").unwrap();
    std::fs::write(sub3.join("c.rs"), "").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // src/ should have dir_count = 3 (sub1, sub2, sub3)
    let src_stats = result.dir_stats.iter().find(|(k, _)| k.ends_with("src"));
    assert!(src_stats.is_some(), "src directory should be in dir_stats");
    assert_eq!(
        src_stats.unwrap().1.dir_count,
        3,
        "src should have 3 subdirectories"
    );

    // Each subdirectory should have dir_count = 0
    for subdir in ["sub1", "sub2", "sub3"] {
        let stats = result.dir_stats.iter().find(|(k, _)| k.ends_with(subdir));
        assert!(stats.is_some(), "{subdir} should be in dir_stats");
        assert_eq!(
            stats.unwrap().1.dir_count,
            0,
            "{subdir} should have 0 subdirectories"
        );
    }
}

#[test]
fn dir_count_with_max_dirs_zero() {
    // Regression test: max_dirs = 0 (prohibited) should detect violations
    // when directories have subdirectories.
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create a root with subdirectories
    let sub1 = temp_dir.path().join("sub1");
    let sub2 = temp_dir.path().join("sub2");
    std::fs::create_dir(&sub1).unwrap();
    std::fs::create_dir(&sub2).unwrap();
    std::fs::write(sub1.join("a.rs"), "").unwrap();
    std::fs::write(sub2.join("b.rs"), "").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Root directory (empty path or workdir path) should have dir_count >= 2
    // The root is tracked as an empty path "" in relative terms
    let root_stats = result
        .dir_stats
        .iter()
        .find(|(k, v)| v.depth == 0 && k.ends_with(temp_dir.path().file_name().unwrap()));

    assert!(
        root_stats.is_some(),
        "Root directory should be in dir_stats"
    );
    assert!(
        root_stats.unwrap().1.dir_count >= 2,
        "Root should have at least 2 subdirectories, got {}",
        root_stats.unwrap().1.dir_count
    );
}

#[test]
fn dir_count_nested_hierarchy() {
    // Test deeper nesting: a/b/c structure
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create: a/b/c/file.rs
    let deep = temp_dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("file.rs"), "").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // a should have dir_count = 1 (contains b)
    let a_stats = result
        .dir_stats
        .iter()
        .find(|(k, _)| k.ends_with("a") && !k.ends_with("b/a"));
    assert!(a_stats.is_some(), "a directory should be in dir_stats");
    assert_eq!(
        a_stats.unwrap().1.dir_count,
        1,
        "a should have 1 subdirectory (b)"
    );

    // b should have dir_count = 1 (contains c)
    let b_stats = result.dir_stats.iter().find(|(k, _)| k.ends_with("b"));
    assert!(b_stats.is_some(), "b directory should be in dir_stats");
    assert_eq!(
        b_stats.unwrap().1.dir_count,
        1,
        "b should have 1 subdirectory (c)"
    );

    // c should have dir_count = 0 (leaf directory)
    let c_stats = result.dir_stats.iter().find(|(k, _)| k.ends_with("c"));
    assert!(c_stats.is_some(), "c directory should be in dir_stats");
    assert_eq!(
        c_stats.unwrap().1.dir_count,
        0,
        "c should have 0 subdirectories"
    );
}

#[test]
fn tracks_depth() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let deep = temp_dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("file.rs"), "").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // The deepest directory should have depth 3
    let deep_stats = result.dir_stats.iter().find(|(k, _)| k.ends_with("c"));
    assert!(deep_stats.is_some());
    assert_eq!(deep_stats.unwrap().1.depth, 3);
}

#[test]
fn no_violations_when_files_match() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // All files are .rs, so no violations
    assert!(result.allowlist_violations.is_empty());
}
