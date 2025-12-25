//! Sibling (co-location) requirement tests.

use std::path::PathBuf;

use crate::config::{SiblingRequire, SiblingRule, SiblingSeverity};

use super::*;

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn empty_match_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: String::new(), // Empty!
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("empty 'match' pattern"));
    }
}

#[test]
fn empty_require_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single(String::new()), // Empty!
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

#[test]
fn group_with_one_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Group {
                group: vec!["{stem}.ts".to_string()], // Only one pattern!
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("at least 2 patterns"));
    }
}

#[test]
fn empty_siblings_array_is_valid_no_op() {
    // No sibling rules - valid, just no sibling checking
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_ok());
}

#[test]
fn invalid_match_glob_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "[invalid".to_string(), // Invalid glob
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

#[test]
fn require_pattern_without_stem_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("test.spec".to_string()), // Missing {stem}!
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("{stem}"));
        assert!(msg.contains("require"));
    }
}

#[test]
fn require_multiple_patterns_one_without_stem_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.tsx".to_string(),
                require: SiblingRequire::Multiple(vec![
                    "{stem}.test.tsx".to_string(), // Valid
                    "styles.css".to_string(),      // Missing {stem}!
                ]),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(msg.contains("styles.css"));
        assert!(msg.contains("{stem}"));
    }
}

// ============================================================================
// Basic Directed Sibling Check Tests
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
fn directed_file_has_sibling_no_violation() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
fn directed_file_missing_sibling_returns_violation() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/lib/foo.ts"), // No foo.spec exists
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
    assert!(!violations[0].is_warning);
}

#[test]
fn directed_with_warn_severity_creates_warning() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Warn, // Warning instead of error
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![PathBuf::from("src/lib/foo.ts")];

    let violations = checker.check_siblings(&files);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].is_warning); // Should be warning
}

#[test]
fn directed_multiple_requires() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.tsx".to_string(),
                require: SiblingRequire::Multiple(vec![
                    "{stem}.test.tsx".to_string(),
                    "{stem}.module.css".to_string(),
                ]),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    // Button.tsx exists but missing both siblings
    let files = vec![PathBuf::from("src/components/Button.tsx")];

    let violations = checker.check_siblings(&files);
    // One violation per missing sibling
    assert_eq!(violations.len(), 2);
    assert!(violations.iter().any(|v| matches!(
        &v.violation_type,
        ViolationType::MissingSibling { expected_sibling_pattern }
        if expected_sibling_pattern == "{stem}.test.tsx"
    )));
    assert!(violations.iter().any(|v| matches!(
        &v.violation_type,
        ViolationType::MissingSibling { expected_sibling_pattern }
        if expected_sibling_pattern == "{stem}.module.css"
    )));
}

// ============================================================================
// Pattern Matching Tests
// ============================================================================

#[test]
fn directed_dir_pattern_not_matching_skips_check() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/components/**".to_string(), // Only components
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
fn directed_file_pattern_not_matching_skips_check() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.tsx".to_string(), // Only .tsx files
                require: SiblingRequire::Single("{stem}.test.tsx".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
// Group (Atomic) Sibling Tests
// ============================================================================

#[test]
fn group_all_files_exist_no_violation() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Group {
                group: vec!["{stem}.tsx".to_string(), "{stem}.test.tsx".to_string()],
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/components/Button.tsx"),
        PathBuf::from("src/components/Button.test.tsx"),
    ];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty());
}

#[test]
fn group_missing_member_returns_violation() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Group {
                group: vec!["{stem}.tsx".to_string(), "{stem}.test.tsx".to_string()],
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    // Button.tsx exists but Button.test.tsx is missing
    let files = vec![PathBuf::from("src/components/Button.tsx")];

    let violations = checker.check_siblings(&files);
    assert_eq!(violations.len(), 1);
    assert_eq!(
        violations[0].path,
        PathBuf::from("src/components/Button.tsx")
    );
    assert!(matches!(
        &violations[0].violation_type,
        ViolationType::GroupIncomplete { group_patterns, missing_patterns }
        if group_patterns.len() == 2 && missing_patterns.contains(&"{stem}.test.tsx".to_string())
    ));
}

#[test]
fn group_with_warn_severity_creates_warning() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Group {
                group: vec!["{stem}.tsx".to_string(), "{stem}.test.tsx".to_string()],
                severity: SiblingSeverity::Warn,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![PathBuf::from("src/components/Button.tsx")];

    let violations = checker.check_siblings(&files);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].is_warning);
}

#[test]
fn group_three_files_one_missing() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Group {
                group: vec![
                    "{stem}.tsx".to_string(),
                    "{stem}.test.tsx".to_string(),
                    "{stem}.module.css".to_string(),
                ],
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    // Button.tsx and Button.test.tsx exist, Button.module.css is missing
    let files = vec![
        PathBuf::from("src/components/Button.tsx"),
        PathBuf::from("src/components/Button.test.tsx"),
    ];

    let violations = checker.check_siblings(&files);
    // Each existing file in the group triggers a violation because group is incomplete
    assert_eq!(violations.len(), 2);
    for v in &violations {
        assert!(matches!(
            &v.violation_type,
            ViolationType::GroupIncomplete { missing_patterns, .. }
            if missing_patterns.contains(&"{stem}.module.css".to_string())
        ));
    }
}

#[test]
fn group_file_not_in_group_no_check() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Group {
                group: vec!["{stem}.tsx".to_string(), "{stem}.test.tsx".to_string()],
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    // A .ts file doesn't match any pattern in the group
    let files = vec![PathBuf::from("src/components/utils.ts")];

    let violations = checker.check_siblings(&files);
    assert!(violations.is_empty());
}

// ============================================================================
// Multiple Files Tests
// ============================================================================

#[test]
fn directed_multiple_files_mixed_results() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
fn directed_nested_directories() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
fn directed_violations_are_sorted_by_path() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
            scope: "**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.tsx".to_string(),
                require: SiblingRequire::Single("{stem}.spec".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
            scope: "**".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*Service.java".to_string(),
                require: SiblingRequire::Single("{stem}Test.java".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
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
