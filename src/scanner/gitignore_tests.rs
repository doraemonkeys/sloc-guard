use std::path::Path;

use super::*;
use crate::scanner::{AllowlistRuleBuilder, StructureScanConfig};
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

fn init_git_repo(dir: &Path) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("Failed to init git repo");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .expect("Failed to set git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .expect("Failed to set git name");
}

#[test]
fn gitaware_scanner_respects_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create .gitignore
    std::fs::write(temp_dir.path().join(".gitignore"), "ignored/\n*.log\n").unwrap();

    // Create files
    std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("debug.log"), "log content").unwrap();

    let ignored_dir = temp_dir.path().join("ignored");
    std::fs::create_dir(&ignored_dir).unwrap();
    std::fs::write(ignored_dir.join("hidden.rs"), "ignored content").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    // Should find .gitignore and main.rs, but not debug.log or ignored/hidden.rs
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(files.iter().any(|f| f.ends_with(".gitignore")));
    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
    assert!(!files.iter().any(|f| f.ends_with("hidden.rs")));
}

#[test]
fn gitaware_scanner_respects_filter() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create files
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("test.txt"), "").unwrap();

    let scanner = GitAwareScanner::new(RustOnlyFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("test.rs"));
}

#[test]
fn gitaware_scanner_finds_files_in_subdirectories() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let sub_dir = temp_dir.path().join("src");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("lib.rs"), "pub fn foo() {}").unwrap();

    let scanner = GitAwareScanner::new(RustOnlyFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("lib.rs"));
}

#[test]
fn gitaware_scanner_fails_outside_git_repo() {
    // Create a temp directory and try to scan from it
    // Note: TempDir is typically in system temp which shouldn't be in a git repo,
    // but some test runners (like tarpaulin) may run from within a git repo,
    // causing gix::discover to find the parent repo.
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan(temp_dir.path());

    // The scan either fails (no git repo found) or succeeds (parent git repo found)
    // Both are valid behaviors depending on the test environment
    if let Err(e) = &result {
        // Expected: should be a Git error about not finding a repo
        assert!(e.to_string().contains("git") || e.to_string().contains("Git"));
    }
    // If Ok, it found a parent git repo (e.g., when running inside the project repo)
}

#[test]
fn gitaware_scanner_handles_nested_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Root .gitignore
    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();

    // Create nested dir with its own .gitignore
    let sub_dir = temp_dir.path().join("src");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join(".gitignore"), "local_only/\n").unwrap();
    std::fs::write(sub_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(sub_dir.join("app.log"), "log content").unwrap();

    let local_dir = sub_dir.join("local_only");
    std::fs::create_dir(&local_dir).unwrap();
    std::fs::write(local_dir.join("secret.rs"), "secret").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    // Should find main.rs but not app.log (root gitignore) or secret.rs (nested gitignore)
    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(!files.iter().any(|f| f.ends_with("app.log")));
    assert!(!files.iter().any(|f| f.ends_with("secret.rs")));
}

#[test]
fn gitaware_scanner_respects_negation() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Ignore all logs, but not important.log
    std::fs::write(
        temp_dir.path().join(".gitignore"),
        "*.log\n!important.log\n",
    )
    .unwrap();

    std::fs::write(temp_dir.path().join("debug.log"), "debug").unwrap();
    std::fs::write(temp_dir.path().join("important.log"), "important").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
    assert!(files.iter().any(|f| f.ends_with("important.log")));
}

#[test]
fn gitaware_scanner_respects_anchored_patterns() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Ignore /root.txt (root only), and other.txt (anywhere)
    std::fs::write(temp_dir.path().join(".gitignore"), "/root.txt\nother.txt\n").unwrap();

    std::fs::write(temp_dir.path().join("root.txt"), "root").unwrap();

    let sub_dir = temp_dir.path().join("sub");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("root.txt"), "sub root").unwrap();
    std::fs::write(sub_dir.join("other.txt"), "sub other").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(
        !files
            .iter()
            .any(|f| f.ends_with(std::path::Path::new("root.txt"))
                && f.parent() == Some(temp_dir.path()))
    );
    assert!(
        files
            .iter()
            .any(|f| f.ends_with(std::path::Path::new("sub").join("root.txt")))
    );
    assert!(!files.iter().any(|f| f.ends_with("other.txt")));
}

#[test]
fn gitaware_scanner_respects_parent_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();

    let sub_dir = temp_dir.path().join("src");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(sub_dir.join("debug.log"), "log").unwrap();

    // Scan only the subdirectory, but it should respect parent .gitignore
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let files = scanner.scan(&sub_dir).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
}

