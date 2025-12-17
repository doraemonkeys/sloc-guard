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
