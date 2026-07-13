//! Check-time `count_exclude` tests.
//!
//! Effective counts are derived from the raw child inventory at check time.
//! These tests lock parity with the former scan-time counting: path-qualified
//! globs, basename fallback for unqualified patterns, and exclusion of both
//! files and directories.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::checker::CountExcludeSource;

use super::*;

fn stats_with_children(files: &[&str], dirs: &[&str], depth: usize) -> DirStats {
    DirStats {
        files: files.iter().map(OsString::from).collect(),
        dirs: dirs.iter().map(OsString::from).collect(),
        depth,
    }
}

fn config_with_count_exclude(max_files: i64, count_exclude: &[&str]) -> StructureConfig {
    StructureConfig {
        max_files: Some(max_files),
        count_exclude: count_exclude.iter().map(ToString::to_string).collect(),
        ..Default::default()
    }
}

#[test]
fn basename_pattern_excludes_files_from_effective_count() {
    let checker =
        StructureChecker::new(&config_with_count_exclude(2, &["*.md", ".gitkeep"])).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        stats_with_children(&["a.rs", "b.rs", "README.md", ".gitkeep"], &[], 1),
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn without_count_exclude_raw_counts_are_used() {
    let checker = StructureChecker::new(&config_with_count_exclude(2, &[])).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        stats_with_children(&["a.rs", "b.rs", "README.md", ".gitkeep"], &[], 1),
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::FileCount);
    assert_eq!(violations[0].actual, 4);
    assert_eq!(violations[0].limit, 2);
}

#[test]
fn path_qualified_pattern_matches_only_its_directory() {
    let checker = StructureChecker::new(&config_with_count_exclude(1, &["src/*.gen"])).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        stats_with_children(&["main.rs", "a.gen"], &[], 1),
    );
    stats.insert(
        PathBuf::from("other"),
        stats_with_children(&["main.rs", "b.gen"], &[], 1),
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("other"));
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn count_exclude_preserves_root_anchors_and_basename_patterns() {
    // Same semantics the scan-time matcher had: unqualified patterns fall back
    // to basename matching, explicit paths never do.
    let checker = StructureChecker::new(&config_with_count_exclude(
        0,
        &["./root.rs", "*.tmp", "./src/**"],
    ))
    .unwrap();
    let mut stats = HashMap::new();
    // Configuration root is keyed as "." after logical rebasing.
    stats.insert(
        PathBuf::from("."),
        stats_with_children(&["root.rs"], &[], 0),
    );
    stats.insert(
        PathBuf::from("nested"),
        stats_with_children(&["root.rs", "cache.tmp"], &[], 1),
    );
    stats.insert(
        PathBuf::from("src"),
        stats_with_children(&["generated.rs"], &[], 1),
    );
    stats.insert(
        PathBuf::from("other/src"),
        stats_with_children(&["generated.rs"], &[], 2),
    );

    let violations = checker.check(&stats);

    // "./root.rs" excludes only the root-level file; "*.tmp" excludes by
    // basename everywhere; "./src/**" excludes the src subtree only.
    assert_eq!(violations.len(), 2);
    assert_eq!(violations[0].path, PathBuf::from("nested"));
    assert_eq!(violations[0].actual, 1);
    assert_eq!(violations[1].path, PathBuf::from("other/src"));
    assert_eq!(violations[1].actual, 1);
}

