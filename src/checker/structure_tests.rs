use std::collections::HashMap;
use std::path::PathBuf;

use tempfile::TempDir;

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
fn collect_dir_stats_counts_files_and_dirs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create files
    std::fs::write(root.join("file1.txt"), "content").unwrap();
    std::fs::write(root.join("file2.txt"), "content").unwrap();

    // Create subdirectories
    std::fs::create_dir(root.join("subdir1")).unwrap();
    std::fs::create_dir(root.join("subdir2")).unwrap();

    let checker = StructureChecker::new(&default_config()).unwrap();
    let (stats, _) = checker.collect_dir_stats(root).unwrap();

    let root_stats = stats.get(root).unwrap();
    assert_eq!(root_stats.file_count, 2);
    assert_eq!(root_stats.dir_count, 2);
}

#[test]
fn collect_dir_stats_ignores_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create files
    std::fs::write(root.join("file1.txt"), "content").unwrap();
    std::fs::write(root.join("file2.md"), "content").unwrap();
    std::fs::write(root.join(".gitkeep"), "").unwrap();

    let config = StructureConfig {
        count_exclude: vec!["*.md".to_string(), ".gitkeep".to_string()],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let (stats, _) = checker.collect_dir_stats(root).unwrap();

    let root_stats = stats.get(root).unwrap();
    assert_eq!(root_stats.file_count, 1); // Only file1.txt counted
}

#[test]
fn collect_dir_stats_recursive() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create nested structure
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(root.join("src/lib.rs"), "").unwrap();
    std::fs::create_dir(root.join("src/utils")).unwrap();
    std::fs::write(root.join("src/utils/helper.rs"), "").unwrap();

    let checker = StructureChecker::new(&default_config()).unwrap();
    let (stats, _) = checker.collect_dir_stats(root).unwrap();

    // Check root has src as subdir
    let root_stats = stats.get(root).unwrap();
    assert_eq!(root_stats.dir_count, 1);
    assert_eq!(root_stats.file_count, 0);

    // Check src has 2 files and 1 subdir
    let src_stats = stats.get(&root.join("src")).unwrap();
    assert_eq!(src_stats.file_count, 2);
    assert_eq!(src_stats.dir_count, 1);

    // Check utils has 1 file
    let utils_stats = stats.get(&root.join("src/utils")).unwrap();
    assert_eq!(utils_stats.file_count, 1);
    assert_eq!(utils_stats.dir_count, 0);
}

