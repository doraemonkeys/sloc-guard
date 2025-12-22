//! Sibling (co-location) requirement tests.

use std::path::PathBuf;

use super::*;

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn require_sibling_without_file_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: None,                               // Missing!
            require_sibling: Some("{stem}.spec".to_string()), // Set
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("require_sibling"));
        assert!(msg.contains("file_pattern"));
    }
}

#[test]
fn file_pattern_without_require_sibling_is_allowed() {
    // file_pattern alone doesn't trigger sibling checking (no-op)
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()),
            require_sibling: None,
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_ok());
}

#[test]
fn invalid_file_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("[invalid".to_string()), // Invalid glob
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

// ============================================================================
// Basic Sibling Check Tests
// ============================================================================

#[test]
fn check_siblings_no_rules_returns_empty() {
    let config = StructureConfig {
        max_files: Some(100),
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/foo.ts"),
        PathBuf::from("src/lib/bar.ts"),
    ];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty());
}

#[test]
fn check_siblings_file_has_sibling_no_violation() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()),
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/foo.ts"),
        PathBuf::from("src/lib/foo.spec"), // Sibling exists
    ];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty());
}

#[test]
fn check_siblings_file_missing_sibling_returns_violation() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()),
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/foo.ts"), // No foo.test.ts exists
    ];

    let violations = checker.check_siblings(&files);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/lib/foo.ts"));
    assert_eq!(
        violations[0].violation_type,
        ViolationType::MissingSibling {
            expected_sibling_pattern: "{stem}.spec".to_string()
        }
    );
}

// ============================================================================
// Pattern Matching Tests
// ============================================================================

#[test]
fn check_siblings_dir_pattern_not_matching_skips_check() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/components/**".to_string(), // Only components
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()),
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/utils/foo.ts"), // utils, not components
    ];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty()); // No check because dir pattern doesn't match
}

#[test]
fn check_siblings_file_pattern_not_matching_skips_check() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.tsx".to_string()), // Only .tsx files
            require_sibling: Some("{stem}.test.tsx".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/foo.ts"), // .ts, not .tsx
    ];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty()); // No check because file pattern doesn't match
}

// ============================================================================
// Multiple Files Tests
// ============================================================================

#[test]
fn check_siblings_multiple_files_mixed_results() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()),
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/foo.ts"),
        PathBuf::from("src/lib/foo.spec"), // foo.ts has sibling
        PathBuf::from("src/lib/bar.ts"),   // bar.ts missing sibling
        PathBuf::from("src/lib/baz.ts"),
        PathBuf::from("src/lib/baz.spec"), // baz.ts has sibling
    ];

    let violations = checker.check_siblings(&files);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/lib/bar.ts"));
}

#[test]
fn check_siblings_test_file_not_checked_for_siblings() {
    // Test that .test.ts files also get checked (they match *.ts)
    // This is expected behavior - if you want to exclude test files,
    // use a more specific file_pattern
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()), // Matches ALL *.ts including *.test.ts
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/foo.test.ts"), // This matches *.ts
    ];

    let violations = checker.check_siblings(&files);
    // foo.test.ts matches *.ts, so it requires foo.test.spec which doesn't exist
    assert_eq!(violations.len(), 1);
}

#[test]
fn check_siblings_nested_directories() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()),
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/components/Button.ts"),
        PathBuf::from("src/components/Button.spec"),
        PathBuf::from("src/utils/helpers.ts"), // Missing sibling
    ];

    let violations = checker.check_siblings(&files);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/utils/helpers.ts"));
}

#[test]
fn check_siblings_sorted_by_path() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "src/**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.ts".to_string()),
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/zebra.ts"),
        PathBuf::from("src/lib/alpha.ts"),
        PathBuf::from("src/lib/middle.ts"),
    ];

    let violations = checker.check_siblings(&files);
    assert_eq!(violations.len(), 3);
    assert_eq!(violations[0].path, PathBuf::from("src/lib/alpha.ts"));
    assert_eq!(violations[1].path, PathBuf::from("src/lib/middle.ts"));
    assert_eq!(violations[2].path, PathBuf::from("src/lib/zebra.ts"));
}

// ============================================================================
// Sibling Template Tests
// ============================================================================

#[test]
fn derive_sibling_path_basic() {
    // Test the derive_sibling_path function indirectly through check_siblings
    // Use .spec for test files so they don't match *.tsx pattern
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*.tsx".to_string()),
            require_sibling: Some("{stem}.spec".to_string()),
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    // Button.tsx expects Button.spec
    let files = vec![
        PathBuf::from("src/Button.tsx"),
        PathBuf::from("src/Button.spec"), // sibling with different extension
    ];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty());
}

#[test]
fn derive_sibling_path_different_template() {
    // Use *Service.java to only match service files, not test files
    let config = StructureConfig {
        rules: vec![StructureRule {
            pattern: "**".to_string(),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            file_naming_pattern: None,
            relative_depth: false,
            file_pattern: Some("*Service.java".to_string()), // Only matches *Service.java
            require_sibling: Some("{stem}Test.java".to_string()), // Java style
            deny_extensions: vec![],
            deny_patterns: vec![],
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    // UserService.java expects UserServiceTest.java
    let files = vec![
        PathBuf::from("src/UserService.java"),
        PathBuf::from("src/UserServiceTest.java"), // Doesn't match *Service.java
    ];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty());
}
