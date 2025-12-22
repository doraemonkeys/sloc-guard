//! Max depth tests including relative depth functionality.

use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

// ============================================================================
// Basic Max Depth Tests
// ============================================================================

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
            scope: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(5), // Override to allow deeper
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
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
            scope: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(-3), // Invalid
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
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

// ============================================================================
// Relative Depth Tests
// ============================================================================

#[test]
fn calculate_base_depth_simple_pattern() {
    // "src/features/**" → base_depth = 2
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/features/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(2),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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

    // Verify base_depth calculation via behavior
    let mut stats = HashMap::new();
    // src/features/module at absolute depth 3, relative depth 1 (within limit)
    stats.insert(
        PathBuf::from("src/features/module"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 3,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty()); // depth 3 - base 2 = 1, within limit of 2
}

#[test]
fn relative_depth_allows_deep_nesting_within_base() {
    // Rule: src/features/** with relative_depth=true, max_depth=2
    // Absolute depth 4 = relative depth 2 (within limit)
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/features/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(2),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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
    // src/features/module/sub at absolute depth 4, relative depth 2
    stats.insert(
        PathBuf::from("src/features/module/sub"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 4,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty()); // 4 - 2 = 2, exactly at limit
}

#[test]
fn relative_depth_violates_when_too_deep() {
    // Rule: src/features/** with relative_depth=true, max_depth=2
    // Absolute depth 5 = relative depth 3 (exceeds limit)
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/features/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(2),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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
        PathBuf::from("src/features/a/b/c"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 5, // absolute depth
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::MaxDepth);
    assert_eq!(violations[0].actual, 3); // relative depth (5 - 2)
    assert_eq!(violations[0].limit, 2);
}

#[test]
fn relative_depth_false_uses_absolute_depth() {
    // Without relative_depth, the absolute depth is used
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/features/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(2),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false, // Default behavior
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
    // src/features/module at absolute depth 3 (exceeds limit of 2)
    stats.insert(
        PathBuf::from("src/features/module"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 3,
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].actual, 3); // absolute depth
    assert_eq!(violations[0].limit, 2);
}

#[test]
fn relative_depth_with_wildcard_in_middle() {
    // Pattern: src/*/utils with wildcard in middle
    // base_depth should be 1 (only "src" is concrete)
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/*/utils/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(1),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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
    // src/feature/utils/helpers at depth 4, base_depth=1, relative=3 (exceeds 1)
    stats.insert(
        PathBuf::from("src/feature/utils/helpers"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 4,
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].actual, 3); // 4 - 1 = 3
    assert_eq!(violations[0].limit, 1);
}

#[test]
fn relative_depth_with_double_star_at_start() {
    // Pattern: **/*.rs → base_depth = 0
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(3),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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
        PathBuf::from("a/b/c/d"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 4,
        },
    );

    let violations = checker.check(&stats);
    // base_depth = 0, so relative = absolute = 4, exceeds limit 3
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].actual, 4);
}

#[test]
fn relative_depth_warn_threshold() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/features/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(5),
            warn_threshold: Some(0.6), // Warn at depth 3
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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
    // Relative depth 4 (abs 6) - above 3 (warn), below 5 (limit)
    stats.insert(
        PathBuf::from("src/features/a/b/c/d"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 6, // abs depth
        },
    );

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].is_warning);
    assert_eq!(violations[0].actual, 4); // relative depth (6 - 2)
    assert_eq!(violations[0].limit, 5);
}

#[test]
fn relative_depth_moving_base_works() {
    // This test demonstrates the main use case:
    // When project moves from src/features to packages/core/src/features,
    // relative depth still works correctly
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "packages/core/src/features/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: Some(2),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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
    // packages/core/src/features/module/sub at depth 6
    // base_depth = 4 (packages, core, src, features)
    // relative = 6 - 4 = 2 (within limit)
    stats.insert(
        PathBuf::from("packages/core/src/features/module/sub"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 6,
        },
    );

    let violations = checker.check(&stats);
    assert!(violations.is_empty());
}

#[test]
fn relative_depth_saturating_sub_for_shallow_paths() {
    // Edge case: what if a matched path is shallower than base_depth?
    // This shouldn't happen normally, but we use saturating_sub to be safe.
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/features/**".to_string(), // base_depth = 2
            max_files: None,
            max_dirs: None,
            max_depth: Some(1),
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: true,
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
    // Path matching the pattern but with depth=1 (should result in relative 0)
    stats.insert(
        PathBuf::from("src/features"),
        DirStats {
            file_count: 0,
            dir_count: 0,
            depth: 2,
        },
    );

    let violations = checker.check(&stats);
    // relative = 2 - 2 = 0, within limit of 1
    assert!(violations.is_empty());
}
