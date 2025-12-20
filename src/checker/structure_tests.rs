use std::collections::HashMap;
use std::path::PathBuf;

use super::*;
use crate::config::{StructureConfig, StructureOverride, StructureRule, UNLIMITED};

fn default_config() -> StructureConfig {
    StructureConfig::default()
}

fn config_with_file_limit(max_files: i64) -> StructureConfig {
    StructureConfig {
        max_files: Some(max_files),
        ..Default::default()
    }
}

fn config_with_dir_limit(max_dirs: i64) -> StructureConfig {
    StructureConfig {
        max_dirs: Some(max_dirs),
        ..Default::default()
    }
}

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
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: Some(10),
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
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
            pattern: "src/generated/**".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
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
            pattern: "src/generated/**".to_string(),
            max_files: Some(100),
            max_dirs: None, // Should inherit global max_dirs=3
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
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
            pattern: "[invalid".to_string(),
            max_files: Some(10),
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

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
            pattern: "src/special/**".to_string(),
            max_files: None, // Inherit 50
            max_dirs: None,
            max_depth: None,
            warn_threshold: Some(0.5), // Rule: warn at 25
            allow_extensions: vec![],
            allow_patterns: vec![],
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
            pattern: "src/generated/**".to_string(),
            max_files: Some(UNLIMITED), // Override to unlimited
            max_dirs: None,             // Inherit global (2)
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
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
            pattern: "src/**".to_string(),
            max_files: Some(-10), // Invalid
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
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
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: Some(-3), // Invalid
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
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

// ============================================================================
// Override Tests
// ============================================================================

#[test]
fn checker_enabled_with_overrides() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy module".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn override_takes_priority_over_global() {
    let config = StructureConfig {
        max_files: Some(10), // Global limit
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: Some(100), // Override limit
            max_dirs: None,
            max_depth: None,
            reason: "Legacy module".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/legacy"),
        DirStats {
            file_count: 50, // Above global limit (10), below override limit (100)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty()); // No violation because override allows 100
}

#[test]
fn override_takes_priority_over_rules() {
    let config = StructureConfig {
        max_files: Some(10),
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: Some(20),
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
        }],
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy module".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/legacy"),
        DirStats {
            file_count: 50, // Above rule limit (20), below override limit (100)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty()); // No violation because override has highest priority
}

#[test]
fn override_reason_included_in_violation() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: Some(10),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy module, scheduled for refactor".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/legacy"),
        DirStats {
            file_count: 15, // Above override limit
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert_eq!(
        violations[0].override_reason,
        Some("Legacy module, scheduled for refactor".to_string())
    );
}

#[test]
fn override_path_suffix_matching() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "legacy".to_string(), // Just the directory name
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/modules/legacy"), // Full path ending with "legacy"
        DirStats {
            file_count: 50,
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty()); // Override should match via suffix
}

#[test]
fn override_full_path_matching() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("other/src/legacy"), // Path ends with src/legacy
        DirStats {
            file_count: 50,
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty()); // Override should match via suffix
}

#[test]
fn override_does_not_match_partial_directory_name() {
    let config = StructureConfig {
        max_files: Some(10),
        overrides: vec![StructureOverride {
            path: "legacy".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/my_legacy"), // Contains "legacy" but doesn't match as component
        DirStats {
            file_count: 50,
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1); // Should NOT match, uses global limit
}

#[test]
fn override_with_unlimited_value() {
    let config = StructureConfig {
        max_files: Some(10),
        overrides: vec![StructureOverride {
            path: "src/generated".to_string(),
            max_files: Some(UNLIMITED), // No limit for generated files
            max_dirs: None,
            max_depth: None,
            reason: "Generated code".to_string(),
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/generated"),
        DirStats {
            file_count: 1000, // Many files, but unlimited allowed
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty()); // No violation because of unlimited
}

#[test]
fn invalid_override_max_files_returns_error() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: Some(-10), // Invalid
            max_dirs: None,
            max_depth: None,
            reason: "Legacy".to_string(),
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_files value in override 1"));
    }
}

#[test]
fn invalid_override_max_dirs_returns_error() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: None,
            max_dirs: Some(-5), // Invalid
            max_depth: None,
            reason: "Legacy".to_string(),
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_dirs value in override 1"));
    }
}

#[test]
fn override_requires_at_least_one_limit() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: None, // Neither set
            max_dirs: None,
            max_depth: None,
            reason: "Legacy".to_string(),
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("must specify at least one of max_files, max_dirs, or max_depth"));
    }
}