#[test]
fn check_directory_integration() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create many files to exceed limit
    for i in 0..15 {
        std::fs::write(root.join(format!("file{i}.txt")), "content").unwrap();
    }

    let checker = StructureChecker::new(&config_with_file_limit(10)).unwrap();
    let violations = checker.check_directory(root).unwrap();

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::FileCount);
    assert_eq!(violations[0].actual, 15);
    assert_eq!(violations[0].limit, 10);
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
fn invalid_ignore_pattern_returns_error() {
    let config = StructureConfig {
        count_exclude: vec!["[invalid".to_string()],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
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
// Scanner Exclude Tests (scanner.exclude should skip directories entirely)
// ============================================================================

#[test]
fn scanner_exclude_skips_directories_entirely() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create regular directory with files
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

    // Create .git directory (should be excluded by scanner.exclude)
    std::fs::create_dir(root.join(".git")).unwrap();
    std::fs::create_dir(root.join(".git/hooks")).unwrap();
    std::fs::write(root.join(".git/config"), "[core]").unwrap();
    std::fs::write(root.join(".git/hooks/pre-commit"), "#!/bin/sh").unwrap();

    // Create target directory (should also be excluded)
    std::fs::create_dir(root.join("target")).unwrap();
    std::fs::write(root.join("target/debug.txt"), "debug").unwrap();

    let config = StructureConfig {
        max_files: Some(10),
        ..Default::default()
    };

    // Pass scanner exclude patterns
    let scanner_exclude = vec![".git/**".to_string(), "target/**".to_string()];
    let checker = StructureChecker::with_scanner_exclude(&config, &scanner_exclude).unwrap();
    let (stats, _) = checker.collect_dir_stats(root).unwrap();

    // Root should have 1 dir (src only, .git and target excluded)
    let root_stats = stats.get(root).unwrap();
    assert_eq!(root_stats.dir_count, 1, "root should only see 'src' dir");

    // .git should NOT be in stats (completely skipped)
    assert!(
        !stats.contains_key(&root.join(".git")),
        ".git should not be traversed"
    );
    assert!(
        !stats.contains_key(&root.join(".git/hooks")),
        ".git/hooks should not be traversed"
    );

    // target should NOT be in stats
    assert!(
        !stats.contains_key(&root.join("target")),
        "target should not be traversed"
    );

    // src should be in stats
    assert!(
        stats.contains_key(&root.join("src")),
        "src should be traversed"
    );
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
fn collect_dir_stats_tracks_depth() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create nested structure
    std::fs::create_dir_all(root.join("a/b/c")).unwrap();
    std::fs::write(root.join("a/b/c/file.txt"), "content").unwrap();

    let config = StructureConfig::default();
    let checker = StructureChecker::new(&config).unwrap();
    let (stats, _) = checker.collect_dir_stats(root).unwrap();

    assert_eq!(stats.get(root).unwrap().depth, 0);
    assert_eq!(stats.get(&root.join("a")).unwrap().depth, 1);
    assert_eq!(stats.get(&root.join("a/b")).unwrap().depth, 2);
    assert_eq!(stats.get(&root.join("a/b/c")).unwrap().depth, 3);
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

// ============================================================================
// Whitelist Mode Tests (allow_extensions / allow_patterns)
// ============================================================================

#[test]
fn invalid_allow_extension_without_dot_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec!["rs".to_string()], // Missing dot
            allow_patterns: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("must start with '.'"));
    }
}

#[test]
fn invalid_allow_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec!["[invalid".to_string()],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

#[test]
fn whitelist_allow_extensions_allows_matching_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(root.join("src/lib.rs"), "").unwrap();

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    assert!(violations.is_empty());
}

#[test]
fn whitelist_allow_extensions_rejects_non_matching_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(root.join("src/config.json"), "{}").unwrap(); // Not allowed

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::DisallowedFile);
    assert!(violations[0].path.ends_with("config.json"));
    assert_eq!(
        violations[0].triggering_rule_pattern,
        Some("**/src".to_string())
    );
}

#[test]
fn whitelist_allow_patterns_allows_matching_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/mod.rs"), "").unwrap();
    std::fs::write(root.join("src/Makefile"), "").unwrap();

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec!["*.rs".to_string(), "Makefile".to_string()],
        }],
        ..Default::default()
    };

    let checker = StructureChecker::new(&config).unwrap();
    let violations = checker.check_directory(root).unwrap();

    assert!(violations.is_empty());
}

#[test]
fn whitelist_combined_extensions_and_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "").unwrap(); // OK via extension
    std::fs::write(root.join("src/Cargo.toml"), "").unwrap(); // OK via pattern
    std::fs::write(root.join("src/random.txt"), "").unwrap(); // VIOLATION

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![".rs".to_string()],
            allow_patterns: vec!["Cargo.toml".to_string()],
        }],
        ..Default::default()
    };

    let checker = StructureChecker::new(&config).unwrap();
    let violations = checker.check_directory(root).unwrap();

    assert_eq!(violations.len(), 1);
    assert!(violations[0].path.ends_with("random.txt"));
}

#[test]
fn count_exclude_bypasses_whitelist_check() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "").unwrap();
    std::fs::write(root.join("src/README.md"), "").unwrap(); // Would violate, but excluded

    let config = StructureConfig {
        count_exclude: vec!["*.md".to_string()],
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    assert!(violations.is_empty()); // README.md bypassed via count_exclude
}

