use std::path::{Path, PathBuf};

use super::*;
use crate::project::ProjectPaths;
use crate::scanner::TestConfigParams;

// =============================================================================
// ScanResult Tests
// =============================================================================

#[test]
fn scan_result_default() {
    let result = ScanResult::default();
    assert!(result.files.is_empty());
    assert!(result.dir_stats.is_empty());
    assert!(result.allowlist_violations.is_empty());
}

// =============================================================================
// StructureScanConfig Core Tests
// =============================================================================

#[test]
fn structure_scan_config_new_creates_config() {
    let config = StructureScanConfig::new(TestConfigParams::default()).unwrap();
    assert!(config.allowlist_rules.is_empty());
}

#[test]
fn structure_scan_config_with_count_exclude() {
    let config = StructureScanConfig::new(TestConfigParams {
        count_exclude_patterns: vec!["*.generated".to_string()],
        ..Default::default()
    })
    .unwrap();
    let path = Path::new("foo.generated");
    assert!(config.count_exclude.is_match(path));
}

#[test]
fn structure_scan_config_with_scanner_exclude() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["**/target/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    let path = Path::new("src/target/build.rs");
    assert!(config.scanner_exclude.is_match(path));
}

#[test]
fn scanner_exclude_directory_pattern_is_anchored_to_its_logical_path() {
    let project_root = PathBuf::from("/repo");
    let project_paths = ProjectPaths::rooted_with_cwd(project_root.clone(), project_root.clone());
    let config = StructureScanConfig::builder()
        .scanner_exclude(vec!["core/vendor/**".to_string()])
        .build()
        .unwrap();

    assert!(config.is_scanner_excluded_with_project_paths(
        &project_root.join("core/vendor"),
        true,
        &project_paths,
    ));
    assert!(!config.is_scanner_excluded_with_project_paths(
        &project_root.join("other/vendor"),
        true,
        &project_paths,
    ));
}

#[test]
fn scanner_exclusions_do_not_gain_implicit_basename_semantics() {
    let project_root = PathBuf::from("/repo");
    let project_paths = ProjectPaths::rooted_with_cwd(project_root.clone(), project_root.clone());
    let config = StructureScanConfig::builder()
        .scanner_exclude(vec!["vendor/**".to_string(), "secret.rs".to_string()])
        .build()
        .unwrap();

    assert!(config.is_scanner_excluded_with_project_paths(
        &project_root.join("vendor"),
        true,
        &project_paths,
    ));
    assert!(!config.is_scanner_excluded_with_project_paths(
        &project_root.join("apps/vendor"),
        true,
        &project_paths,
    ));
    assert!(config.is_scanner_excluded_with_project_paths(
        &project_root.join("secret.rs"),
        false,
        &project_paths,
    ));
    assert!(!config.is_scanner_excluded_with_project_paths(
        &project_root.join("nested/secret.rs"),
        false,
        &project_paths,
    ));
}

#[test]
fn file_only_exclusion_does_not_prune_a_same_named_directory() {
    let project_root = PathBuf::from("/repo");
    let project_paths = ProjectPaths::rooted_with_cwd(project_root.clone(), project_root.clone());
    let config = StructureScanConfig::builder()
        .scanner_exclude(vec!["vendor".to_string()])
        .build()
        .unwrap();
    let path = project_root.join("vendor");

    assert!(config.is_scanner_excluded_with_project_paths(&path, false, &project_paths,));
    assert!(!config.is_scanner_excluded_with_project_paths(&path, true, &project_paths,));
}

#[test]
fn structure_scan_config_extracts_directory_prefixes() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["target/**".to_string(), "node_modules/**".to_string()],
        ..Default::default()
    })
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
    let result = StructureScanConfig::new(TestConfigParams {
        count_exclude_patterns: vec!["[invalid".to_string()],
        ..Default::default()
    });
    assert!(result.is_err());
}