#[test]
fn directories_are_excluded_from_dir_count() {
    let config = StructureConfig {
        max_dirs: Some(1),
        count_exclude: vec!["build".to_string()],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        stats_with_children(&[], &["api", "build"], 1),
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn excluded_directory_still_counts_without_pattern() {
    let config = StructureConfig {
        max_dirs: Some(1),
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src"),
        stats_with_children(&[], &["api", "build"], 1),
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, ViolationType::DirCount);
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn warn_thresholds_apply_to_effective_counts() {
    let config = StructureConfig {
        max_files: Some(10),
        warn_threshold: Some(0.8), // Warn above 8 files
        count_exclude: vec!["*.md".to_string()],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    // 10 raw children, 8 effective: at (not above) the warn boundary.
    let files: Vec<String> = (0..8)
        .map(|i| format!("f{i}.rs"))
        .chain(["a.md".to_string(), "b.md".to_string()])
        .collect();
    let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
    stats.insert(
        PathBuf::from("src"),
        stats_with_children(&file_refs, &[], 1),
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn count_exclude_applies_within_rule_scoped_limits() {
    // The global exclusion caliber applies no matter which rule's limits win.
    let config = StructureConfig {
        max_files: Some(100),
        count_exclude: vec!["*.gen".to_string()],
        rules: vec![StructureRule {
            scope: "src/api/**".to_string(),
            max_files: Some(1),
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/api/v1"),
        stats_with_children(&["handler.rs", "schema.gen"], &[], 3),
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn invalid_count_exclude_pattern_returns_error() {
    let config = StructureConfig {
        max_files: Some(10),
        count_exclude: vec!["[invalid".to_string()],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);

    assert!(result.is_err());
}

// =============================================================================
// Rule-level count_exclude: the rule that wins limit resolution (last match)
// also defines the counting caliber, unioned with the global exclusions.
// =============================================================================

#[test]
fn rule_count_exclude_applies_only_within_rule_scope() {
    let config = StructureConfig {
        max_files: Some(1),
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            count_exclude: vec!["*.gen".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/api"),
        stats_with_children(&["handler.rs", "schema.gen"], &[], 2),
    );
    stats.insert(
        PathBuf::from("docs"),
        stats_with_children(&["index.md", "site.gen"], &[], 1),
    );

    let violations = checker.check(&stats);

    // The rule sets no limits, so max_files falls back to the global value,
    // but its counting caliber still applies inside its scope. Outside the
    // scope, *.gen counts normally.
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("docs"));
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn earlier_matching_rule_count_exclude_is_superseded() {
    // Both rules match src/api/v1; the LAST one wins and it defines the
    // caliber, so the first rule's *.gen exclusion must not leak through.
    let config = StructureConfig {
        rules: vec![
            StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(1),
                count_exclude: vec!["*.gen".to_string()],
                ..Default::default()
            },
            StructureRule {
                scope: "src/api/**".to_string(),
                max_files: Some(1),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/api/v1"),
        stats_with_children(&["main.rs", "schema.gen"], &[], 3),
    );
    stats.insert(
        PathBuf::from("src/lib"),
        stats_with_children(&["main.rs", "schema.gen"], &[], 2),
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/api/v1"));
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn last_matching_rule_count_exclude_wins() {
    // Mirror of the superseded case: the exclusion sits on the LAST rule.
    let config = StructureConfig {
        rules: vec![
            StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(1),
                ..Default::default()
            },
            StructureRule {
                scope: "src/api/**".to_string(),
                max_files: Some(1),
                count_exclude: vec!["*.gen".to_string()],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/api/v1"),
        stats_with_children(&["main.rs", "schema.gen"], &[], 3),
    );
    stats.insert(
        PathBuf::from("src/lib"),
        stats_with_children(&["main.rs", "schema.gen"], &[], 2),
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/lib"));
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn rule_count_exclude_unions_with_global() {
    // Global housekeeping excludes stay active when a rule adds its own.
    let config = StructureConfig {
        count_exclude: vec![".gitkeep".to_string()],
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            max_files: Some(1),
            count_exclude: vec!["*.gen".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/api"),
        stats_with_children(&["main.rs", "schema.gen", ".gitkeep"], &[], 2),
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn global_count_exclude_applies_when_matched_rule_adds_none() {
    let config = StructureConfig {
        count_exclude: vec![".gitkeep".to_string()],
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            max_files: Some(1),
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/api"),
        stats_with_children(&["main.rs", ".gitkeep"], &[], 2),
    );

    let violations = checker.check(&stats);

    assert!(violations.is_empty());
}

#[test]
fn rule_count_exclude_path_qualified_vs_basename() {
    // Within one rule's scope: a path-qualified pattern only excludes under
    // its anchored directory, while an unqualified pattern matches basenames
    // anywhere the rule applies.
    let config = StructureConfig {
        max_files: Some(1),
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            count_exclude: vec!["src/api/*.gen".to_string(), "*.tmp".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/api"),
        stats_with_children(&["main.rs", "schema.gen", "cache.tmp"], &[], 2),
    );
    stats.insert(
        PathBuf::from("src/lib"),
        stats_with_children(&["main.rs", "schema.gen", "cache.tmp"], &[], 2),
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/lib"));
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn rule_count_exclude_applies_to_dir_count() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            max_dirs: Some(1),
            count_exclude: vec!["build".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let mut stats = HashMap::new();
    stats.insert(
        PathBuf::from("src/app"),
        stats_with_children(&[], &["api", "build"], 2),
    );
    stats.insert(
        PathBuf::from("src/pkg"),
        stats_with_children(&[], &["api", "cache"], 2),
    );

    let violations = checker.check(&stats);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/pkg"));
    assert_eq!(violations[0].violation_type, ViolationType::DirCount);
    assert_eq!(violations[0].actual, 2);
}

#[test]
fn invalid_rule_count_exclude_pattern_returns_error() {
    let config = StructureConfig {
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            max_files: Some(10),
            count_exclude: vec!["[invalid".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = StructureChecker::new(&config);

    assert!(result.is_err());
}

// =============================================================================
// Explain: count_exclude provenance and raw vs effective counts.
// =============================================================================

#[test]
fn explain_lists_global_then_winning_rule_count_exclude() {
    let config = StructureConfig {
        count_exclude: vec![".gitkeep".to_string()],
        rules: vec![
            StructureRule {
                scope: "src/**".to_string(),
                count_exclude: vec!["*.gen".to_string()],
                ..Default::default()
            },
            StructureRule {
                scope: "src/api/**".to_string(),
                count_exclude: vec!["*.tmp".to_string()],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let explanation = checker.explain(&PathBuf::from("src/api/v1"), None);

    // Global first, then the winning (last-match) rule; the superseded
    // rule's *.gen contributes nothing even though its scope matches.
    assert_eq!(explanation.counts, None);
    assert_eq!(explanation.count_exclude.len(), 2);
    assert_eq!(explanation.count_exclude[0].pattern, ".gitkeep");
    assert_eq!(
        explanation.count_exclude[0].source,
        CountExcludeSource::Global
    );
    assert_eq!(explanation.count_exclude[1].pattern, "*.tmp");
    assert_eq!(
        explanation.count_exclude[1].source,
        CountExcludeSource::Rule {
            index: 1,
            scope: "src/api/**".to_string()
        }
    );
    assert!(explanation.count_exclude[1].excluded_files.is_empty());
}

#[test]
fn explain_reports_raw_and_effective_counts_with_per_pattern_hits() {
    let config = StructureConfig {
        count_exclude: vec!["*.md".to_string()],
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            count_exclude: vec!["*.gen".to_string(), "build".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let stats = stats_with_children(
        &["main.rs", "README.md", "schema.gen"],
        &["api", "build"],
        2,
    );

    let explanation = checker.explain(&PathBuf::from("src/app"), Some(&stats));

    let counts = explanation.counts.unwrap();
    assert_eq!(counts.raw_file_count, 3);
    assert_eq!(counts.raw_dir_count, 2);
    assert_eq!(counts.effective_file_count, 1);
    assert_eq!(counts.effective_dir_count, 1);

    assert_eq!(
        explanation.count_exclude[0].excluded_files,
        vec!["README.md"]
    );
    assert!(explanation.count_exclude[0].excluded_dirs.is_empty());
    assert_eq!(
        explanation.count_exclude[1].excluded_files,
        vec!["schema.gen"]
    );
    assert_eq!(explanation.count_exclude[2].excluded_dirs, vec!["build"]);
    assert!(explanation.count_exclude[2].excluded_files.is_empty());
}

#[test]
fn explain_attributes_child_to_every_matching_pattern() {
    // A child matched by both a global and a rule pattern shows up under
    // each, while the effective count drops it only once.
    let config = StructureConfig {
        count_exclude: vec!["*.md".to_string()],
        rules: vec![StructureRule {
            scope: "docs/**".to_string(),
            count_exclude: vec!["README.*".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let stats = stats_with_children(&["README.md", "guide.md"], &[], 2);

    let explanation = checker.explain(&PathBuf::from("docs/site"), Some(&stats));

    assert_eq!(
        explanation.count_exclude[0].excluded_files,
        vec!["README.md", "guide.md"]
    );
    assert_eq!(
        explanation.count_exclude[1].excluded_files,
        vec!["README.md"]
    );
    assert_eq!(explanation.counts.unwrap().effective_file_count, 0);
}

#[test]
fn explain_shows_rule_caliber_even_when_limits_fall_back_to_global() {
    // The winning rule defines the counting caliber even though every limit
    // field falls back to the global value.
    let config = StructureConfig {
        max_files: Some(5),
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            count_exclude: vec!["*.gen".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();

    let explanation = checker.explain(&PathBuf::from("src/api"), None);

    assert_eq!(explanation.effective_max_files, Some(5));
    assert_eq!(explanation.count_exclude.len(), 1);
    assert_eq!(
        explanation.count_exclude[0].source,
        CountExcludeSource::Rule {
            index: 0,
            scope: "src/**".to_string()
        }
    );
}

#[test]
fn explain_outside_any_rule_lists_only_global_patterns() {
    let config = StructureConfig {
        count_exclude: vec!["*.md".to_string()],
        rules: vec![StructureRule {
            scope: "src/**".to_string(),
            count_exclude: vec!["*.gen".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let stats = stats_with_children(&["site.gen", "index.md"], &[], 1);

    let explanation = checker.explain(&PathBuf::from("docs"), Some(&stats));

    assert_eq!(explanation.count_exclude.len(), 1);
    assert_eq!(
        explanation.count_exclude[0].source,
        CountExcludeSource::Global
    );
    let counts = explanation.counts.unwrap();
    assert_eq!(counts.raw_file_count, 2);
    // *.gen is not active here: the rule did not win for this directory.
    assert_eq!(counts.effective_file_count, 1);
}

#[test]
fn explain_path_qualified_pattern_reports_no_hits_elsewhere() {
    let config = StructureConfig {
        count_exclude: vec!["docs/*.md".to_string()],
        ..Default::default()
    };
    let checker = StructureChecker::new(&config).unwrap();
    let stats = stats_with_children(&["README.md"], &[], 1);

    let explanation = checker.explain(&PathBuf::from("src"), Some(&stats));

    assert!(explanation.count_exclude[0].excluded_files.is_empty());
    let counts = explanation.counts.unwrap();
    assert_eq!(counts.effective_file_count, counts.raw_file_count);
}
