use std::path::Path;

use super::*;
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
// Basic DirectoryScanner Tests
// =============================================================================

#[test]
fn scanner_finds_files_in_directory() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 2);
}

#[test]
fn scanner_finds_files_in_subdirectories() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("src");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("main.rs"), "fn main() {}").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("main.rs"));
}

#[test]
fn scanner_respects_filter() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("test.txt"), "").unwrap();

    let scanner = DirectoryScanner::new(RustOnlyFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("test.rs"));
}

#[test]
fn file_scanner_default_scan_all_uses_scan() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let filter = GlobFilter::new(Vec::new(), &[]).unwrap();
    let scanner = DirectoryScanner::new(filter);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];

    let files = scanner.scan_all(&paths).unwrap();

    assert_eq!(files.len(), 2);
}

// =============================================================================
// DirectoryScanner .gitignore Support Tests (Non-Git Repo)
// =============================================================================

#[test]
fn directory_scanner_with_gitignore_respects_gitignore_without_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    // Note: NOT initializing a git repo

    std::fs::write(temp_dir.path().join(".gitignore"), "ignored/\n*.log\n").unwrap();

    std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("debug.log"), "log content").unwrap();

    let ignored_dir = temp_dir.path().join("ignored");
    std::fs::create_dir(&ignored_dir).unwrap();
    std::fs::write(ignored_dir.join("hidden.rs"), "ignored content").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(files.iter().any(|f| f.ends_with(".gitignore")));
    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
    assert!(!files.iter().any(|f| f.ends_with("hidden.rs")));
}

#[test]
fn directory_scanner_without_gitignore_ignores_gitignore_file() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("debug.log"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, false);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(files.iter().any(|f| f.ends_with("debug.log")));
}

#[test]
fn directory_scanner_with_gitignore_nested_gitignore() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();

    let sub_dir = temp_dir.path().join("src");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join(".gitignore"), "local_only/\n").unwrap();
    std::fs::write(sub_dir.join("main.rs"), "").unwrap();
    std::fs::write(sub_dir.join("app.log"), "").unwrap();

    let local_dir = sub_dir.join("local_only");
    std::fs::create_dir(&local_dir).unwrap();
    std::fs::write(local_dir.join("secret.rs"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(!files.iter().any(|f| f.ends_with("app.log")));
    assert!(!files.iter().any(|f| f.ends_with("secret.rs")));
}

#[test]
fn directory_scanner_with_gitignore_no_gitignore_file() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("test.log"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 2);
}

#[test]
fn directory_scanner_with_gitignore_scan_with_structure() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "logs/\n").unwrap();

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let logs_dir = temp_dir.path().join("logs");
    std::fs::create_dir(&logs_dir).unwrap();
    for i in 0..5 {
        std::fs::write(logs_dir.join(format!("log{i}.txt")), "").unwrap();
    }

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert!(
        !result
            .files
            .iter()
            .any(|f| f.to_string_lossy().contains("logs"))
    );
    // .gitignore + main.rs
    assert_eq!(result.files.len(), 2);
}

#[test]
fn directory_scanner_with_gitignore_negation_pattern() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(
        temp_dir.path().join(".gitignore"),
        "*.log\n!important.log\n",
    )
    .unwrap();
    std::fs::write(temp_dir.path().join("debug.log"), "").unwrap();
    std::fs::write(temp_dir.path().join("important.log"), "").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(files.iter().any(|f| f.ends_with("important.log")));
    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
}

// =============================================================================
// DirectoryScanner .gitignore Edge Cases (Issue #3)
// =============================================================================

#[test]
fn directory_scanner_with_gitignore_anchored_pattern() {
    // Anchored patterns (starting with /) only match at root level
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(
        temp_dir.path().join(".gitignore"),
        "/root.txt\ncommon.txt\n",
    )
    .unwrap();

    // Root level files
    std::fs::write(temp_dir.path().join("root.txt"), "root content").unwrap();
    std::fs::write(temp_dir.path().join("common.txt"), "common at root").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();

    // Subdirectory with same filenames
    let sub_dir = temp_dir.path().join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(
        sub_dir.join("root.txt"),
        "not ignored - anchored only at root",
    )
    .unwrap();
    std::fs::write(sub_dir.join("common.txt"), "ignored - non-anchored").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    // Anchored /root.txt: only root level is ignored
    assert!(
        !files
            .iter()
            .any(|f| { f.ends_with("root.txt") && !f.to_string_lossy().contains("subdir") })
    );
    // Non-anchored common.txt: ignored everywhere
    assert!(!files.iter().any(|f| f.ends_with("common.txt")));
    // subdir/root.txt should NOT be ignored (anchored pattern)
    assert!(
        files
            .iter()
            .any(|f| { f.to_string_lossy().contains("subdir") && f.ends_with("root.txt") })
    );
    // main.rs should be included
    assert!(files.iter().any(|f| f.ends_with("main.rs")));
}

