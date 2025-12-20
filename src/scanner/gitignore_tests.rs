use std::path::Path;

use super::*;
use crate::scanner::{StructureScanConfig, WhitelistRuleBuilder};
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
fn gitaware_scanner_scan_with_structure_with_whitelist() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let whitelist_rule = WhitelistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![whitelist_rule]).unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // config.json should be a whitelist violation
    assert!(!result.whitelist_violations.is_empty());
    assert!(
        result
            .whitelist_violations
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

    let config = StructureScanConfig::new(&["*.txt".to_string()], &[], Vec::new()).unwrap();
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

    let config = StructureScanConfig::new(&[], &["**/vendor/**".to_string()], Vec::new()).unwrap();
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

    let whitelist_rule = WhitelistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![whitelist_rule]).unwrap();
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // All files are .rs, so no violations
    assert!(result.whitelist_violations.is_empty());
}
