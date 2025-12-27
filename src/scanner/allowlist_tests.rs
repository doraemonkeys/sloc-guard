use std::path::Path;

use super::*;
use crate::scanner::TestConfigParams;

// =============================================================================
// matches_directory with dot-prefixed paths Tests (regression tests for scope matching bug)
// =============================================================================

#[test]
fn allowlist_rule_matches_directory_with_dot_prefix() {
    // Scope pattern like {src,src/**} should match both ./src and src
    let rule = AllowlistRuleBuilder::new("{src,src/**}".to_string())
        .build()
        .unwrap();

    // Should match without dot prefix
    assert!(rule.matches_directory(Path::new("src")));
    assert!(rule.matches_directory(Path::new("src/lib")));
    assert!(rule.matches_directory(Path::new("src/output")));

    // Should also match WITH dot prefix (the bug fix)
    assert!(rule.matches_directory(Path::new("./src")));
    assert!(rule.matches_directory(Path::new("./src/lib")));
    assert!(rule.matches_directory(Path::new("./src/output")));
}

#[test]
fn allowlist_rule_matches_directory_with_backslash_dot_prefix() {
    // Windows-style paths
    let rule = AllowlistRuleBuilder::new("{src,src/**}".to_string())
        .build()
        .unwrap();

    // Should match with backslash paths
    assert!(rule.matches_directory(Path::new(".\\src")));
    assert!(rule.matches_directory(Path::new(".\\src\\lib")));
}

#[test]
fn allowlist_rule_matches_directory_glob_star_with_dot_prefix() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    // Without dot prefix
    assert!(rule.matches_directory(Path::new("src/lib")));

    // With dot prefix
    assert!(rule.matches_directory(Path::new("./src/lib")));
}

#[test]
fn allowlist_rule_matches_directory_double_star_prefix_with_dot() {
    let rule = AllowlistRuleBuilder::new("**/src".to_string())
        .build()
        .unwrap();

    // Should match paths with and without dot prefix
    assert!(rule.matches_directory(Path::new("project/src")));
    assert!(rule.matches_directory(Path::new("./project/src")));
}

#[test]
fn allowlist_rule_builder_creates_rule() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    assert_eq!(rule.scope, "src/**");
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

#[test]
fn allowlist_rule_file_no_match_empty_allowlist() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![])
        .with_patterns(vec![])
        .build()
        .unwrap();
    assert!(!rule.file_matches(Path::new("src/main.rs")));
}

#[test]
fn allowlist_rule_builder_invalid_allow_pattern() {
    let result = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["[invalid".to_string()])
        .build();
    assert!(result.is_err());
}

#[test]
fn allowlist_rule_empty_extension_list() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![])
        .build()
        .unwrap();
    assert!(rule.allow_extensions.is_empty());
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
fn allowlist_rule_file_matches_by_full_path() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["**/special.txt".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/nested/special.txt")));
}

#[test]
fn allowlist_rule_no_extension_match_when_file_has_no_extension() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    assert!(!rule.file_matches(Path::new("src/Makefile")));
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
fn allowlist_rule_extension_match_with_dot() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string(), ".toml".to_string()])
        .build()
        .unwrap();

    assert!(rule.file_matches(Path::new("src/main.rs")));
    assert!(rule.file_matches(Path::new("src/Cargo.toml")));
    assert!(!rule.file_matches(Path::new("src/data.json")));
}

// =============================================================================
// Allow Files/Dirs Tests
// =============================================================================

#[test]
fn allowlist_rule_builder_with_allow_files() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_allow_files(vec!["Cargo.*".to_string(), "README*".to_string()])
        .build()
        .unwrap();
    assert_eq!(rule.allow_file_strs.len(), 2);
}

#[test]
fn allowlist_rule_builder_with_allow_dirs() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_allow_dirs(vec!["utils".to_string(), "test*".to_string()])
        .build()
        .unwrap();
    assert_eq!(rule.allow_dir_strs.len(), 2);
}

#[test]
fn allowlist_rule_has_dir_allowlist_empty() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    assert!(!rule.has_dir_allowlist());
}

#[test]
fn allowlist_rule_has_dir_allowlist_with_dirs() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_allow_dirs(vec!["utils".to_string()])
        .build()
        .unwrap();
    assert!(rule.has_dir_allowlist());
}

#[test]
fn allowlist_rule_dir_matches() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_allow_dirs(vec!["utils".to_string(), "test*".to_string()])
        .build()
        .unwrap();
    assert!(rule.dir_matches(Path::new("src/utils")));
    assert!(rule.dir_matches(Path::new("src/tests")));
    assert!(rule.dir_matches(Path::new("src/test_helpers")));
    assert!(!rule.dir_matches(Path::new("src/vendor")));
}

#[test]
fn allowlist_rule_dir_matches_empty_allowlist() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_allow_dirs(vec![])
        .build()
        .unwrap();
    assert!(!rule.dir_matches(Path::new("src/utils")));
}

