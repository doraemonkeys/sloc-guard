//! Tests for structure validation logic.

use super::*;

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