// ============================================================================
// Max Depth Tests
// ============================================================================

fn config_with_depth_limit(max_depth: i64) -> StructureConfig {
    StructureConfig {
        max_depth: Some(max_depth),
        ..Default::default()
    }
}

#[test]
fn checker_enabled_with_depth_limit() {
    let checker = StructureChecker::new(&config_with_depth_limit(2)).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn check_depth_under_limit_returns_no_violations() {
    let checker = StructureChecker::new(&config_with_depth_limit(3)).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("root"),
        DirStats {
            file_count: 0,
            dir_count: 1,
            depth: 0,
        },
    );
    stats.insert(
        PathBuf::from("root/sub1"),
        DirStats {
            file_count: 0,
            dir_count: 1,
            depth: 1,
        },
    );
    stats.insert(
        PathBuf::from("root/sub1/sub2"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 2,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty());
}

#[test]
fn check_depth_over_limit_returns_violation() {
    let checker = StructureChecker::new(&config_with_depth_limit(2)).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("root"),
        DirStats {
            file_count: 0,
            dir_count: 1,
            depth: 0,
        },
    );
    stats.insert(
        PathBuf::from("root/sub1"),
        DirStats {
            file_count: 0,
            dir_count: 1,
            depth: 1,
        },
    );
    stats.insert(
        PathBuf::from("root/sub1/sub2"),
        DirStats {
            file_count: 0,
            dir_count: 1,
            depth: 2,
        },
    );
    stats.insert(
        PathBuf::from("root/sub1/sub2/sub3"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 3, // Exceeds limit of 2
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::MaxDepth);
    assert_eq!(violations[0].actual, 3);
    assert_eq!(violations[0].limit, 2);
}

#[test]
fn unlimited_depth_skips_check() {
    let config = StructureConfig {
        max_depth: Some(UNLIMITED),
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("root/a/b/c/d/e/f"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 100, // Very deep, but unlimited
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty());
}

#[test]
fn rule_overrides_global_depth_limit() {
    let config = StructureConfig {
        max_depth: Some(2),
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(5), // Override to allow deeper
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/a/b/c"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 4, // Exceeds global (2), but within rule (5)
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty());
}

#[test]
fn depth_warn_threshold() {
    let config = StructureConfig {
        max_depth: Some(5),
        warn_threshold: Some(0.6), // Warn at depth 3
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("root/a/b/c"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 4, // Above 3 (warn), below 5 (limit)
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].is_warning);
    assert_eq!(violations[0].violation_type, ViolationType::MaxDepth);
}

#[test]
fn invalid_max_depth_value_returns_error() {
    let config = StructureConfig {
        max_depth: Some(-5), // Invalid
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_depth value"));
    }
}

#[test]
fn invalid_rule_max_depth_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(-3), // Invalid
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("Invalid max_depth value in rule 1"));
    }
}

#[test]
fn override_with_max_depth_only() {
    let config = StructureConfig {
        max_depth: Some(2),
        overrides: vec![StructureOverride {
            path: "deep".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(10), // Only max_depth set
            reason: "Deep nesting allowed".to_string(),
        }],
        ..Default::default()
    };

    // Should succeed because at least one limit (max_depth) is set
    let result = StructureChecker::new(&config);
    assert!(result.is_ok());
}

#[test]
fn checker_enabled_with_whitelist_rule() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![".rs".to_string()],
            allow_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn with_scanner_exclude_creates_checker() {
    let config = config_with_file_limit(10);
    let exclude_patterns = vec!["node_modules".to_string()];
    let checker = StructureChecker::with_scanner_exclude(&config, &exclude_patterns).unwrap();
    assert!(checker.is_enabled());
}

#[test]
fn invalid_override_max_depth_returns_error() {
    let config = StructureConfig {
        overrides: vec![StructureOverride {
            path: "src/special".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(-5), // Invalid: less than -1
            reason: "test".to_string(),
        }],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

#[test]
fn override_empty_path_does_not_match() {
    let config = StructureConfig {
        max_files: Some(10),
        overrides: vec![StructureOverride {
            path: String::new(), // Empty path
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "empty path".to_string(),
        }],
        ..Default::default()
    };
    // Creating a checker with empty override path should work (but not match anything)
    let checker = StructureChecker::new(&config).unwrap();
    // Get limits for a real path - should use global limits, not override
    let limits = checker.get_limits(&PathBuf::from("src/lib"));
    assert_eq!(limits.max_files, Some(10));
}