#[test]
fn allowlist_rule_file_matches_allow_files() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_allow_files(vec!["*.config".to_string(), "Makefile".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/app.config")));
    assert!(rule.file_matches(Path::new("src/Makefile")));
    assert!(!rule.file_matches(Path::new("src/main.rs")));
}

#[test]
fn allowlist_rule_has_allowlist_with_allow_files() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_allow_files(vec!["Cargo.*".to_string()])
        .build()
        .unwrap();
    assert!(rule.has_allowlist());
}

#[test]
fn allowlist_rule_has_allowlist_empty() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .build()
        .unwrap();
    assert!(!rule.has_allowlist());
}

// =============================================================================
// Deny Pattern Tests
// =============================================================================

#[test]
fn allowlist_rule_file_matches_deny_extension() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_extensions(vec![".exe".to_string(), ".dll".to_string()])
        .build()
        .unwrap();

    assert!(rule.file_matches_deny(Path::new("src/app.exe")).is_some());
    assert!(rule.file_matches_deny(Path::new("src/lib.dll")).is_some());
    assert!(rule.file_matches_deny(Path::new("src/main.rs")).is_none());
}

#[test]
fn allowlist_rule_file_matches_deny_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_patterns(vec!["*.bak".to_string(), "temp_*".to_string()])
        .build()
        .unwrap();

    assert!(rule.file_matches_deny(Path::new("src/file.bak")).is_some());
    assert!(
        rule.file_matches_deny(Path::new("src/temp_data.txt"))
            .is_some()
    );
    assert!(rule.file_matches_deny(Path::new("src/main.rs")).is_none());
}

#[test]
fn allowlist_rule_file_matches_deny_file_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_deny_files(vec!["*.bak".to_string(), "temp_*".to_string()])
        .build()
        .unwrap();

    // Should match filename patterns
    assert!(rule.file_matches_deny(Path::new("src/file.bak")).is_some());
    assert!(
        rule.file_matches_deny(Path::new("src/temp_data.txt"))
            .is_some()
    );
    assert!(rule.file_matches_deny(Path::new("src/main.rs")).is_none());
}

// =============================================================================
// Per-Rule Deny Integration Tests
// =============================================================================

use crate::checker::ViolationType;
use tempfile::TempDir;

struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
}

