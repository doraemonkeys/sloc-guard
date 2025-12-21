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
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];

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
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let result = scanner.scan_all(&paths);

    // Should not error regardless of git status
    assert!(result.is_ok());
}

// =============================================================================
// StructureScanConfig Tests
// =============================================================================

#[test]
fn structure_scan_config_new_creates_config() {
    let config = StructureScanConfig::new(&[], &[], Vec::new()).unwrap();
    assert!(config.allowlist_rules.is_empty());
}

#[test]
fn structure_scan_config_with_count_exclude() {
    let config = StructureScanConfig::new(&["*.generated".to_string()], &[], Vec::new()).unwrap();
    let path = Path::new("foo.generated");
    assert!(config.count_exclude.is_match(path));
}

#[test]
fn structure_scan_config_with_scanner_exclude() {
    let config = StructureScanConfig::new(&[], &["**/target/**".to_string()], Vec::new()).unwrap();
    let path = Path::new("src/target/build.rs");
    assert!(config.scanner_exclude.is_match(path));
}

#[test]
fn structure_scan_config_extracts_dir_names() {
    let config = StructureScanConfig::new(
        &[],
        &["target/**".to_string(), "node_modules/**".to_string()],
        Vec::new(),
    )
    .unwrap();
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"target".to_string())
    );
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"node_modules".to_string())
    );
}

#[test]
fn structure_scan_config_invalid_pattern_returns_error() {
    let result = StructureScanConfig::new(&["[invalid".to_string()], &[], Vec::new());
    assert!(result.is_err());
}

// =============================================================================
// AllowlistRule Tests
// =============================================================================

#[test]
fn allowlist_rule_builder_creates_rule() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    assert_eq!(rule.pattern, "src/**");
    assert_eq!(rule.allow_extensions, vec![".rs".to_string()]);
}

#[test]
fn allowlist_rule_builder_with_patterns() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["*.config".to_string()])
        .build()
        .unwrap();
    assert!(!rule.allow_patterns.is_empty());
}

#[test]
fn allowlist_rule_matches_directory() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    assert!(rule.matches_directory(Path::new("src/lib")));
    assert!(!rule.matches_directory(Path::new("tests/lib")));
}

#[test]
fn allowlist_rule_file_matches_extension() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string(), ".toml".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/main.rs")));
    assert!(rule.file_matches(Path::new("src/Cargo.toml")));
    assert!(!rule.file_matches(Path::new("src/config.json")));
}

#[test]
fn allowlist_rule_file_matches_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["Makefile".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/Makefile")));
    assert!(!rule.file_matches(Path::new("src/config.json")));
}

#[test]
fn allowlist_rule_invalid_pattern_returns_error() {
    let result = AllowlistRuleBuilder::new("[invalid".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build();
    assert!(result.is_err());
}

// =============================================================================
// scan_with_structure Tests
// =============================================================================

#[test]
fn scan_with_structure_collects_files_and_stats() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub fn foo() {}").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.files.len(), 2);
    assert!(result.dir_stats.contains_key(&src_dir));
    assert_eq!(result.dir_stats[&src_dir].file_count, 2);
}

#[test]
fn scan_with_structure_counts_subdirectories() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let sub1 = src_dir.join("sub1");
    let sub2 = src_dir.join("sub2");
    std::fs::create_dir_all(&sub1).unwrap();
    std::fs::create_dir_all(&sub2).unwrap();
    std::fs::write(sub1.join("a.rs"), "").unwrap();
    std::fs::write(sub2.join("b.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.dir_stats[&src_dir].dir_count, 2);
}

#[test]
fn scan_with_structure_tracks_depth() {
    let temp_dir = TempDir::new().unwrap();
    let deep = temp_dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("file.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    assert_eq!(result.dir_stats[&deep].depth, 3);
}

#[test]
fn scan_with_structure_respects_scanner_exclude() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(target_dir.join("build.rs"), "").unwrap();

    // Use **/target/** which matches files inside target directory
    let config = StructureScanConfig::new(&[], &["**/target/**".to_string()], Vec::new()).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // target files should be excluded
    assert_eq!(result.files.len(), 1);
    // target dir should not be counted since it's excluded
    assert!(
        result
            .dir_stats
            .get(temp_dir.path())
            .is_none_or(|s| s.dir_count <= 1)
    );
}

#[test]
fn scan_with_structure_respects_count_exclude() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("generated.txt"), "").unwrap();

    let config = StructureScanConfig::new(&["*.txt".to_string()], &[], Vec::new()).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // txt files should be found but not counted
    assert_eq!(result.files.len(), 2);
    // Only .rs file is counted (txt is excluded from count)
    assert_eq!(result.dir_stats[&src_dir].file_count, 1);
}

