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
    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n!important.log\n").unwrap();

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

    assert!(!files.iter().any(|f| f.ends_with(std::path::Path::new("root.txt")) && f.parent() == Some(temp_dir.path())));
    assert!(files.iter().any(|f| f.ends_with(std::path::Path::new("sub").join("root.txt"))));
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
    std::fs::write(temp_dir.path().join(".gitignore"), "**/logs/*.log\nfile with spaces.txt\n").unwrap();

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