use std::collections::HashMap;
use std::path::PathBuf;

use tempfile::TempDir;

use super::*;
use crate::config::{StructureConfig, StructureRule, UNLIMITED};

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
            warn_threshold: None,
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
            warn_threshold: None,
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
            warn_threshold: None,
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
    let stats = checker.collect_dir_stats(root).unwrap();

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
    let stats = checker.collect_dir_stats(root).unwrap();

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
    let stats = checker.collect_dir_stats(root).unwrap();

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
        },
    );
    stats.insert(
        PathBuf::from("a_dir"),
        DirStats {
            file_count: 10,
            dir_count: 0,
        },
    );
    stats.insert(
        PathBuf::from("m_dir"),
        DirStats {
            file_count: 10,
            dir_count: 0,
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
    let violation = StructureViolation::new(PathBuf::from("src"), ViolationType::FileCount, 15, 10);

    assert_eq!(violation.path, PathBuf::from("src"));
    assert_eq!(violation.violation_type, ViolationType::FileCount);
    assert_eq!(violation.actual, 15);
    assert_eq!(violation.limit, 10);
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
            warn_threshold: None,
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
            max_files: None,            // Inherit 50
            max_dirs: None,
            warn_threshold: Some(0.5),  // Rule: warn at 25
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
            warn_threshold: None,
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
            warn_threshold: None,
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
            warn_threshold: None,
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
