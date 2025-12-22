//! Override functionality tests.

use std::collections::HashMap;
use std::path::PathBuf;

use super::*;

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
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],

            deny_file_patterns: vec![],
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

// ============================================================================
// Override Validation Error Tests
// ============================================================================

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
