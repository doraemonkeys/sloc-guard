//! Rule priority tests: last match wins behavior.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::checker::explain::{MatchStatus, StructureRuleMatch};

use super::*;

#[test]
fn multiple_matching_rules_last_match_wins() {
    // When multiple rules match, the LAST declared rule should win
    // (consistent with content rules behavior)
    let config = StructureConfig {
        max_files: Some(5), // Global default
        rules: vec![
            StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(10), // First rule: 10 files
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
            StructureRule {
                scope: "src/generated/**".to_string(),
                max_files: Some(100), // Second rule (more specific): 100 files
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/generated/protos"),
        DirStats {
            file_count: 50, // Above 10 (first rule), below 100 (second rule)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    // Should use second rule (100 files), not first rule (10 files)
    assert!(violations.is_empty());
}

#[test]
fn last_match_wins_more_restrictive_rule_last() {
    // If a more restrictive rule is declared LAST, it should take effect
    let config = StructureConfig {
        max_files: Some(100), // Global: very permissive
        rules: vec![
            StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(50), // First rule: permissive
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
            StructureRule {
                scope: "src/core/**".to_string(),
                max_files: Some(10), // Second rule: restrictive
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/core/engine"),
        DirStats {
            file_count: 15, // Above 10 (second rule), below 50 (first rule)
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    // Should use second rule (10 files) because it's declared LAST
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].limit, 10);
}

#[test]
fn three_rules_last_matching_wins() {
    // With three rules where all match, the last one should win
    let config = StructureConfig {
        rules: vec![
            StructureRule {
                scope: "**".to_string(),
                max_files: Some(5),
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
            StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(10),
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
            StructureRule {
                scope: "src/lib/**".to_string(),
                max_files: Some(20),
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/lib/utils"),
        DirStats {
            file_count: 15, // Above 5, above 10, below 20
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    // Last matching rule (max_files=20) should win
    assert!(violations.is_empty());
}

#[test]
fn non_matching_rules_skipped_in_priority() {
    // Rules that don't match should not affect the result
    let config = StructureConfig {
        rules: vec![
            StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(10), // Matches
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
            StructureRule {
                scope: "tests/**".to_string(), // Does NOT match src/lib
                max_files: Some(100),
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/lib"),
        DirStats {
            file_count: 15, // Above 10
            dir_count: 0,
            depth: 0,
        },
    );

    let violations = checker.check(&stats);
    // Only first rule matches (max_files=10), second rule doesn't match
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].limit, 10);
}

#[test]
fn explain_reports_last_matching_rule() {
    let config = StructureConfig {
        rules: vec![
            StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(10),
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
            StructureRule {
                scope: "src/lib/**".to_string(),
                max_files: Some(50),
                max_dirs: None,
                max_depth: None,
                warn_threshold: None,
                warn_files_at: None,
                warn_dirs_at: None,
                warn_files_threshold: None,
                warn_dirs_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                allow_files: vec![],
                allow_dirs: vec![],
                file_naming_pattern: None,
                relative_depth: false,
                file_pattern: None,
                require_sibling: None,
                deny_extensions: vec![],
                deny_patterns: vec![],
                deny_files: vec![],
                deny_dirs: vec![],
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let explanation = checker.explain(&PathBuf::from("src/lib/utils"));

    // The matched rule should be the LAST one (index 1)
    match explanation.matched_rule {
        StructureRuleMatch::Rule { index, pattern } => {
            assert_eq!(index, 1);
            assert_eq!(pattern, "src/lib/**");
        }
        _ => panic!("Expected Rule match, got {:?}", explanation.matched_rule),
    }

    // Check rule chain statuses
    // rules[0] should be Superseded (matches but not the last)
    // rules[1] should be Matched (the last matching rule)
    let rules_in_chain: Vec<_> = explanation
        .rule_chain
        .iter()
        .filter(|c| c.source.starts_with("structure.rules"))
        .collect();
    assert_eq!(rules_in_chain.len(), 2);
    assert_eq!(rules_in_chain[0].status, MatchStatus::Superseded);
    assert_eq!(rules_in_chain[1].status, MatchStatus::Matched);

    // Effective limit should be from the last rule (50)
    assert_eq!(explanation.effective_max_files, Some(50));
}