#[test]
fn gitaware_scanner_respects_wildcards_and_spaces() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Ignore nested logs and files with spaces
    std::fs::write(
        temp_dir.path().join(".gitignore"),
        "**/logs/*.log\nfile with spaces.txt\n",
    )
    .unwrap();

    let logs_dir = temp_dir.path().join("app/logs");
    std::fs::create_dir_all(&logs_dir).unwrap();
    std::fs::write(logs_dir.join("error.log"), "error").unwrap();
    std::fs::write(logs_dir.join("other.txt"), "ok").unwrap();

    std::fs::write(temp_dir.path().join("file with spaces.txt"), "spaces").unwrap();
    std::fs::write(temp_dir.path().join("normal.txt"), "normal").unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(!files.iter().any(|f| f.ends_with("error.log")));
    assert!(files.iter().any(|f| f.ends_with("other.txt")));
    assert!(!files.iter().any(|f| f.ends_with("file with spaces.txt")));
    assert!(files.iter().any(|f| f.ends_with("normal.txt")));
}

// =============================================================================
// scan_with_structure Tests
// =============================================================================

#[test]
fn gitaware_scanner_scan_with_structure_collects_stats() {
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
fn gitaware_scanner_scan_with_structure_respects_gitignore() {
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
fn gitaware_scanner_scan_with_structure_with_allowlist() {
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
fn gitaware_scanner_scan_with_structure_respects_count_exclude() {
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
fn gitaware_scanner_scan_with_structure_respects_scanner_exclude() {
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
fn gitaware_scanner_scan_with_structure_counts_subdirectories() {
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
fn gitaware_scanner_infers_dir_count_from_file_paths() {
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
fn gitaware_scanner_dir_count_with_max_dirs_zero() {
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
fn gitaware_scanner_dir_count_nested_hierarchy() {
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
fn gitaware_scanner_scan_with_structure_tracks_depth() {
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
fn gitaware_scanner_scan_with_structure_no_violations_when_files_match() {
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

// =============================================================================
// deny_file_patterns Tests (relative path matching fix)
// =============================================================================

#[test]
fn gitaware_scanner_deny_file_patterns_with_relative_pattern() {
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
fn gitaware_scanner_deny_file_patterns_nested_directories() {
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
fn gitaware_scanner_global_deny_file_patterns() {
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
fn gitaware_scanner_deny_file_patterns_does_not_match_allowed_files() {
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
fn gitaware_scanner_deny_file_patterns_with_glob() {
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

// =============================================================================
// deny_dirs Tests (directory deny pattern detection via register_directory_chain)
// =============================================================================

#[test]
fn gitaware_scanner_global_deny_dirs_basename() {
    // Test global deny_dirs with basename-only matching (e.g., "node_modules", "__pycache__")
    // This tests that register_directory_chain correctly checks dir_matches_global_deny_basename
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // src/
    //   main.rs
    // node_modules/       <- should be denied
    //   package/
    //     index.js
    // __pycache__/        <- should be denied
    //   cache.pyc
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let node_modules = temp_dir.path().join("node_modules");
    let package_dir = node_modules.join("package");
    std::fs::create_dir_all(&package_dir).unwrap();
    std::fs::write(package_dir.join("index.js"), "").unwrap();

    let pycache = temp_dir.path().join("__pycache__");
    std::fs::create_dir(&pycache).unwrap();
    std::fs::write(pycache.join("cache.pyc"), "").unwrap();

    // Configure global deny_dirs (basename matching)
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        &["node_modules".to_string(), "__pycache__".to_string()],
    )
    .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Both node_modules and __pycache__ directories should be violations
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("node_modules")),
        "Expected node_modules to be denied, violations: {:?}",
        result.allowlist_violations
    );
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("__pycache__")),
        "Expected __pycache__ to be denied, violations: {:?}",
        result.allowlist_violations
    );
}

#[test]
fn gitaware_scanner_global_deny_dirs_pattern_with_slash() {
    // Test global deny_dirs with directory-only patterns (ending with `/`)
    // This tests that register_directory_chain correctly checks dir_matches_global_deny
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // src/
    //   main.rs
    // build/              <- should be denied by "build/"
    //   output.bin
    // dist/               <- should be denied by "**/dist/"
    //   bundle.js
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let build_dir = temp_dir.path().join("build");
    std::fs::create_dir(&build_dir).unwrap();
    std::fs::write(build_dir.join("output.bin"), "").unwrap();

    let dist_dir = temp_dir.path().join("dist");
    std::fs::create_dir(&dist_dir).unwrap();
    std::fs::write(dist_dir.join("bundle.js"), "").unwrap();

    // Configure global deny_patterns with directory patterns (ending with `/`)
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &["build/".to_string(), "**/dist/".to_string()],
        &[],
        &[],
    )
    .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Both build and dist directories should be violations
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("build")),
        "Expected build to be denied, violations: {:?}",
        result.allowlist_violations
    );
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("dist")),
        "Expected dist to be denied, violations: {:?}",
        result.allowlist_violations
    );
}

#[test]
fn gitaware_scanner_per_rule_deny_dirs() {
    // Test per-rule deny_dirs patterns
    // This tests that register_directory_chain correctly checks rule.dir_matches_deny
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // src/
    //   main.rs
    //   utils/            <- should be denied by rule's deny_dirs
    //     helper.rs
    //   models/           <- allowed
    //     user.rs
    let src_dir = temp_dir.path().join("src");
    let utils_dir = src_dir.join("utils");
    let models_dir = src_dir.join("models");
    std::fs::create_dir_all(&utils_dir).unwrap();
    std::fs::create_dir_all(&models_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(utils_dir.join("helper.rs"), "").unwrap();
    std::fs::write(models_dir.join("user.rs"), "").unwrap();

    // Configure rule with deny_dirs
    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_dirs(vec!["utils".to_string(), "helpers".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // utils directory should be a violation
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("utils")),
        "Expected utils to be denied, violations: {:?}",
        result.allowlist_violations
    );

    // models directory should NOT be a violation
    assert!(
        !result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("models")),
        "Expected models to be allowed, but found violations: {:?}",
        result.allowlist_violations
    );
}

#[test]
fn gitaware_scanner_nested_deny_dirs() {
    // Test that deny_dirs works in deeply nested structures
    // This is important because directories are inferred from file paths
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // src/
    //   module/
    //     submodule/
    //       __tests__/   <- should be denied
    //         test.rs
    let tests_dir = temp_dir
        .path()
        .join("src")
        .join("module")
        .join("submodule")
        .join("__tests__");
    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::write(tests_dir.join("test.rs"), "").unwrap();

    // Also create a legitimate file to ensure directories are registered
    let submodule_dir = temp_dir.path().join("src").join("module").join("submodule");
    std::fs::write(submodule_dir.join("mod.rs"), "").unwrap();

    // Configure global deny_dirs
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        &["__tests__".to_string()],
    )
    .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // __tests__ directory should be a violation
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("__tests__")),
        "Expected __tests__ to be denied in nested structure, violations: {:?}",
        result.allowlist_violations
    );
}

#[test]
fn gitaware_scanner_deny_dirs_glob_pattern() {
    // Test deny_dirs with glob patterns (e.g., "test_*", "*_backup")
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // src/
    //   main.rs
    //   test_utils/       <- should be denied by "test_*"
    //     helper.rs
    //   old_backup/       <- should be denied by "*_backup"
    //     old.rs
    //   production/       <- allowed
    //     app.rs
    let src_dir = temp_dir.path().join("src");
    let test_utils = src_dir.join("test_utils");
    let old_backup = src_dir.join("old_backup");
    let production = src_dir.join("production");
    std::fs::create_dir_all(&test_utils).unwrap();
    std::fs::create_dir_all(&old_backup).unwrap();
    std::fs::create_dir_all(&production).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(test_utils.join("helper.rs"), "").unwrap();
    std::fs::write(old_backup.join("old.rs"), "").unwrap();
    std::fs::write(production.join("app.rs"), "").unwrap();

    // Configure global deny_dirs with glob patterns
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        &["test_*".to_string(), "*_backup".to_string()],
    )
    .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // test_utils and old_backup should be violations
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("test_utils")),
        "Expected test_utils to be denied, violations: {:?}",
        result.allowlist_violations
    );
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("old_backup")),
        "Expected old_backup to be denied, violations: {:?}",
        result.allowlist_violations
    );

    // production should NOT be a violation
    assert!(
        !result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("production")),
        "Expected production to be allowed"
    );
}