#[test]
fn per_rule_deny_extensions_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.ts"), "").unwrap();
    std::fs::write(src_dir.join("legacy.js"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_extensions(vec![".js".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("legacy.js"));
}

#[test]
fn per_rule_deny_patterns_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("temp_cache.txt"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_patterns(vec!["temp_*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("temp_cache.txt")
    );
}

#[test]
fn deny_takes_precedence_over_allowlist() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("backup.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .with_deny_patterns(vec!["backup*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("backup.rs"));
    matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedFile { .. }
    );
}

#[test]
fn per_rule_deny_dirs_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();

    let temp_dir_child = src_dir.join("temp");
    std::fs::create_dir(&temp_dir_child).unwrap();
    std::fs::write(temp_dir_child.join("file.rs"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_dirs(vec!["temp*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("temp"));
    assert!(matches!(
        &result.allowlist_violations[0].violation_type,
        ViolationType::DeniedDirectory { .. }
    ));
}

#[test]
fn per_rule_deny_file_patterns_trigger_violation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("temp_cache.txt"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_deny_files(vec!["temp_*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("temp_cache.txt")
    );
}

#[test]
fn deny_file_patterns_takes_precedence_over_allowlist() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.rs"), "").unwrap();
    std::fs::write(src_dir.join("backup.rs"), "").unwrap();

    // Even though .rs is allowed, backup* should be denied
    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .with_deny_files(vec!["backup*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("backup.rs"));
}

#[test]
fn per_rule_deny_still_takes_precedence_over_per_rule_allow() {
    // Within the same rule, deny should still take precedence over allow
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("allowed.rs"), "").unwrap();
    std::fs::write(src_dir.join("backup.rs"), "").unwrap();

    // Rule allows .rs but denies backup*
    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".rs".to_string()])
        .with_deny_files(vec!["backup*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // backup.rs should be denied even though .rs is allowed
    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(result.allowlist_violations[0].path.ends_with("backup.rs"));
}

// =============================================================================
// Per-Rule Allow Overrides Global Deny Tests
// =============================================================================

#[test]
fn per_rule_allow_files_overrides_global_deny_files() {
    // Global deny_files = ["secrets.*"] denies secrets.* everywhere
    // But [[structure.rules]] scope="**/docs" allow_files=["secrets.md"] should allow docs/secrets.md
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let docs_dir = temp_dir.path().join("docs");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&docs_dir).unwrap();

    // This should be denied (global deny, no per-rule allow)
    std::fs::write(src_dir.join("secrets.json"), "").unwrap();
    // This should be allowed (per-rule allow overrides global deny)
    std::fs::write(docs_dir.join("secrets.md"), "").unwrap();
    // This should still be denied in docs (not in allow_files list)
    std::fs::write(docs_dir.join("secrets.env"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/docs".to_string())
        .with_allow_files(vec!["secrets.md".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        global_deny_files: vec!["secrets.*".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Only secrets.json and secrets.env should be denied, secrets.md should be allowed
    assert_eq!(
        result.allowlist_violations.len(),
        2,
        "Expected 2 violations: secrets.json (global deny) and secrets.env (global deny, not in allow_files). Got: {:?}",
        result
            .allowlist_violations
            .iter()
            .map(|v| &v.path)
            .collect::<Vec<_>>()
    );

    let violation_filenames: Vec<_> = result
        .allowlist_violations
        .iter()
        .filter_map(|v| v.path.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();
    assert!(violation_filenames.contains(&"secrets.json".to_string()));
    assert!(violation_filenames.contains(&"secrets.env".to_string()));
    assert!(!violation_filenames.contains(&"secrets.md".to_string()));
}

#[test]
fn per_rule_allow_extensions_overrides_global_deny_extensions() {
    // Global deny_extensions = [".exe"] denies .exe everywhere
    // But [[structure.rules]] scope="**/tools" allow_extensions=[".exe"] should allow tools/*.exe
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let tools_dir = temp_dir.path().join("tools");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&tools_dir).unwrap();

    // This should be denied (global deny, no per-rule allow)
    std::fs::write(src_dir.join("malware.exe"), "").unwrap();
    // This should be allowed (per-rule allow_extensions overrides global deny)
    std::fs::write(tools_dir.join("helper.exe"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/tools".to_string())
        .with_extensions(vec![".exe".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        global_deny_extensions: vec![".exe".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Only malware.exe should be denied, helper.exe should be allowed
    assert_eq!(
        result.allowlist_violations.len(),
        1,
        "Expected 1 violation: malware.exe. Got: {:?}",
        result
            .allowlist_violations
            .iter()
            .map(|v| &v.path)
            .collect::<Vec<_>>()
    );
    assert!(result.allowlist_violations[0].path.ends_with("malware.exe"));
}

#[test]
fn per_rule_allow_patterns_overrides_global_deny_patterns() {
    // Global deny_patterns = ["*.bak"] denies *.bak everywhere
    // But [[structure.rules]] scope="**/backup" allow_patterns=["*.bak"] should allow backup/*.bak
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let backup_dir = temp_dir.path().join("backup");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&backup_dir).unwrap();

    // This should be denied (global deny, no per-rule allow)
    std::fs::write(src_dir.join("temp.bak"), "").unwrap();
    // This should be allowed (per-rule allow_patterns overrides global deny)
    std::fs::write(backup_dir.join("data.bak"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/backup".to_string())
        .with_patterns(vec!["*.bak".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        global_deny_patterns: vec!["*.bak".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Only temp.bak should be denied, data.bak should be allowed
    assert_eq!(
        result.allowlist_violations.len(),
        1,
        "Expected 1 violation: temp.bak. Got: {:?}",
        result
            .allowlist_violations
            .iter()
            .map(|v| &v.path)
            .collect::<Vec<_>>()
    );
    assert!(result.allowlist_violations[0].path.ends_with("temp.bak"));
}

#[test]
fn per_rule_allow_dirs_overrides_global_deny_dirs() {
    // Global deny_dirs = ["temp*"] denies temp* directories everywhere
    // But [[structure.rules]] scope="**/workspace" allow_dirs=["temp*"] should allow workspace/temp*
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let workspace_dir = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&workspace_dir).unwrap();

    // This should be denied (global deny, no per-rule allow)
    let temp_cache = src_dir.join("temp_cache");
    std::fs::create_dir(&temp_cache).unwrap();
    std::fs::write(temp_cache.join("file.txt"), "").unwrap();

    // This should be allowed (per-rule allow_dirs overrides global deny)
    let temp_data = workspace_dir.join("temp_data");
    std::fs::create_dir(&temp_data).unwrap();
    std::fs::write(temp_data.join("file.txt"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/workspace".to_string())
        .with_allow_dirs(vec!["temp*".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![allowlist_rule],
        global_deny_dirs: vec!["temp*".to_string()],
        ..Default::default()
    })
    .unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Only temp_cache should be denied, temp_data should be allowed
    let dir_violations: Vec<_> = result
        .allowlist_violations
        .iter()
        .filter(|v| matches!(v.violation_type, ViolationType::DeniedDirectory { .. }))
        .collect();
    assert_eq!(
        dir_violations.len(),
        1,
        "Expected 1 directory violation: temp_cache. Got: {:?}",
        dir_violations.iter().map(|v| &v.path).collect::<Vec<_>>()
    );
    assert!(dir_violations[0].path.ends_with("temp_cache"));
}
