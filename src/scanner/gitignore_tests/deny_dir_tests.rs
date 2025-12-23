//! Tests for `deny_dirs` functionality in `GitAwareScanner`.
//!
//! Covers: global basename matching, directory patterns with slashes, per-rule `deny_dirs`,
//! nested structures, glob patterns, duplicate violation prevention, and combined file/dir denial.

use super::super::{FileScanner, GitAwareScanner, StructureScanConfig};
use super::mock_filters::{AcceptAllFilter, init_git_repo};
use crate::scanner::AllowlistRuleBuilder;
use crate::scanner::TestConfigParams;
use tempfile::TempDir;

#[test]
fn global_deny_dirs_basename() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_dirs: vec!["node_modules".to_string(), "__pycache__".to_string()],
        ..Default::default()
    })
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
fn global_deny_dirs_pattern_with_slash() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["build/".to_string(), "**/dist/".to_string()],
        ..Default::default()
    })
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
fn per_rule_deny_dirs() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
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
fn nested_deny_dirs() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_dirs: vec!["__tests__".to_string()],
        ..Default::default()
    })
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
fn deny_dirs_glob_pattern() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_dirs: vec!["test_*".to_string(), "*_backup".to_string()],
        ..Default::default()
    })
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
fn deny_dirs_no_duplicate_violations() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_dirs: vec!["node_modules".to_string()],
        ..Default::default()
    })
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
fn deny_dirs_combined_with_file_deny() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_files: vec!["secrets.json".to_string()],
        global_deny_dirs: vec!["__pycache__".to_string()],
        ..Default::default()
    })
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
fn per_rule_deny_dirs_nested_rule_scope() {
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
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
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