#[test]
fn scan_with_structure_detects_allowlist_violations() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // config.json should be an allowlist violation
    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("config.json"));
}

#[test]
fn scan_with_structure_no_violation_for_matching_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("lib.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // All files are .rs, so no violations
    assert!(result.allowlist_violations.is_empty());
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

    // Should not error
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
fn scan_result_default() {
    let result = ScanResult::default();
    assert!(result.files.is_empty());
    assert!(result.dir_stats.is_empty());
    assert!(result.allowlist_violations.is_empty());
}

#[test]
fn allowlist_rule_file_no_match_empty_allowlist() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![])
        .with_patterns(vec![])
        .build()
        .unwrap();
    // Empty allowlist matches nothing
    assert!(!rule.file_matches(Path::new("src/main.rs")));
}

#[test]
fn structure_scan_config_is_scanner_excluded_file() {
    let config = StructureScanConfig::new(&[], &["*.lock".to_string()], Vec::new()).unwrap();
    assert!(config.scanner_exclude.is_match(Path::new("Cargo.lock")));
    assert!(!config.scanner_exclude.is_match(Path::new("Cargo.toml")));
}

#[test]
fn structure_scan_config_is_count_excluded() {
    let config =
        StructureScanConfig::new(&["*.generated.rs".to_string()], &[], Vec::new()).unwrap();
    assert!(
        config
            .count_exclude
            .is_match(Path::new("types.generated.rs"))
    );
    assert!(!config.count_exclude.is_match(Path::new("types.rs")));
}

#[test]
fn structure_scan_config_find_matching_allowlist_rule() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![rule]).unwrap();

    assert!(
        config
            .allowlist_rules
            .iter()
            .any(|r| r.matches_directory(Path::new("src/lib")))
    );
    assert!(
        !config
            .allowlist_rules
            .iter()
            .any(|r| r.matches_directory(Path::new("tests/lib")))
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

    // Should not error
    assert!(result.is_ok());
}

#[test]
fn allowlist_rule_builder_invalid_allow_pattern() {
    let result = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["[invalid".to_string()])
        .build();
    assert!(result.is_err());
}

#[test]
fn scan_files_with_gitignore_true_fallback() {
    // When gitignore is true but not in a git repo, should fallback
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();

    let files = scan_files(&[temp_dir.path().to_path_buf()], &[], true);
    // Should not error, either finds files or falls back
    assert!(files.is_ok());
}

#[test]
fn structure_scan_config_extract_dir_names_windows_paths() {
    let config = StructureScanConfig::new(&[], &["target\\**".to_string()], Vec::new()).unwrap();
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"target".to_string())
    );
}

#[test]
fn allowlist_rule_empty_extension_list() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![])
        .build()
        .unwrap();
    // Empty extension list means no extension matches
    assert!(rule.allow_extensions.is_empty());
}

#[test]
fn scan_with_structure_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path().join("empty");
    std::fs::create_dir(&empty_dir).unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Should have the empty directory in stats
    assert!(result.dir_stats.contains_key(&empty_dir));
    assert_eq!(result.dir_stats[&empty_dir].file_count, 0);
}