#[test]
fn whitelist_applies_only_to_files_not_directories() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::create_dir(root.join("src/utils")).unwrap(); // Directory, not checked
    std::fs::write(root.join("src/main.rs"), "").unwrap();

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    assert!(violations.is_empty()); // Directories are not subject to whitelist
}

#[test]
fn empty_whitelist_allows_all_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/anything.xyz"), "").unwrap();

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    assert!(violations.is_empty());
}

#[test]
fn whitelist_recursive_pattern_applies_to_subdirectories() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir_all(root.join("src/sub")).unwrap();
    std::fs::write(root.join("src/main.rs"), "").unwrap();
    std::fs::write(root.join("src/sub/lib.rs"), "").unwrap();
    std::fs::write(root.join("src/sub/data.json"), "").unwrap(); // VIOLATION

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src/**".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    assert_eq!(violations.len(), 1);
    assert!(violations[0].path.ends_with("data.json"));
}

#[test]
fn whitelist_with_file_without_extension() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "").unwrap();
    std::fs::write(root.join("src/Dockerfile"), "").unwrap(); // No extension, must match pattern

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![".rs".to_string()],
            allow_patterns: vec!["Dockerfile".to_string()],
        }],
        ..Default::default()
    };

    let checker = StructureChecker::new(&config).unwrap();
    let violations = checker.check_directory(root).unwrap();

    assert!(violations.is_empty()); // Dockerfile allowed via pattern
}

#[test]
fn whitelist_violation_without_pattern_match() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/Dockerfile"), "").unwrap(); // No extension, doesn't match

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    assert_eq!(violations.len(), 1);
    assert!(violations[0].path.ends_with("Dockerfile"));
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
fn whitelist_multiple_extensions() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "").unwrap();
    std::fs::write(root.join("src/test.go"), "").unwrap();
    std::fs::write(root.join("src/config.txt"), "").unwrap(); // VIOLATION

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![".rs".to_string(), ".go".to_string()],
            allow_patterns: vec![],
        }],
        ..Default::default()
    };

    let checker = StructureChecker::new(&config).unwrap();
    let violations = checker.check_directory(root).unwrap();

    assert_eq!(violations.len(), 1);
    assert!(violations[0].path.ends_with("config.txt"));
}

#[test]
fn whitelist_collect_dir_stats_returns_violations_separately() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "").unwrap();
    std::fs::write(root.join("src/bad.txt"), "").unwrap(); // VIOLATION

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let (stats, whitelist_violations) = checker.collect_dir_stats(root).unwrap();

    // Stats should include all directories
    assert!(stats.contains_key(root));
    assert!(stats.contains_key(&root.join("src")));

    // Whitelist violations should be collected
    assert_eq!(whitelist_violations.len(), 1);
    assert!(whitelist_violations[0].path.ends_with("bad.txt"));
}

#[test]
fn whitelist_no_match_returns_no_violations() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create directory that doesn't match the whitelist rule pattern
    std::fs::create_dir(root.join("other")).unwrap();
    std::fs::write(root.join("other/any.txt"), "").unwrap();

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
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
    let violations = checker.check_directory(root).unwrap();

    // No violations because "other" doesn't match "**/src" pattern
    assert!(violations.is_empty());
}

#[test]
fn whitelist_pattern_full_path_match() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::create_dir(root.join("src/config")).unwrap();
    std::fs::write(root.join("src/main.rs"), "").unwrap();
    std::fs::write(root.join("src/config/settings.json"), "").unwrap();

    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**/src".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec!["config/*".to_string()],
        }],
        ..Default::default()
    };

    let checker = StructureChecker::new(&config).unwrap();
    let violations = checker.check_directory(root).unwrap();

    // main.rs should be a violation (doesn't match any extension or pattern)
    assert_eq!(violations.len(), 1);
    assert!(violations[0].path.ends_with("main.rs"));
}
