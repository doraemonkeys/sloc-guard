//! File/dir limit violation tests: warnings, unlimited values, validation errors.

use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

// ============================================================================
// File/Dir Limit Violations
// ============================================================================

#[test]
fn check_over_file_limit_returns_violation() {
    let checker = StructureChecker::new(&config_with_file_limit(10)).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 15,
            dir_count: 2,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src"));
    assert_eq!(violations[0].violation_type, ViolationType::FileCount);
    assert_eq!(violations[0].actual, 15);
    assert_eq!(violations[0].limit, 10);
}

#[test]
fn check_over_dir_limit_returns_violation() {
    let checker = StructureChecker::new(&config_with_dir_limit(3)).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 5,
            dir_count: 5,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src"));
    assert_eq!(violations[0].violation_type, ViolationType::DirCount);
    assert_eq!(violations[0].actual, 5);
    assert_eq!(violations[0].limit, 3);
}

#[test]
fn check_both_limits_exceeded_returns_both_violations() {
    let config = StructureConfig {
        max_files: Some(5),
        max_dirs: Some(2),
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 10,
            dir_count: 5,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 2);
}

#[test]
fn rule_overrides_global_limit() {
    let config = StructureConfig {
        max_files: Some(5),
        rules: vec![StructureRule {
            scope: "src/generated/**".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],

            deny_files: vec![],
            deny_dirs: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/generated/protos"),
        DirStats {
            file_count: 50,
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn rule_inherits_unset_limit_from_global() {
    let config = StructureConfig {
        max_files: Some(5),
        max_dirs: Some(3),
        rules: vec![StructureRule {
            scope: "src/generated/**".to_string(),
            max_files: Some(100),
            max_dirs: None, // Should inherit global max_dirs=3
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],

            deny_files: vec![],
            deny_dirs: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/generated/protos"),
        DirStats {
            file_count: 50,
            dir_count: 5, // Exceeds inherited limit of 3
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::DirCount);
    assert_eq!(violations[0].limit, 3);
}

// ============================================================================
// Warning Threshold Tests
// ============================================================================

#[test]
fn warn_threshold_triggers_warning_below_hard_limit() {
    let config = StructureConfig {
        max_files: Some(50),
        warn_threshold: Some(0.9), // Warn at 45 files
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 47, // Above 45 (warn), below 50 (limit)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert!(violations[0].is_warning);
    assert_eq!(violations[0].violation_type, ViolationType::FileCount);
    assert_eq!(violations[0].actual, 47);
    assert_eq!(violations[0].limit, 50);
}

#[test]
fn warn_threshold_no_warning_below_threshold() {
    let config = StructureConfig {
        max_files: Some(50),
        warn_threshold: Some(0.9), // Warn at 45 files
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 44, // Below 45 (warn threshold)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn warn_threshold_error_above_hard_limit() {
    let config = StructureConfig {
        max_files: Some(50),
        warn_threshold: Some(0.9),
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 55, // Above 50 (hard limit)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert!(!violations[0].is_warning); // Not a warning, an error
    assert_eq!(violations[0].violation_type, ViolationType::FileCount);
    assert_eq!(violations[0].actual, 55);
    assert_eq!(violations[0].limit, 50);
}

#[test]
fn warn_threshold_dir_count() {
    let config = StructureConfig {
        max_dirs: Some(10),
        warn_threshold: Some(0.8), // Warn at 8 dirs
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 0,
            dir_count: 9, // Above 8 (warn), below 10 (limit)
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert!(violations[0].is_warning);
    assert_eq!(violations[0].violation_type, ViolationType::DirCount);
}

#[test]
fn warn_threshold_rule_overrides_global() {
    let config = StructureConfig {
        max_files: Some(50),
        warn_threshold: Some(0.9), // Global: warn at 45
        rules: vec![StructureRule {
            scope: "src/special/**".to_string(),
            max_files: None, // Inherit 50
            max_dirs: None,
            max_depth: None,
            warn_threshold: Some(0.5), // Rule: warn at 25
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],

            deny_files: vec![],
            deny_dirs: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/special/dir"),
        DirStats {
            file_count: 30, // Above 25 (rule warn), below 50 (limit)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert!(violations[0].is_warning);
}

#[test]
fn no_warn_threshold_means_no_warnings() {
    let config = StructureConfig {
        max_files: Some(50),
        // No warn_threshold set
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 49, // Just under limit, but no warn_threshold
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    // No violations because we're under the limit and no warn_threshold
    assert!(violations.is_empty());
}

// ============================================================================
// Unlimited (-1) Value Tests
// ============================================================================

#[test]
fn unlimited_file_limit_skips_check() {
    let config = StructureConfig {
        max_files: Some(UNLIMITED),
        max_dirs: Some(2),
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 1000, // Would exceed any normal limit
            dir_count: 1,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    // No file violations because max_files is unlimited
    assert!(violations.is_empty());
}

#[test]
fn unlimited_dir_limit_skips_check() {
    let config = StructureConfig {
        max_files: Some(5),
        max_dirs: Some(UNLIMITED),
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 3,
            dir_count: 100, // Would exceed any normal limit
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    // No dir violations because max_dirs is unlimited
    assert!(violations.is_empty());
}

#[test]
fn rule_can_set_unlimited_to_override_global() {
    let config = StructureConfig {
        max_files: Some(5),
        max_dirs: Some(2),
        rules: vec![StructureRule {
            scope: "src/generated/**".to_string(),
            max_files: Some(UNLIMITED), // Override to unlimited
            max_dirs: None,             // Inherit global (2)
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],

            deny_files: vec![],
            deny_dirs: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/generated/protos"),
        DirStats {
            file_count: 500, // Would exceed global limit but unlimited by rule
            dir_count: 5,    // Exceeds inherited limit of 2
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    // Only dir violation (inherits global limit of 2)
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::DirCount);
    assert_eq!(violations[0].limit, 2);
}

#[test]
fn unconfigured_max_dirs_allows_unlimited_directories() {
    // When max_dirs is not configured (None), directories with any number
    // of subdirectories should not trigger violations.
    let config = StructureConfig {
        max_files: Some(100), // Only file limit is set
        max_dirs: None,       // Directory limit is NOT configured
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        DirStats {
            file_count: 5,
            dir_count: 99, // Large number of subdirectories
            depth: 0,
        },
    );
    stats.insert(
        PathBuf::from("tests"),
        DirStats {
            file_count: 3,
            dir_count: 50, // Another directory with many subdirs
            depth: 0,
        },
    );

    let violations = checker.check(&stats);

    // No violations because max_dirs is not configured
    assert!(violations.is_empty());
}

// ============================================================================
// Validation Error Tests
// ============================================================================

#[test]
fn invalid_max_files_value_returns_error() {
    let config = StructureConfig {
        max_files: Some(-2), // Invalid: less than -1
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_files value"));
    }
}

#[test]
fn invalid_max_dirs_value_returns_error() {
    let config = StructureConfig {
        max_dirs: Some(-5), // Invalid: less than -1
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_dirs value"));
    }
}

#[test]
fn invalid_rule_max_files_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            max_files: Some(-10), // Invalid
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],

            deny_files: vec![],
            deny_dirs: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_files value in rule 1"));
    }
}

#[test]
fn invalid_rule_max_dirs_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            max_files: None,
            max_dirs: Some(-3), // Invalid
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],

            deny_files: vec![],
            deny_dirs: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_dirs value in rule 1"));
    }
}
