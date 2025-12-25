//! Tests for structure validation logic.

use super::*;
use crate::config::{SiblingRule, SiblingSeverity};

// =============================================================================
// Allow/Deny Mutual Exclusion Tests
// =============================================================================

#[test]
fn global_allow_deny_mutual_exclusion_fails() {
    let config = StructureConfig {
        allow_files: vec!["*.rs".to_string()],
        deny_files: vec!["*.bak".to_string()],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

#[test]
fn global_allow_only_passes() {
    let config = StructureConfig {
        allow_files: vec!["*.rs".to_string()],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_ok());
}

#[test]
fn global_deny_only_passes() {
    let config = StructureConfig {
        deny_files: vec!["*.bak".to_string()],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_ok());
}

#[test]
fn global_allow_dirs_with_deny_extensions_fails() {
    let config = StructureConfig {
        allow_dirs: vec!["src".to_string()],
        deny_extensions: vec![".bak".to_string()],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

#[test]
fn rule_allow_deny_mutual_exclusion_fails() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            allow_files: vec!["*.rs".to_string()],
            deny_files: vec!["*.bak".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

#[test]
fn rule_allow_only_passes() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            allow_extensions: vec![".rs".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_ok());
}

#[test]
fn rule_allow_dirs_with_deny_patterns_fails() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            allow_dirs: vec!["utils".to_string()],
            deny_patterns: vec!["*.bak".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_err());
}

// =============================================================================
// Sibling Group Pattern Validation Tests
// =============================================================================

#[test]
fn group_pattern_without_stem_placeholder_fails() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            siblings: vec![SiblingRule::Group {
                // "index.ts" is missing {stem} placeholder
                group: vec![
                    "{stem}.tsx".to_string(),
                    "index.ts".to_string(), // Invalid: no {stem}
                ],
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let result = StructureChecker::new(&config);
    assert!(result.is_err());
    let err = result.err().expect("expected error");
    assert!(
        err.to_string().contains("{stem}"),
        "Error should mention {{stem}} placeholder: {err}"
    );
}

#[test]
fn group_pattern_with_stem_placeholder_passes() {
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
    let result = StructureChecker::new(&config);
    assert!(result.is_ok());
}