#[test]
fn composite_scanner_excludes_dir_by_name() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.rs"), "").unwrap();
    let node_modules = temp_dir.path().join("node_modules");
    std::fs::create_dir(&node_modules).unwrap();
    std::fs::write(node_modules.join("lib.rs"), "").unwrap();

    // Pattern **/node_modules/** should exclude files in node_modules
    let scanner = CompositeScanner::new(vec!["**/node_modules/**".to_string()], false);
    let result = scanner.scan(temp_dir.path()).unwrap();

    // Should only find test.rs
    assert_eq!(result.len(), 1);
    assert!(result[0].ends_with("test.rs"));
}

#[test]
fn structure_scan_config_is_scanner_excluded_by_dir_name() {
    let config = StructureScanConfig::new(&[], &["target/**".to_string()], Vec::new()).unwrap();
    // Should match directory name
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"target".to_string())
    );
}

#[test]
fn allowlist_rule_matches_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["Makefile".to_string(), "*.config".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/Makefile")));
    assert!(rule.file_matches(Path::new("src/app.config")));
    assert!(!rule.file_matches(Path::new("src/random.txt")));
}

#[test]
fn scan_with_structure_filter_excludes_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("test.txt"), "").unwrap();

    let scanner = DirectoryScanner::new(RustOnlyFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Only .rs file in files list
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
    // But dir_stats still counts both files (filter doesn't affect counting)
    assert_eq!(result.dir_stats[&src_dir].file_count, 2);
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

    // build directory should be excluded
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
}

#[test]
fn scan_with_structure_depth_zero_at_root() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("root.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Root directory should have depth 0
    assert!(result.dir_stats.contains_key(temp_dir.path()));
    assert_eq!(result.dir_stats[temp_dir.path()].depth, 0);
}

#[test]
fn structure_scan_config_empty_patterns_match_nothing() {
    let config = StructureScanConfig::new(&[], &[], Vec::new()).unwrap();
    assert!(!config.count_exclude.is_match(Path::new("any.rs")));
    assert!(!config.scanner_exclude.is_match(Path::new("any.rs")));
}

#[test]
fn allowlist_rule_file_matches_by_full_path() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["**/special.txt".to_string()])
        .build()
        .unwrap();
    // Pattern should match full path too
    assert!(rule.file_matches(Path::new("src/nested/special.txt")));
}

#[test]
fn scan_with_structure_multiple_dirs_at_same_level() {
    let temp_dir = TempDir::new().unwrap();
    let dir1 = temp_dir.path().join("dir1");
    let dir2 = temp_dir.path().join("dir2");
    let dir3 = temp_dir.path().join("dir3");
    std::fs::create_dir(&dir1).unwrap();
    std::fs::create_dir(&dir2).unwrap();
    std::fs::create_dir(&dir3).unwrap();
    std::fs::write(dir1.join("a.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Root should have 3 subdirectories
    assert_eq!(result.dir_stats[temp_dir.path()].dir_count, 3);
}

#[test]
fn structure_scan_config_is_scanner_excluded_directory_by_name() {
    // Test that directories matching exclude dir names are excluded
    let config =
        StructureScanConfig::new(&[], &["node_modules/**".to_string()], Vec::new()).unwrap();

    // For directories, check if the name matches
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"node_modules".to_string())
    );
}

#[test]
fn scan_with_structure_handles_files_at_root() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("b.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Root should have 2 files
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.dir_stats[temp_dir.path()].file_count, 2);
}

#[test]
fn structure_scan_config_combined_patterns() {
    let config = StructureScanConfig::new(
        &["*.gen".to_string()],
        &["vendor/**".to_string(), "dist/**".to_string()],
        Vec::new(),
    )
    .unwrap();

    assert!(config.count_exclude.is_match(Path::new("foo.gen")));
    assert!(config.scanner_exclude.is_match(Path::new("vendor/lib.rs")));
    assert!(config.scanner_exclude.is_match(Path::new("dist/bundle.js")));
}

#[test]
fn scan_all_with_structure_merges_results() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    std::fs::write(temp_dir1.path().join("a.rs"), "").unwrap();
    std::fs::write(temp_dir2.path().join("b.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let paths = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
    ];
    let result = scanner.scan_all_with_structure(&paths, None).unwrap();

    assert_eq!(result.files.len(), 2);
    assert!(result.dir_stats.contains_key(temp_dir1.path()));
    assert!(result.dir_stats.contains_key(temp_dir2.path()));
}

