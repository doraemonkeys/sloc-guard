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