#[test]
fn directory_scanner_with_gitignore_complex_wildcards() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(
        temp_dir.path().join(".gitignore"),
        "**/temp/**\n*.bak\ntest_*.log\n",
    )
    .unwrap();

    // Regular files
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("config.bak"), "").unwrap();
    std::fs::write(temp_dir.path().join("test_debug.log"), "").unwrap();
    std::fs::write(temp_dir.path().join("production.log"), "").unwrap();

    // Nested temp directory
    let temp_nested = temp_dir.path().join("src").join("temp").join("cache");
    std::fs::create_dir_all(&temp_nested).unwrap();
    std::fs::write(temp_nested.join("cached.rs"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(files.iter().any(|f| f.ends_with("production.log")));
    assert!(!files.iter().any(|f| f.ends_with("config.bak")));
    assert!(!files.iter().any(|f| f.ends_with("test_debug.log")));
    // Anything under **/temp/** should be ignored
    assert!(!files.iter().any(|f| f.ends_with("cached.rs")));
}

#[test]
fn directory_scanner_with_gitignore_scan_subdirectory_directly() {
    // When scanning a subdirectory, parent .gitignore files should still apply
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();

    let sub_dir = temp_dir.path().join("src");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(sub_dir.join("debug.log"), "log content").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    // Scan the subdirectory directly, not the root
    let files = scanner.scan(&sub_dir).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    // Parent .gitignore should still be respected (ignore crate traverses parents)
    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
}

#[test]
fn directory_scanner_with_gitignore_relative_dot_path() {
    // Test scanning with "." as current directory
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("ignored.txt"), "").unwrap();

    // Use the temp_dir path directly (simulates scanning ".")
    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(!files.iter().any(|f| f.ends_with("ignored.txt")));
}

// =============================================================================
// DirectoryScanner WalkDir Pruning Tests (Issue #4)
// =============================================================================

#[test]
fn directory_scanner_walkdir_prunes_excluded_directories() {
    // Test that scan_with_structure_walkdir (non-gitignore path) correctly prunes
    use super::StructureScanConfig;
    use super::structure_config::TestConfigParams;

    let temp_dir = TempDir::new().unwrap();

    // Create a structure with excluded directories
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();

    // Create excluded directory with deep nesting
    let excluded = temp_dir.path().join("node_modules");
    std::fs::create_dir(&excluded).unwrap();
    let deep_nested = excluded.join("pkg").join("dist");
    std::fs::create_dir_all(&deep_nested).unwrap();
    std::fs::write(deep_nested.join("bundle.js"), "").unwrap();

    // Configure scanner_exclude to prune node_modules
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["node_modules/**".to_string()],
        ..Default::default()
    })
    .unwrap();

    // Use DirectoryScanner WITHOUT gitignore (exercises scan_with_structure_walkdir)
    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, false);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // src/main.rs should be found
    assert!(result.files.iter().any(|f| f.ends_with("main.rs")));
    // node_modules content should be pruned entirely
    assert!(!result.files.iter().any(|f| f.ends_with("bundle.js")));
    // node_modules directory itself should not appear in dir_stats (pruned early)
    assert!(
        !result
            .dir_stats
            .keys()
            .any(|p| { p.to_string_lossy().contains("node_modules") })
    );
}

#[test]
fn directory_scanner_walkdir_prunes_nested_excluded() {
    use super::StructureScanConfig;
    use super::structure_config::TestConfigParams;

    let temp_dir = TempDir::new().unwrap();

    // Create structure: src/main.rs, src/.cache/data.json, build/output.js
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let cache_dir = src_dir.join(".cache");
    std::fs::create_dir(&cache_dir).unwrap();
    std::fs::write(cache_dir.join("data.json"), "").unwrap();

    let build_dir = temp_dir.path().join("build");
    std::fs::create_dir(&build_dir).unwrap();
    std::fs::write(build_dir.join("output.js"), "").unwrap();

    // Exclude both .cache and build directories
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec![".cache/**".to_string(), "build/**".to_string()],
        ..Default::default()
    })
    .unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, false);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // main.rs should be found
    assert!(result.files.iter().any(|f| f.ends_with("main.rs")));
    // Both excluded directories' contents should be pruned
    assert!(!result.files.iter().any(|f| f.ends_with("data.json")));
    assert!(!result.files.iter().any(|f| f.ends_with("output.js")));
}