#[test]
fn allowlist_rule_no_extension_match_when_file_has_no_extension() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    // File without extension shouldn't match extension-based rule
    assert!(!rule.file_matches(Path::new("src/Makefile")));
}

#[test]
fn structure_scan_config_extract_dir_names_complex() {
    // Test various pattern formats
    let config = StructureScanConfig::new(
        &[],
        &[
            "**/node_modules/**".to_string(),
            "build/**".to_string(),
            "**/*.tmp".to_string(), // Not a dir pattern
        ],
        Vec::new(),
    )
    .unwrap();

    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"node_modules".to_string())
    );
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"build".to_string())
    );
    // *.tmp is not a directory pattern (contains *)
    assert!(
        !config
            .scanner_exclude_dir_names
            .iter()
            .any(|n| n.contains("tmp"))
    );
}

#[test]
fn scan_with_structure_with_count_exclude_does_not_affect_file_list() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("test.gen"), "").unwrap();

    // Exclude .gen from counting but not from file list
    let config = StructureScanConfig::new(&["*.gen".to_string()], &[], Vec::new()).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Both files in file list
    assert_eq!(result.files.len(), 2);
    // Only .rs counted
    assert_eq!(result.dir_stats[&src_dir].file_count, 1);
}

#[test]
fn scan_with_structure_with_scanner_exclude_skips_entirely() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    let vendor = temp_dir.path().join("vendor");
    std::fs::create_dir(&vendor).unwrap();
    std::fs::write(vendor.join("lib.rs"), "").unwrap();

    let config = StructureScanConfig::new(&[], &["**/vendor/**".to_string()], Vec::new()).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // vendor files completely excluded
    assert_eq!(result.files.len(), 1);
    assert!(
        !result
            .files
            .iter()
            .any(|f| f.to_string_lossy().contains("vendor"))
    );
}

#[test]
fn allowlist_rule_matches_directory_partial() {
    let rule = AllowlistRuleBuilder::new("**/src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    assert!(rule.matches_directory(Path::new("project/src/lib")));
    assert!(rule.matches_directory(Path::new("src/nested/deep")));
    assert!(!rule.matches_directory(Path::new("tests/unit")));
}

#[test]
fn find_matching_allowlist_rule_returns_first_match() {
    let rule1 = AllowlistRuleBuilder::new("**/src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let rule2 = AllowlistRuleBuilder::new("**/tests/**".to_string())
        .with_extensions(vec![".rs".to_string(), ".txt".to_string()])
        .build()
        .unwrap();

    let config = StructureScanConfig::new(&[], &[], vec![rule1, rule2]).unwrap();

    let src_rule = config.find_matching_allowlist_rule(Path::new("project/src/lib"));
    assert!(src_rule.is_some());
    assert_eq!(src_rule.unwrap().pattern, "**/src/**");

    let test_rule = config.find_matching_allowlist_rule(Path::new("project/tests/unit"));
    assert!(test_rule.is_some());
    assert_eq!(test_rule.unwrap().pattern, "**/tests/**");

    let none_rule = config.find_matching_allowlist_rule(Path::new("project/docs"));
    assert!(none_rule.is_none());
}

#[test]
fn scan_with_structure_allowlist_violation_includes_rule_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("config.json"), "{}").unwrap();

    let rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![rule]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert_eq!(
        result.allowlist_violations[0].triggering_rule_pattern,
        Some("**/src".to_string())
    );
}

#[test]
fn scan_with_structure_dir_excluded_by_name_match() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    let target_dir = temp_dir.path().join("target");
    std::fs::create_dir(&target_dir).unwrap();
    std::fs::write(target_dir.join("build.rs"), "").unwrap();

    // Use pattern that matches files inside target - use **/target/** for both dir name and file matching
    let config = StructureScanConfig::new(&[], &["**/target/**".to_string()], Vec::new()).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // target directory files should be excluded
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
}