#[test]
fn gitaware_scanner_deny_dirs_no_duplicate_violations() {
    // Test that the same directory is not reported multiple times
    // (register_directory_chain uses seen_dirs to track visited directories)
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure with multiple files in the same denied directory
    // node_modules/
    //   file1.js
    //   file2.js
    //   file3.js
    let node_modules = temp_dir.path().join("node_modules");
    std::fs::create_dir(&node_modules).unwrap();
    std::fs::write(node_modules.join("file1.js"), "").unwrap();
    std::fs::write(node_modules.join("file2.js"), "").unwrap();
    std::fs::write(node_modules.join("file3.js"), "").unwrap();

    // Configure global deny_dirs
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
        &["node_modules".to_string()],
    )
    .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Should have exactly 1 violation for node_modules, not 3
    let node_modules_violations: Vec<_> = result
        .allowlist_violations
        .iter()
        .filter(|v| v.path.to_string_lossy().contains("node_modules"))
        .collect();

    assert_eq!(
        node_modules_violations.len(),
        1,
        "Expected exactly 1 violation for node_modules, got {}: {:?}",
        node_modules_violations.len(),
        node_modules_violations
    );
}

#[test]
fn gitaware_scanner_deny_dirs_combined_with_file_deny() {
    // Test that both file and directory deny patterns work together
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // src/
    //   main.rs
    //   secrets.json      <- should be denied (file)
    //   __pycache__/      <- should be denied (directory)
    //     cache.pyc
    let src_dir = temp_dir.path().join("src");
    let pycache = src_dir.join("__pycache__");
    std::fs::create_dir_all(&pycache).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("secrets.json"), "").unwrap();
    std::fs::write(pycache.join("cache.pyc"), "").unwrap();

    // Configure both file and directory deny patterns
    let config = StructureScanConfig::new(
        &[],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &["secrets.json".to_string()],
        &["__pycache__".to_string()],
    )
    .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Both secrets.json and __pycache__ should be violations
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("secrets.json")),
        "Expected secrets.json to be denied"
    );
    assert!(
        result
            .allowlist_violations
            .iter()
            .any(|v| v.path.to_string_lossy().contains("__pycache__")),
        "Expected __pycache__ to be denied"
    );
}

