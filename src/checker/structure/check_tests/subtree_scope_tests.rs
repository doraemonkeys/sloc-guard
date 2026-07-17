//! Subtree scope (`scope = "dir/"`) tests: one rule covers a directory and its descendants.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::checker::explain::{DirInventorySource, StructureRuleMatch};
use crate::config::{SiblingRequire, SiblingRule, SiblingSeverity};

use super::*;

#[test]
fn subtree_scope_limits_apply_to_base_and_descendants() {
    let config = StructureConfig {
        rules: vec![make_rule("src/", Some(2))],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(PathBuf::from("src"), dir_stats(3, 0, 1));
    stats.insert(PathBuf::from("src/nested"), dir_stats(3, 0, 2));
    stats.insert(PathBuf::from("other"), dir_stats(3, 0, 1));

    let violations = checker.check(&stats);
    let violating_paths: Vec<_> = violations.iter().map(|v| v.path.clone()).collect();
    assert_eq!(
        violating_paths,
        vec![PathBuf::from("src"), PathBuf::from("src/nested")]
    );
}

#[test]
fn subtree_scope_does_not_match_sibling_name_prefix() {
    let config = StructureConfig {
        rules: vec![make_rule("src/", Some(2))],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(PathBuf::from("src2"), dir_stats(10, 0, 1));
    stats.insert(PathBuf::from("srcx/deep"), dir_stats(10, 0, 2));

    assert!(checker.check(&stats).is_empty());
}

#[test]
fn subtree_scope_with_brace_alternation_covers_both_roots() {
    let config = StructureConfig {
        rules: vec![make_rule("web/{src,test}/", Some(2))],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(PathBuf::from("web/src"), dir_stats(3, 0, 2));
    stats.insert(PathBuf::from("web/test"), dir_stats(3, 0, 2));
    stats.insert(PathBuf::from("web/test/fixtures"), dir_stats(3, 0, 3));
    stats.insert(PathBuf::from("web/docs"), dir_stats(3, 0, 2));

    let violations = checker.check(&stats);
    let violating_paths: Vec<_> = violations.iter().map(|v| v.path.clone()).collect();
    assert_eq!(
        violating_paths,
        vec![
            PathBuf::from("web/src"),
            PathBuf::from("web/test"),
            PathBuf::from("web/test/fixtures"),
        ]
    );
}

#[test]
fn subtree_scope_participates_in_last_match_wins() {
    let config = StructureConfig {
        rules: vec![
            make_rule("src/", Some(2)),
            make_rule("src/generated/", Some(100)),
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(PathBuf::from("src/generated"), dir_stats(50, 0, 2));
    stats.insert(PathBuf::from("src/generated/protos"), dir_stats(50, 0, 3));
    stats.insert(PathBuf::from("src/lib"), dir_stats(50, 0, 2));

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/lib"));
}

#[test]
fn subtree_scope_relative_depth_anchors_at_base_directory() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/features/".to_string(),
            max_depth: Some(1),
            relative_depth: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    // Depth 4 from scan root, but only 2 below src/features: exceeds the limit of 1.
    stats.insert(PathBuf::from("src/features/auth/api"), dir_stats(0, 0, 4));
    // The base directory itself sits at relative depth 0.
    stats.insert(PathBuf::from("src/features"), dir_stats(0, 0, 2));

    let violations = checker.check(&stats);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/features/auth/api"));
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn explain_reports_subtree_scope_for_base_directory() {
    let config = StructureConfig {
        rules: vec![make_rule("web/{src,test}/", Some(30))],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    for dir in ["web/src", "web/src/components"] {
        let explanation = checker.explain(&PathBuf::from(dir), DirInventorySource::NotScanned);
        match &explanation.matched_rule {
            StructureRuleMatch::Rule { pattern, .. } => assert_eq!(pattern, "web/{src,test}/"),
            StructureRuleMatch::Default => panic!("expected rule match for {dir}"),
        }
        assert_eq!(explanation.effective_max_files, Some(30));
    }
}

#[test]
fn subtree_scope_sibling_rule_applies_in_base_and_nested_dirs() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/".to_string(),
            siblings: vec![SiblingRule::Directed {
                match_pattern: "*.ts".to_string(),
                require: SiblingRequire::Single("{stem}.test.ts".to_string()),
                severity: SiblingSeverity::Error,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let files = vec![
        PathBuf::from("src/api.ts"),
        PathBuf::from("src/deep/util.ts"),
        PathBuf::from("outside/free.ts"),
    ];

    let violations = checker.check_siblings(&files);
    let violating_paths: Vec<_> = violations.iter().map(|v| v.path.clone()).collect();
    assert_eq!(
        violating_paths,
        vec![
            PathBuf::from("src/api.ts"),
            PathBuf::from("src/deep/util.ts")
        ]
    );
}