#[test]
fn structure_scan_config_is_scanner_excluded_file() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["*.lock".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.scanner_exclude.is_match(Path::new("Cargo.lock")));
    assert!(!config.scanner_exclude.is_match(Path::new("Cargo.toml")));
}

#[test]
fn structure_scan_config_is_count_excluded() {
    let config = StructureScanConfig::new(TestConfigParams {
        count_exclude_patterns: vec!["*.generated.rs".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(
        config
            .count_exclude
            .is_match(Path::new("types.generated.rs"))
    );
    assert!(!config.count_exclude.is_match(Path::new("types.rs")));
}

#[test]
fn count_exclude_preserves_root_anchors_and_basename_patterns() {
    let project_root = PathBuf::from("/repo");
    let project_paths = ProjectPaths::rooted_with_cwd(project_root.clone(), project_root.clone());
    let config = StructureScanConfig::new(TestConfigParams {
        count_exclude_patterns: vec![
            "./root.rs".to_string(),
            "*.tmp".to_string(),
            "./src/**".to_string(),
        ],
        ..Default::default()
    })
    .unwrap();

    assert!(
        config.is_count_excluded_with_project_paths(&project_root.join("root.rs"), &project_paths,)
    );
    assert!(!config.is_count_excluded_with_project_paths(
        &project_root.join("nested/root.rs"),
        &project_paths,
    ));
    assert!(config.is_count_excluded_with_project_paths(
        &project_root.join("nested/cache.tmp"),
        &project_paths,
    ));
    assert!(config.is_count_excluded_with_project_paths(
        &project_root.join("src/generated.rs"),
        &project_paths,
    ));
    assert!(!config.is_count_excluded_with_project_paths(
        &project_root.join("other/src/generated.rs"),
        &project_paths,
    ));
}

#[test]
fn global_deny_patterns_preserve_root_anchors_and_basename_patterns() {
    let config = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["./root.rs".to_string(), "temp_*".to_string()],
        ..Default::default()
    })
    .unwrap();

    assert!(
        config
            .file_matches_global_deny(Path::new("root.rs"))
            .is_some()
    );
    assert!(
        config
            .file_matches_global_deny(Path::new("nested/root.rs"))
            .is_none()
    );
    assert!(
        config
            .file_matches_global_deny(Path::new("nested/temp_cache"))
            .is_some()
    );
}

#[test]
fn directory_only_deny_patterns_preserve_path_qualification() {
    let unqualified = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["node_modules/".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(
        unqualified
            .dir_matches_global_deny(Path::new("src/node_modules"))
            .is_some()
    );

    let root_anchored = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["./node_modules/".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(
        root_anchored
            .dir_matches_global_deny(Path::new("node_modules"))
            .is_some()
    );
    assert!(
        root_anchored
            .dir_matches_global_deny(Path::new("src/node_modules"))
            .is_none()
    );

    let qualified = StructureScanConfig::new(TestConfigParams {
        global_deny_patterns: vec!["src/node_modules/".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(
        qualified
            .dir_matches_global_deny(Path::new("src/node_modules"))
            .is_some()
    );
    assert!(
        qualified
            .dir_matches_global_deny(Path::new("other/src/node_modules"))
            .is_none()
    );
}

#[test]
fn structure_scan_config_find_matching_allowlist_rule() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![rule],
        ..Default::default()
    })
    .unwrap();

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
fn structure_scan_config_extracts_directory_prefixes_from_windows_paths() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["target\\**".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"target".to_string())
    );
}

#[test]
fn structure_scan_config_records_target_directory_prefix() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["target/**".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"target".to_string())
    );
}

#[test]
fn structure_scan_config_empty_patterns_match_nothing() {
    let config = StructureScanConfig::new(TestConfigParams::default()).unwrap();
    assert!(!config.count_exclude.is_match(Path::new("any.rs")));
    assert!(!config.scanner_exclude.is_match(Path::new("any.rs")));
}

