//! Scanner exclude directory pruning tests.
//!
//! Tests for scanner.exclude behavior that prunes entire directory subtrees
//! rather than just filtering individual files.

use tempfile::TempDir;

use super::*;
use crate::config::ScannerConfig;
use crate::scanner::TestConfigParams;

/// Regression test: Default `ScannerConfig` must exclude .git/** to prevent
/// structure checks from failing on git internals (e.g., .git/objects with 253 subdirs).
///
/// This test verifies the fix for the bug where running `sloc-guard check` on a
/// project with default config would fail with errors like:
///   ✗ FAILED: ./.git/objects
///      Directories: 253 (limit: 10)
#[test]
fn default_scanner_config_excludes_git_directory() {
    // Verify the default config has .git/** in exclude patterns
    let default_scanner = ScannerConfig::default();
    assert!(
        default_scanner.exclude.contains(&".git/**".to_string()),
        "Default ScannerConfig must contain '.git/**' in exclude patterns"
    );

    // Now verify this actually works during scanning
    let temp_dir = TempDir::new().unwrap();

    // Create a realistic .git structure that would fail structure checks
    let git_dir = temp_dir.path().join(".git");
    let objects_dir = git_dir.join("objects");
    std::fs::create_dir_all(&objects_dir).unwrap();

    // Simulate git object directories (2-char hex prefixes)
    for i in 0..50 {
        let prefix = format!("{i:02x}");
        let obj_subdir = objects_dir.join(&prefix);
        std::fs::create_dir(&obj_subdir).unwrap();
        std::fs::write(obj_subdir.join("object_file"), "git object data").unwrap();
    }
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").unwrap();
    std::fs::write(git_dir.join("config"), "[core]").unwrap();

    // Create normal project structure
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();

    // Use the default scanner exclude patterns
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: default_scanner.exclude,
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // .git should be completely excluded - no files from it
    assert!(
        !result
            .files
            .iter()
            .any(|f| f.to_string_lossy().contains(".git")),
        "No files from .git should be in scan results"
    );

    // .git should not appear in dir_stats
    assert!(
        !result.dir_stats.contains_key(&git_dir),
        ".git directory should not be in dir_stats"
    );
    assert!(
        !result.dir_stats.contains_key(&objects_dir),
        ".git/objects directory should not be in dir_stats"
    );

    // Normal files should be found
    assert!(result.files.iter().any(|f| f.ends_with("main.rs")));
    assert!(result.files.iter().any(|f| f.ends_with("Cargo.toml")));

    // Root should only count src as subdirectory, not .git
    let root_stats = result.dir_stats.get(temp_dir.path());
    assert!(root_stats.is_some());
    assert_eq!(
        root_stats.unwrap().dir_count,
        1,
        "Root should only have 1 subdirectory (src), .git should be excluded"
    );
}

#[test]
fn scan_with_structure_scanner_exclude_prunes_directory_subtree() {
    // Regression test: scanner.exclude should skip traversal entirely, not just filter
    let temp_dir = TempDir::new().unwrap();

    // Create a deep nested structure inside excluded directory
    let git_dir = temp_dir.path().join(".git");
    let objects_dir = git_dir.join("objects");
    let deep_dir = objects_dir.join("aa").join("bb").join("cc");
    std::fs::create_dir_all(&deep_dir).unwrap();

    // Add many files to simulate .git/objects structure
    for i in 0..30 {
        std::fs::write(objects_dir.join(format!("obj{i}")), "").unwrap();
    }
    std::fs::write(deep_dir.join("deep_obj"), "").unwrap();

    // Also add a normal file outside .git
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec![".git/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Only main.rs should be found
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));

    // .git directory should NOT be in dir_stats (not traversed)
    assert!(!result.dir_stats.contains_key(&git_dir));
    assert!(!result.dir_stats.contains_key(&objects_dir));

    // Root dir should NOT count .git as a subdirectory
    let root_stats = result.dir_stats.get(temp_dir.path());
    assert!(root_stats.is_none() || root_stats.unwrap().dir_count == 0);
}

#[test]
fn scan_with_structure_scanner_exclude_dotgit_pattern() {
    // Test typical .git exclusion pattern used in real configs
    let temp_dir = TempDir::new().unwrap();

    let git_dir = temp_dir.path().join(".git");
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    std::fs::write(hooks_dir.join("pre-commit"), "#!/bin/sh").unwrap();
    std::fs::write(git_dir.join("config"), "[core]").unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").unwrap();

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec![".git/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Only src/main.rs found
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));

    // .git not in stats
    assert!(!result.dir_stats.contains_key(&git_dir));

    // src is in stats
    assert!(result.dir_stats.contains_key(&src_dir));
    assert_eq!(result.dir_stats[&src_dir].file_count, 1);
}

#[test]
fn scan_with_structure_scanner_exclude_multiple_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple excluded directories
    let git_dir = temp_dir.path().join(".git");
    let target_dir = temp_dir.path().join("target");
    let node_modules = temp_dir.path().join("node_modules");
    std::fs::create_dir(&git_dir).unwrap();
    std::fs::create_dir(&target_dir).unwrap();
    std::fs::create_dir(&node_modules).unwrap();

    std::fs::write(git_dir.join("config"), "").unwrap();
    std::fs::write(target_dir.join("debug.rs"), "").unwrap();
    std::fs::write(node_modules.join("pkg.js"), "").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec![
            ".git/**".to_string(),
            "target/**".to_string(),
            "node_modules/**".to_string(),
        ],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.files.len(), 1);
    assert!(!result.dir_stats.contains_key(&git_dir));
    assert!(!result.dir_stats.contains_key(&target_dir));
    assert!(!result.dir_stats.contains_key(&node_modules));
}

#[test]
fn scan_with_structure_gitignore_scanner_exclude_prunes_directory() {
    // Test with gitignore-enabled scanner
    let temp_dir = TempDir::new().unwrap();

    let excluded_dir = temp_dir.path().join("vendor");
    let deep_dir = excluded_dir.join("deep").join("nested");
    std::fs::create_dir_all(&deep_dir).unwrap();
    std::fs::write(deep_dir.join("file.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["vendor/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
    assert!(!result.dir_stats.contains_key(&excluded_dir));
}

#[test]
fn scan_with_structure_excluded_dir_not_counted_in_parent_stats() {
    let temp_dir = TempDir::new().unwrap();

    // Create one included and one excluded directory
    let src_dir = temp_dir.path().join("src");
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::create_dir(&git_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(git_dir.join("HEAD"), "").unwrap();

    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec![".git/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Root should only count src as subdirectory, not .git
    let root_stats = result.dir_stats.get(temp_dir.path());
    assert!(root_stats.is_some());
    assert_eq!(root_stats.unwrap().dir_count, 1);
}
