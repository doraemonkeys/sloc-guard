//! Tests for basic `GitAwareScanner` `scan()` functionality.
//!
//! Covers: gitignore respect, file filters, subdirectories, negation patterns,
//! anchored patterns, parent gitignore inheritance, wildcards, and spaces.

use super::super::{FileScanner, GitAwareScanner};
use super::mock_filters::{AcceptAllFilter, RustOnlyFilter, init_git_repo};
use tempfile::TempDir;

#[test]
fn respects_gitignore() {
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
fn respects_filter() {
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
fn finds_files_in_subdirectories() {
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
fn fails_outside_git_repo() {
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
fn handles_nested_gitignore() {
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
fn respects_negation() {
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
fn respects_anchored_patterns() {
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
fn respects_parent_gitignore() {
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
fn respects_wildcards_and_spaces() {
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

/// Bug regression test: scanning from a subdirectory should find files.
///
/// When `sloc-guard stats` is run from a subdirectory (e.g., `src/`), the scanner
/// receives the subdirectory path. The gix dirwalk pathspec pattern must correctly
/// match files within that subdirectory. Without a trailing `/`, gix may interpret
/// the pattern as matching a file named "src" rather than the directory contents.
#[test]
fn finds_files_when_scanning_subdirectory_directly() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create files in root
    std::fs::write(temp_dir.path().join("root.rs"), "fn root() {}").unwrap();

    // Create subdirectory with files
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub mod lib;").unwrap();

    // Create nested subdirectory
    let nested_dir = src_dir.join("utils");
    std::fs::create_dir(&nested_dir).unwrap();
    std::fs::write(nested_dir.join("helper.rs"), "fn helper() {}").unwrap();

    // Scan only the src subdirectory (simulates running from src/)
    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let files = scanner.scan(&src_dir).unwrap();

    // Should find all files in src/ and its subdirectories
    assert!(
        files.len() >= 3,
        "Expected at least 3 files in src/, got {}: {:?}",
        files.len(),
        files
    );
    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(files.iter().any(|f| f.ends_with("lib.rs")));
    assert!(files.iter().any(|f| f.ends_with("helper.rs")));

    // Should NOT find files outside src/
    assert!(!files.iter().any(|f| f.ends_with("root.rs")));
}

/// Bug regression test: scanning with relative path "." from a subdirectory.
///
/// This test simulates the exact scenario when running `sloc-guard stats` from
/// a subdirectory: the scanner receives `Path::new(".")` which resolves to the
/// subdirectory. The pathspec pattern must correctly match files.
#[test]
fn finds_files_when_scanning_with_relative_dot_path() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create files in root
    std::fs::write(temp_dir.path().join("root.rs"), "fn root() {}").unwrap();

    // Create subdirectory with files
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub mod lib;").unwrap();

    // Change to src directory and scan with "."
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&src_dir).unwrap();

    let scanner = GitAwareScanner::new(AcceptAllFilter);
    let result = scanner.scan(std::path::Path::new("."));

    // Restore original directory before assertions (to avoid affecting other tests)
    std::env::set_current_dir(&original_dir).unwrap();

    let files = result.expect("scan should succeed");

    // Should find files in the current directory (src/)
    assert!(
        files.len() >= 2,
        "Expected at least 2 files when scanning '.', got {}: {:?}",
        files.len(),
        files
    );
    assert!(
        files.iter().any(|f| f.ends_with("main.rs")),
        "Should find main.rs, got: {files:?}"
    );
    assert!(
        files.iter().any(|f| f.ends_with("lib.rs")),
        "Should find lib.rs, got: {files:?}"
    );

    // Should NOT find files outside src/
    assert!(
        !files.iter().any(|f| f.ends_with("root.rs")),
        "Should not find root.rs from parent directory"
    );
}
