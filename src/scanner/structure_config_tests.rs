use std::path::Path;

use super::*;

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
// StructureScanConfig Tests
// =============================================================================

#[test]
fn structure_scan_config_new_creates_config() {
    let config = StructureScanConfig::new(&[], &[], Vec::new(), Vec::new(), &[], &[]).unwrap();
    assert!(config.allowlist_rules.is_empty());
}

#[test]
fn structure_scan_config_with_count_exclude() {
    let config = StructureScanConfig::new(
        &["*.generated".to_string()],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();
    let path = Path::new("foo.generated");
    assert!(config.count_exclude.is_match(path));
}

#[test]
fn structure_scan_config_with_scanner_exclude() {
    let config = StructureScanConfig::new(
        &[],
        &["**/target/**".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();
    let path = Path::new("src/target/build.rs");
    assert!(config.scanner_exclude.is_match(path));
}

#[test]
fn structure_scan_config_extracts_dir_names() {
    let config = StructureScanConfig::new(
        &[],
        &["target/**".to_string(), "node_modules/**".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
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
    let result = StructureScanConfig::new(
        &["[invalid".to_string()],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    );
    assert!(result.is_err());
}

#[test]
fn structure_scan_config_is_scanner_excluded_file() {
    let config = StructureScanConfig::new(
        &[],
        &["*.lock".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();
    assert!(config.scanner_exclude.is_match(Path::new("Cargo.lock")));
    assert!(!config.scanner_exclude.is_match(Path::new("Cargo.toml")));
}

#[test]
fn structure_scan_config_is_count_excluded() {
    let config = StructureScanConfig::new(
        &["*.generated.rs".to_string()],
        &[],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();
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
    let config = StructureScanConfig::new(&[], &[], vec![rule], Vec::new(), &[], &[]).unwrap();

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
fn structure_scan_config_extract_dir_names_windows_paths() {
    let config = StructureScanConfig::new(
        &[],
        &["target\\**".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"target".to_string())
    );
}

#[test]
fn structure_scan_config_is_scanner_excluded_by_dir_name() {
    let config = StructureScanConfig::new(
        &[],
        &["target/**".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();
    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"target".to_string())
    );
}

#[test]
fn structure_scan_config_empty_patterns_match_nothing() {
    let config = StructureScanConfig::new(&[], &[], Vec::new(), Vec::new(), &[], &[]).unwrap();
    assert!(!config.count_exclude.is_match(Path::new("any.rs")));
    assert!(!config.scanner_exclude.is_match(Path::new("any.rs")));
}

#[test]
fn structure_scan_config_combined_patterns() {
    let config = StructureScanConfig::new(
        &["*.gen".to_string()],
        &["vendor/**".to_string(), "dist/**".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();

    assert!(config.count_exclude.is_match(Path::new("foo.gen")));
    assert!(config.scanner_exclude.is_match(Path::new("vendor/lib.rs")));
    assert!(config.scanner_exclude.is_match(Path::new("dist/bundle.js")));
}

#[test]
fn structure_scan_config_is_scanner_excluded_directory_by_name() {
    let config = StructureScanConfig::new(
        &[],
        &["node_modules/**".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();

    assert!(
        config
            .scanner_exclude_dir_names
            .contains(&"node_modules".to_string())
    );
}

#[test]
fn structure_scan_config_extract_dir_names_complex() {
    let config = StructureScanConfig::new(
        &[],
        &[
            "**/node_modules/**".to_string(),
            "build/**".to_string(),
            "**/*.tmp".to_string(), // Not a dir pattern
        ],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
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
    assert!(
        !config
            .scanner_exclude_dir_names
            .iter()
            .any(|n| n.contains("tmp"))
    );
}

#[test]
fn structure_scan_config_pattern_without_trailing_glob() {
    let config = StructureScanConfig::new(
        &[],
        &["*.log".to_string()],
        Vec::new(),
        Vec::new(),
        &[],
        &[],
    )
    .unwrap();
    assert!(config.scanner_exclude_dir_names.is_empty());
}

#[test]
fn structure_scan_config_find_no_matching_rule() {
    let rule = AllowlistRuleBuilder::new("**/src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    let config = StructureScanConfig::new(&[], &[], vec![rule], Vec::new(), &[], &[]).unwrap();

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

    let config =
        StructureScanConfig::new(&[], &[], vec![rule1, rule2], Vec::new(), &[], &[]).unwrap();

    let src_rule = config.find_matching_allowlist_rule(Path::new("project/src/lib"));
    assert!(src_rule.is_some());
    assert_eq!(src_rule.unwrap().pattern, "**/src/**");

    let test_rule = config.find_matching_allowlist_rule(Path::new("project/tests/unit"));
    assert!(test_rule.is_some());
    assert_eq!(test_rule.unwrap().pattern, "**/tests/**");

    let none_rule = config.find_matching_allowlist_rule(Path::new("project/docs"));
    assert!(none_rule.is_none());
}
