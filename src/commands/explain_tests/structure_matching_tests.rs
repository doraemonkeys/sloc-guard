//! Tests for structure rule matching in explain command.
//!
//! Covers: rule pattern matching, last-rule-wins semantics, rule chain statuses,
//! and handling of no-limits configuration.

use std::path::PathBuf;

use crate::checker::{MatchStatus, StructureRuleMatch};
use crate::config::{StructureConfig, StructureRule};

#[test]
fn explain_structure_rule_matches() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![StructureRule {
            scope: "src/components/*".to_string(),
            max_files: Some(50),
            max_dirs: Some(10),
            ..Default::default()
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/components/Button"));

    assert!(matches!(
        explanation.matched_rule,
        StructureRuleMatch::Rule { index: 0, pattern, .. } if pattern == "src/components/*"
    ));
    assert_eq!(explanation.effective_max_files, Some(50));
    assert_eq!(explanation.effective_max_dirs, Some(10));
}

#[test]
fn explain_structure_rule_with_reason() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![StructureRule {
            scope: "src/legacy/*".to_string(),
            max_files: Some(100),
            reason: Some("Legacy directory".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/legacy/module"));

    assert!(matches!(
        &explanation.matched_rule,
        StructureRuleMatch::Rule { index: 0, reason, .. } if *reason == Some("Legacy directory".to_string())
    ));
    assert_eq!(explanation.effective_max_files, Some(100));
    assert_eq!(
        explanation.override_reason,
        Some("Legacy directory".to_string())
    );
}

#[test]
fn explain_structure_default_matches() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));

    assert!(matches!(
        explanation.matched_rule,
        StructureRuleMatch::Default
    ));
    assert_eq!(explanation.effective_max_files, Some(10));
    assert_eq!(explanation.effective_max_dirs, Some(5));
}

#[test]
fn explain_structure_rule_chain_statuses() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![
            StructureRule {
                scope: "src/*".to_string(),
                max_files: Some(20),
                max_dirs: Some(10),
                ..Default::default()
            },
            StructureRule {
                scope: "test/*".to_string(),
                max_files: Some(30),
                max_dirs: Some(15),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();

    // Check a path that matches the first rule
    let explanation = checker.explain(&PathBuf::from("src/components"));

    let chain = &explanation.rule_chain;

    // First rule should match
    let first_rule = chain.iter().find(|c| c.source == "structure.rules[0]");
    assert_eq!(first_rule.unwrap().status, MatchStatus::Matched);

    // Second rule should not match
    let second_rule = chain.iter().find(|c| c.source == "structure.rules[1]");
    assert_eq!(second_rule.unwrap().status, MatchStatus::NoMatch);

    // Default should be superseded
    let default_entry = chain.iter().find(|c| c.source == "structure (default)");
    assert_eq!(default_entry.unwrap().status, MatchStatus::Superseded);
}