#[test]
fn allowlist_rule_extension_match_with_dot() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string(), ".toml".to_string()])
        .build()
        .unwrap();

    // Match files with these extensions
    assert!(rule.file_matches(Path::new("src/main.rs")));
    assert!(rule.file_matches(Path::new("src/Cargo.toml")));
    assert!(!rule.file_matches(Path::new("src/data.json")));
}

#[test]
fn structure_scan_config_pattern_without_trailing_glob() {
    // Pattern without /** shouldn't extract dir name
    let config = StructureScanConfig::new(&[], &["*.log".to_string()], Vec::new()).unwrap();
    assert!(config.scanner_exclude_dir_names.is_empty());
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

    // dist files should be excluded via CompositeScanner's exclude patterns
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("main.rs"));
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

#[test]
fn structure_scan_config_find_no_matching_rule() {
    let rule = AllowlistRuleBuilder::new("**/src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![rule]).unwrap();

    // Path that doesn't match any rule
    let result = config.find_matching_allowlist_rule(Path::new("docs/readme"));
    assert!(result.is_none());
}

#[test]
fn scan_with_structure_nested_directory_depth() {
    let temp_dir = TempDir::new().unwrap();
    let deep = temp_dir.path().join("a").join("b").join("c").join("d");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("file.rs"), "").unwrap();

    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Check depth tracking is correct
    let deep_stats = result.dir_stats.get(&deep);
    assert!(deep_stats.is_some());
    assert_eq!(deep_stats.unwrap().depth, 4);
}

// =============================================================================
// DirectoryScanner .gitignore Support Tests (Non-Git Repo)
// =============================================================================

#[test]
fn directory_scanner_with_gitignore_respects_gitignore_without_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    // Note: NOT initializing a git repo

    // Create .gitignore
    std::fs::write(temp_dir.path().join(".gitignore"), "ignored/\n*.log\n").unwrap();

    // Create files
    std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(temp_dir.path().join("debug.log"), "log content").unwrap();

    let ignored_dir = temp_dir.path().join("ignored");
    std::fs::create_dir(&ignored_dir).unwrap();
    std::fs::write(ignored_dir.join("hidden.rs"), "ignored content").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    // Should find .gitignore and main.rs, but not debug.log or ignored/hidden.rs
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

    // use_gitignore = false
    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, false);
    let files = scanner.scan(temp_dir.path()).unwrap();

    // Should find all files including debug.log
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
    // When .gitignore doesn't exist, should work normally
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("test.log"), "").unwrap();

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    // Should find all files since no .gitignore exists
    assert_eq!(files.len(), 2);
}

#[test]
fn composite_scanner_fallback_respects_gitignore() {
    // Test that DirectoryScanner with use_gitignore respects .gitignore
    // Note: We test DirectoryScanner directly because CompositeScanner's fallback
    // only triggers on GitRepoNotFound, but temp dirs may be inside a git repo
    // (e.g., during tarpaulin runs with TEMP=.tmp)
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();
    std::fs::write(temp_dir.path().join("debug.log"), "").unwrap();

    // Use DirectoryScanner with gitignore support directly
    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let files = scanner.scan(temp_dir.path()).unwrap();

    // Should respect .gitignore
    assert!(files.iter().any(|f| f.ends_with("main.rs")));
    assert!(!files.iter().any(|f| f.ends_with("debug.log")));
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
    // Add files to logs
    for i in 0..5 {
        std::fs::write(logs_dir.join(format!("log{i}.txt")), "").unwrap();
    }

    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // logs directory should be ignored
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

    // Ignore all .log files except important.log
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

#[test]
fn composite_scanner_fallback_respects_gitignore_scan_with_structure() {
    // Test DirectoryScanner with gitignore + scan_with_structure
    // Note: We test DirectoryScanner directly because CompositeScanner's fallback
    // only triggers on GitRepoNotFound, but temp dirs may be inside a git repo
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join(".gitignore"), "build/\n").unwrap();
    std::fs::write(temp_dir.path().join("main.rs"), "").unwrap();

    let build_dir = temp_dir.path().join("build");
    std::fs::create_dir(&build_dir).unwrap();
    std::fs::write(build_dir.join("output.js"), "").unwrap();

    // Use DirectoryScanner with gitignore support directly
    let scanner = DirectoryScanner::with_gitignore(AcceptAllFilter, true);
    let result = scanner.scan_with_structure(temp_dir.path(), None).unwrap();

    // Should respect .gitignore
    assert!(
        !result
            .files
            .iter()
            .any(|f| f.to_string_lossy().contains("build"))
    );
}