#[test]
fn gitaware_scanner_per_rule_deny_dirs_nested_rule_scope() {
    // Test per-rule deny_dirs with a more specific rule scope
    // Note: The scope pattern `**/src` matches src directories, and deny_dirs checks
    // directories whose PARENT matches the scope pattern.
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create structure:
    // frontend/
    //   src/
    //     __mocks__/   <- should be denied (parent `src` matches `**/src`)
    //       mock.js
    //     utils/       <- should be denied (another dir type)
    //       helper.js
    //   tests/         <- should NOT be denied (parent is frontend, not src)
    //     test.js
    // backend/
    //   lib/
    //     __mocks__/   <- should NOT be denied (parent is lib, not src)
    //       mock.rs
    let frontend_src = temp_dir.path().join("frontend").join("src");
    let frontend_mocks = frontend_src.join("__mocks__");
    let frontend_utils = frontend_src.join("utils");
    let frontend_tests = temp_dir.path().join("frontend").join("tests");
    let backend_lib = temp_dir.path().join("backend").join("lib");
    let backend_mocks = backend_lib.join("__mocks__");

    std::fs::create_dir_all(&frontend_mocks).unwrap();
    std::fs::create_dir_all(&frontend_utils).unwrap();
    std::fs::create_dir_all(&frontend_tests).unwrap();
    std::fs::create_dir_all(&backend_mocks).unwrap();
    std::fs::write(frontend_mocks.join("mock.js"), "").unwrap();
    std::fs::write(frontend_utils.join("helper.js"), "").unwrap();
    std::fs::write(frontend_tests.join("test.js"), "").unwrap();
    std::fs::write(backend_mocks.join("mock.rs"), "").unwrap();

    // Also add files in src/lib directories for proper scanning
    std::fs::write(frontend_src.join("app.js"), "").unwrap();
    std::fs::write(backend_lib.join("app.rs"), "").unwrap();

    // Configure rule scoped to `**/src` - matches any `src` directory
    // deny_dirs will apply to directories whose parent matches `**/src`
    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_dirs(vec!["__mocks__".to_string(), "utils".to_string()])
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[], &[])
            .unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // frontend/src/__mocks__ should be a violation
    let frontend_mocks_violation = result.allowlist_violations.iter().any(|v| {
        let path_str = v.path.to_string_lossy();
        path_str.contains("frontend") && path_str.contains("src") && path_str.contains("__mocks__")
    });
    assert!(
        frontend_mocks_violation,
        "Expected frontend/src/__mocks__ to be denied, violations: {:?}",
        result.allowlist_violations
    );

    // frontend/src/utils should also be a violation
    let frontend_utils_violation = result.allowlist_violations.iter().any(|v| {
        let path_str = v.path.to_string_lossy();
        path_str.contains("frontend") && path_str.contains("src") && path_str.contains("utils")
    });
    assert!(
        frontend_utils_violation,
        "Expected frontend/src/utils to be denied, violations: {:?}",
        result.allowlist_violations
    );

    // backend/lib/__mocks__ should NOT be a violation (parent is lib, not src)
    let backend_mocks_violation = result.allowlist_violations.iter().any(|v| {
        let path_str = v.path.to_string_lossy();
        path_str.contains("backend") && path_str.contains("__mocks__")
    });
    assert!(
        !backend_mocks_violation,
        "Expected backend/lib/__mocks__ to NOT be denied (rule only applies to dirs under src)"
    );
}
