//! Basic structure checker tests: enabling conditions, simple violations.

use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

#[test]
fn checker_not_enabled_by_default() {
    let checker = StructureChecker::new(&default_config()).unwrap();
    assert!(!checker.is_enabled());
}

#[test]
fn checker_enabled_with_file_limit() {
    let checker = StructureChecker::new(&config_with_file_limit(10)).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn checker_enabled_with_dir_limit() {
    let checker = StructureChecker::new(&config_with_dir_limit(5)).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn checker_enabled_with_rules() {
    let config = StructureConfig {
        rules: vec![make_rule("src/**", Some(10))],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn checker_enabled_with_allowlist_rule() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![".rs".to_string()],
            allow_patterns: vec![],
            allow_files: vec![],
            allow_dirs: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],
            deny_files: vec![],
            deny_dirs: vec![],
            reason: None,
            expires: None,
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn check_empty_stats_returns_no_violations() {
    let checker = StructureChecker::new(&config_with_file_limit(10)).unwrap();
    let stats = HashMap::new();

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn check_under_limit_returns_no_violations() {
    let checker = StructureChecker::new(&config_with_file_limit(10)).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 5,
            dir_count: 2,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn violations_sorted_by_path() {
    let checker = StructureChecker::new(&config_with_file_limit(5)).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("z_dir"),
        DirStats {
            file_count: 10,
            dir_count: 0,
            depth: 0,
        },
    );
    stats.insert(
        PathBuf::from("a_dir"),
        DirStats {
            file_count: 10,
            dir_count: 0,
            depth: 0,
        },
    );
    stats.insert(
        PathBuf::from("m_dir"),
        DirStats {
            file_count: 10,
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 3);
    assert_eq!(violations[0].path, PathBuf::from("a_dir"));
    assert_eq!(violations[1].path, PathBuf::from("m_dir"));
    assert_eq!(violations[2].path, PathBuf::from("z_dir"));
}

#[test]
fn dir_stats_default() {
    let stats = DirStats::default();
    assert_eq!(stats.file_count, 0);
    assert_eq!(stats.dir_count, 0);
}

#[test]
fn violation_type_equality() {
    assert_eq!(ViolationType::FileCount, ViolationType::FileCount);
    assert_eq!(ViolationType::DirCount, ViolationType::DirCount);
    assert_ne!(ViolationType::FileCount, ViolationType::DirCount);
}

#[test]
fn structure_violation_new() {
    let violation =
        StructureViolation::new(PathBuf::from("src"), ViolationType::FileCount, 15, 10, None);

    assert_eq!(violation.path, PathBuf::from("src"));
    assert_eq!(violation.violation_type, ViolationType::FileCount);
    assert_eq!(violation.actual, 15);
    assert_eq!(violation.limit, 10);
    assert_eq!(violation.override_reason, None);
}

#[test]
fn invalid_rule_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "[invalid".to_string(),
            max_files: Some(10),
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            allow_files: vec![],
            allow_dirs: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],
            deny_files: vec![],
            deny_dirs: vec![],
            reason: None,
            expires: None,
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}
