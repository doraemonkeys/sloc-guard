use std::path::Path;

use super::*;
use tempfile::TempDir;

struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
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
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let files = scanner.scan_all(&paths).unwrap();

    assert_eq!(files.len(), 2);
}

#[test]
fn composite_scanner_scan_all_without_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), false);
    let files = scanner.scan_all(&[temp_dir.path().to_path_buf()]).unwrap();

    assert_eq!(files.len(), 1);
}

#[test]
fn composite_scanner_scan_with_gitignore_true_does_not_error() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), true);
    let result = scanner.scan(temp_dir.path());

    assert!(result.is_ok());
}

#[test]
fn composite_scanner_scan_all_with_gitignore_true() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), true);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let result = scanner.scan_all(&paths);

    assert!(result.is_ok());
}

#[test]
fn composite_scanner_scan_with_structure_without_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), false);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.files.len(), 1);
    assert!(result.dir_stats.contains_key(&src_dir));
}

#[test]
fn composite_scanner_scan_with_structure_with_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), true);
    let result = scanner.scan_with_structure(temp_dir.path(), None);

    assert!(result.is_ok());
}

#[test]
fn composite_scanner_scan_all_with_structure() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), false);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let result = scanner.scan_all_with_structure(&paths, None).unwrap();

    assert_eq!(result.files.len(), 2);
}

#[test]
fn composite_scanner_scan_all_with_structure_with_gitignore() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), true);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let result = scanner.scan_all_with_structure(&paths, None);

    assert!(result.is_ok());
}

#[test]
fn composite_scanner_excludes_dir_by_name() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    let node_modules = temp_dir.path().join("node_modules");
    std::fs::create_dir(&node_modules).unwrap();
    std::fs::write(node_modules.join("lib.rs"), "").unwrap();

    let scanner = CompositeScanner::new(vec!["**/node_modules/**".to_string()], false);
    let result = scanner.scan(temp_dir.path()).unwrap();

    assert_eq!(result.len(), 1);
    assert!(result[0].ends_with("test.rs"));
}

#[test]
fn composite_scanner_scan_with_structure_excludes_pattern() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    let build_dir = temp_dir.path().join("build");
    std::fs::create_dir(&build_dir).unwrap();
    std::fs::write(build_dir.join("output.rs"), "").unwrap();

    let scanner = CompositeScanner::new(vec!["**/build/**".to_string()], false);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
}

#[test]
fn composite_scanner_scan_with_structure_uses_exclude_patterns() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    let dist_dir = temp_dir.path().join("dist");
    std::fs::create_dir(&dist_dir).unwrap();
    std::fs::write(dist_dir.join("bundle.js"), "").unwrap();

    let scanner = CompositeScanner::new(vec!["**/dist/**".to_string()], false);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
}

#[test]
fn composite_scanner_fallback_respects_gitignore() {
    // Test DirectoryScanner with use_gitignore respects .gitignore
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("debug.log"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
}

#[test]
fn composite_scanner_fallback_respects_gitignore_scan_with_structure() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "build/\n").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();

    let build_dir = temp_dir.path().join("build");
    std::fs::create_dir(&build_dir).unwrap();
    std::fs::write(build_dir.join("output.js"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert!(
        !result
            .files
            .iter()
            .any(|f| f.to_string_lossy().contains("build"))
    );
}

#[test]
fn scan_files_without_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("lib.rs"), "").unwrap();

    let files = scan_files(&[temp_dir.path().to_path_buf()], &[], false).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn scan_files_with_exclude() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    std::fs::create_dir(temp_dir.path().join("vendor")).unwrap();
    std::fs::write(temp_dir.path().join("vendor/lib.rs"), "").unwrap();

    let files = scan_files(
        &[temp_dir.path().to_path_buf()],
        &["**/vendor/**".to_string()],
        false,
    )
    .unwrap();
    assert_eq!(files.len(), 1);
}

#[test]
fn scan_files_with_gitignore_true_fallback() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();

    let files = scan_files(&[temp_dir.path().to_path_buf()], &[], true);
    assert!(files.is_ok());
}

#[test]
fn scan_files_function_basic() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("lib.rs"), "").unwrap();

    let files = scan_files(&[temp_dir.path().to_path_buf()], &[], false).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn scan_files_function_with_exclude() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    let node_modules = temp_dir.path().join("node_modules");
    std::fs::create_dir(&node_modules).unwrap();
    std::fs::write(node_modules.join("dep.rs"), "").unwrap();

    let files = scan_files(
        &[temp_dir.path().to_path_buf()],
        &["**/node_modules/**".to_string()],
        false,
    )
    .unwrap();
    assert_eq!(files.len(), 1);
}

#[test]
fn composite_scanner_scan_all_no_gitignore_multiple_paths() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), false);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let files = scanner.scan_all(&paths).unwrap();

    assert_eq!(files.len(), 2);
}

#[test]
fn composite_scanner_scan_all_structure_no_gitignore_multiple_paths() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = CompositeScanner::new(Vec::new(), false);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let result = scanner.scan_all_with_structure(&paths, None).unwrap();

    assert_eq!(result.files.len(), 2);
}
