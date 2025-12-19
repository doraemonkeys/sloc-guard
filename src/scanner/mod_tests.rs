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

// =============================================================================
// CompositeScanner Tests
// =============================================================================

#[test]
fn composite_scanner_new_creates_scanner() {
    let scanner = CompositeScanner::new(Vec::new(), false);
    assert!(!scanner.use_gitignore);
}

#[test]
fn composite_scanner_scan_without_gitignore_finds_files() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), false);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 2);
}

#[test]
fn composite_scanner_scan_excludes_patterns() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    std::fs::create_dir(temp_dir.path().join("target")).unwrap();
    std::fs::write(temp_dir.path().join("target/build.rs"), "").unwrap();

    let scanner = CompositeScanner::new(vec!["**/target/**".to_string()], false);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("test.rs"));
}

#[test]
fn composite_scanner_scan_all_combines_results() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), false);
    let paths = vec![temp_dir1.path().to_path_buf(), temp_dir2.path().to_path_buf()];
    let files = scanner.scan_all(&paths).unwrap();

    assert_eq!(files.len(), 2);
}

#[test]
fn composite_scanner_scan_all_without_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();

    // use_gitignore = false, uses directory scanner directly
    let scanner = CompositeScanner::new(Vec::new(), false);
    let files = scanner.scan_all(&[temp_dir.path().to_path_buf()]).unwrap();

    assert_eq!(files.len(), 1);
}

#[test]
fn composite_scanner_scan_with_gitignore_true_does_not_error() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();

    // use_gitignore = true, should not error (may use git scanner or fallback)
    let scanner = CompositeScanner::new(Vec::new(), true);
    let result = scanner.scan(temp_dir.path());

    // Should not error regardless of git status
    assert!(result.is_ok());
}

#[test]
fn file_scanner_default_scan_all_uses_scan() {
    // Test that the default scan_all implementation calls scan for each path
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let filter = GlobFilter::new(Vec::new(), &[]).unwrap();
    let scanner = DirectoryScanner::new(filter);
    let paths = vec![temp_dir1.path().to_path_buf(), temp_dir2.path().to_path_buf()];

    // Uses the default scan_all implementation from FileScanner trait
    let files = scanner.scan_all(&paths).unwrap();

    assert_eq!(files.len(), 2);
}

#[test]
fn composite_scanner_scan_all_with_gitignore_true() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    // use_gitignore = true
    let scanner = CompositeScanner::new(Vec::new(), true);
    let paths = vec![temp_dir1.path().to_path_buf(), temp_dir2.path().to_path_buf()];
    let result = scanner.scan_all(&paths);

    // Should not error regardless of git status
    assert!(result.is_ok());
}