// =============================================================================
// Naming Pattern Tests
// =============================================================================

#[test]
fn allowlist_rule_builder_with_naming_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    assert!(rule.naming_pattern_str.is_some());
    assert_eq!(
        rule.naming_pattern_str.unwrap(),
        "^[A-Z][a-zA-Z0-9]*\\.tsx$"
    );
}

#[test]
fn allowlist_rule_naming_pattern_matches_valid_filename() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();

    // PascalCase filenames should match
    assert!(rule.filename_matches_naming_pattern(Path::new("Button.tsx")));
    assert!(rule.filename_matches_naming_pattern(Path::new("UserProfile.tsx")));
    assert!(rule.filename_matches_naming_pattern(Path::new("A.tsx")));
}

#[test]
fn allowlist_rule_naming_pattern_rejects_invalid_filename() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();

    // Non-PascalCase filenames should not match
    assert!(!rule.filename_matches_naming_pattern(Path::new("button.tsx")));
    assert!(!rule.filename_matches_naming_pattern(Path::new("userProfile.tsx")));
    assert!(!rule.filename_matches_naming_pattern(Path::new("user_profile.tsx")));
    assert!(!rule.filename_matches_naming_pattern(Path::new("Button.ts"))); // wrong extension
}

#[test]
fn allowlist_rule_no_naming_pattern_accepts_all() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    // Without naming pattern, all filenames should pass
    assert!(rule.filename_matches_naming_pattern(Path::new("anything.rs")));
    assert!(rule.filename_matches_naming_pattern(Path::new("123.txt")));
    assert!(rule.filename_matches_naming_pattern(Path::new("UPPERCASE.rs")));
}

#[test]
fn allowlist_rule_builder_invalid_regex_returns_error() {
    let result = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("[invalid".to_string()))
        .build();
    assert!(result.is_err());
}

#[test]
fn scan_with_structure_detects_naming_violations() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Button.tsx"), "").unwrap();
    std::fs::write(src_dir.join("userCard.tsx"), "").unwrap(); // Invalid: camelCase

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // userCard.tsx should be a naming violation
    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("userCard.tsx"));
    assert!(matches!(
        result.allowlist_violations[0].violation_type,
        crate::checker::ViolationType::NamingConvention { .. }
    ));
}

#[test]
fn scan_with_structure_no_naming_violation_when_pattern_matches() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Button.tsx"), "").unwrap();
    std::fs::write(src_dir.join("UserCard.tsx"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // All filenames match PascalCase, no violations
    assert!(result.allowlist_violations.is_empty());
}

#[test]
fn scan_with_structure_naming_violation_includes_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("lowercase.tsx"), "").unwrap();

    let pattern = "^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string();
    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_naming_pattern(Some(pattern.clone()))
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    match &result.allowlist_violations[0].violation_type {
        crate::checker::ViolationType::NamingConvention { expected_pattern } => {
            assert_eq!(expected_pattern, &pattern);
        }
        _ => panic!("Expected NamingConvention violation"),
    }
}

#[test]
fn scan_with_structure_combined_allowlist_and_naming_violations() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Button.tsx"), "").unwrap(); // Valid
    std::fs::write(src_dir.join("button.tsx"), "").unwrap(); // Naming violation
    std::fs::write(src_dir.join("config.json"), "").unwrap(); // Allowlist violation (wrong extension)

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".tsx".to_string()])
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![allowlist_rule]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Two violations: config.json (disallowed) and button.tsx (naming)
    assert_eq!(result.allowlist_violations.len(), 2);
}