#[test]
fn structure_scan_config_combined_patterns() {
    let config = StructureScanConfig::new(TestConfigParams {
        count_exclude_patterns: vec!["*.gen".to_string()],
        scanner_exclude_patterns: vec!["vendor/**".to_string(), "dist/**".to_string()],
        ..Default::default()
    })
    .unwrap();

    assert!(config.count_exclude.is_match(Path::new("foo.gen")));
    assert!(config.scanner_exclude.is_match(Path::new("vendor/lib.rs")));
    assert!(config.scanner_exclude.is_match(Path::new("dist/bundle.js")));
}

#[test]
fn structure_scan_config_records_node_modules_directory_prefix() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["node_modules/**".to_string()],
        ..Default::default()
    })
    .unwrap();

    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"node_modules".to_string())
    );
}

#[test]
fn structure_scan_config_preserves_complete_directory_prefix_globs() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec![
            "**/node_modules/**".to_string(),
            "build/**".to_string(),
            "**/*.tmp".to_string(), // Not a dir pattern
        ],
        ..Default::default()
    })
    .unwrap();

    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"**/node_modules".to_string())
    );
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"build".to_string())
    );
    assert!(
        !config
            .scanner_exclude_dir_names
            .iter()
            .any(|n| n.contains("tmp"))
    );
}

#[test]
fn structure_scan_config_strips_only_one_terminal_recursive_suffix() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["vendor/**/**".to_string()],
        ..Default::default()
    })
    .unwrap();

    assert_eq!(config.scanner_exclude_dir_names, vec!["vendor/**"]);
}

#[test]
fn non_recursive_file_glob_is_not_a_directory_prefix() {
    let project_root = PathBuf::from("/repo");
    let project_paths = ProjectPaths::rooted_with_cwd(project_root.clone(), project_root.clone());
    let config = StructureScanConfig::builder()
        .scanner_exclude(vec!["vendor/[!x]*".to_string()])
        .build()
        .unwrap();

    assert!(config.scanner_exclude_dir_names.is_empty());
    assert!(!config.is_scanner_excluded_with_project_paths(
        &project_root.join("vendor"),
        true,
        &project_paths,
    ));
    assert!(config.is_scanner_excluded_with_project_paths(
        &project_root.join("vendor/a.rs"),
        false,
        &project_paths,
    ));
    assert!(!config.is_scanner_excluded_with_project_paths(
        &project_root.join("vendor/x.rs"),
        false,
        &project_paths,
    ));
}

#[test]
fn structure_scan_config_pattern_without_trailing_glob() {
    let config = StructureScanConfig::new(TestConfigParams {
        scanner_exclude_patterns: vec!["*.log".to_string()],
        ..Default::default()
    })
    .unwrap();
    assert!(config.scanner_exclude_dir_names.is_empty());
}

#[test]
fn structure_scan_config_find_no_matching_rule() {
    let rule = AllowlistRuleBuilder::new("**/src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![rule],
        ..Default::default()
    })
    .unwrap();

    let result = config.find_matching_allowlist_rule(Path::new("docs/readme"));
    assert!(result.is_none());
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

    let config = StructureScanConfig::new(TestConfigParams {
        allowlist_rules: vec![rule1, rule2],
        ..Default::default()
    })
    .unwrap();

    let src_rule = config.find_matching_allowlist_rule(Path::new("project/src/lib"));
    assert!(src_rule.is_some());
    assert_eq!(src_rule.unwrap().scope, "**/src/**");

    let test_rule = config.find_matching_allowlist_rule(Path::new("project/tests/unit"));
    assert!(test_rule.is_some());
    assert_eq!(test_rule.unwrap().scope, "**/tests/**");

    let none_rule = config.find_matching_allowlist_rule(Path::new("project/docs"));
    assert!(none_rule.is_none());
}
