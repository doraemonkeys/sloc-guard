use std::path::Path;
use std::path::PathBuf;

use super::*;

fn default_config() -> Config {
    Config::default()
}

fn stats_with_code(code: usize) -> LineStats {
    LineStats {
        total: code + 10,
        code,
        comment: 5,
        blank: 5,
        ignored: 0,
    }
}

#[test]
fn check_passes_under_threshold() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 500);
}

#[test]
fn check_fails_over_threshold() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(600);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_failed());
}

#[test]
fn check_warns_near_threshold() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(460);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_warning());
}

#[test]
fn check_uses_rule_specific_limit() {
    let mut config = default_config();
    // Use V2 content.rules format instead of legacy config.rules
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 300,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(350);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_failed());
    assert_eq!(result.limit(), 300);
}

#[test]
fn check_uses_override_for_specific_file() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "legacy.rs".to_string(),
        max_lines: 800,
        reason: Some("Legacy code".to_string()),
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(700);

    let result = checker.check(Path::new("src/legacy.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);
    assert_eq!(result.override_reason(), Some("Legacy code"));
}

#[test]
fn check_override_without_reason() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "special.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(700);

    let result = checker.check(Path::new("src/special.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);
    assert_eq!(result.override_reason(), None);
}

#[test]
fn check_no_override_reason_when_using_default() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.override_reason(), None);
}

#[test]
fn check_result_usage_percent() {
    let result = CheckResult::Passed {
        path: PathBuf::from("test.rs"),
        stats: LineStats {
            total: 260,
            code: 250,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
    };

    assert!((result.usage_percent() - 50.0).abs() < 0.01);
}

#[test]
fn custom_warning_threshold() {
    let checker = ThresholdChecker::new(default_config()).with_warning_threshold(0.8);
    let stats = stats_with_code(410);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_warning());
}

#[test]
fn override_does_not_match_partial_filename() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "parser.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // "my_parser.rs" should NOT match override for "parser.rs"
    let result = checker.check(Path::new("src/my_parser.rs"), &stats);

    assert!(result.is_failed());
    assert_eq!(result.limit(), 500); // default limit, not override
}

#[test]
fn override_matches_exact_filename() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "parser.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // "parser.rs" should match override for "parser.rs"
    let result = checker.check(Path::new("src/parser.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);
}

#[test]
fn override_matches_full_path() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "src/legacy/parser.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // "src/legacy/parser.rs" should match
    let result = checker.check(Path::new("src/legacy/parser.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);

    // "other/src/legacy/parser.rs" should also match (ends with)
    let result2 = checker.check(Path::new("other/src/legacy/parser.rs"), &stats);
    assert!(result2.is_passed());
    assert_eq!(result2.limit(), 800);

    // "legacy/parser.rs" should NOT match (missing "src" component)
    let result3 = checker.check(Path::new("legacy/parser.rs"), &stats);
    assert!(result3.is_failed());
    assert_eq!(result3.limit(), 500);
}

#[test]
fn check_result_is_methods() {
    let passed = CheckResult::Passed {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(100),
        limit: 500,
        override_reason: None,
    };
    let failed = CheckResult::Failed {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(600),
        limit: 500,
        override_reason: None,
        suggestions: None,
    };
    let warning = CheckResult::Warning {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(450),
        limit: 500,
        override_reason: None,
        suggestions: None,
    };

    assert!(passed.is_passed());
    assert!(!passed.is_failed());
    assert!(failed.is_failed());
    assert!(!failed.is_passed());
    assert!(warning.is_warning());
    assert!(!warning.is_failed());
}

#[test]
fn path_rule_matches_glob_pattern() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(800);

    // File matching the glob pattern should use path_rule max_lines
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit(), 1000);
}

#[test]
fn path_rule_does_not_match_unrelated_path() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // File not matching the pattern should use default
    let result = checker.check(Path::new("src/lib.rs"), &stats);
    assert!(result.is_failed());
    assert_eq!(result.limit(), 500);
}

#[test]
fn path_rule_has_lower_priority_than_override() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });
    config.overrides.push(crate::config::FileOverride {
        path: "special.rs".to_string(),
        max_lines: 2000,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(1500);

    // Override should take priority over path_rule
    let result = checker.check(Path::new("src/generated/special.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit(), 2000);
}

#[test]
fn path_rule_has_higher_priority_than_extension_rule() {
    let mut config = default_config();
    // Use V2 content.rules format: extension rule first, then specific path rule
    // Since "last match wins", the path rule should override the extension rule
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 300,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
    });
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/proto/**".to_string(),
        max_lines: 800,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(400);

    // path_rule should override extension rule (last match wins)
    let result = checker.check(Path::new("src/proto/messages.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);

    // Non-matching path should use extension rule
    let result2 = checker.check(Path::new("src/lib.rs"), &stats);
    assert!(result2.is_failed());
    assert_eq!(result2.limit(), 300);
}

#[test]
fn path_rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: Some(1.0), // Disable warnings
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(999); // 99.9% of limit

    // With warn_threshold=1.0, should not warn
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed());
}

#[test]
fn path_rule_without_warn_threshold_uses_default() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(950); // 95% of limit, above 90%

    // Without custom warn_threshold, should use default (0.9)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_warning());
}

#[test]
fn multiple_path_rules_last_match_wins() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/**".to_string(),
        max_lines: 600,
        warn_threshold: None,
    });
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(700);

    // Last matching rule should be used (1000 limit)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed()); // 700 < 1000
    assert_eq!(result.limit(), 1000);
}

#[test]
fn rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    // Use V2 content.rules format
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: Some(0.8),
        skip_comments: None,
        skip_blank: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // With rule warn_threshold=0.8, should warn at 82%
    let result = checker.check(Path::new("test.rs"), &stats);
    assert!(result.is_warning());
}

#[test]
fn rule_without_warn_threshold_uses_default() {
    let mut config = default_config();
    // Use V2 content.rules format
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // Without rule warn_threshold, should use default 0.9 (no warning at 82%)
    let result = checker.check(Path::new("test.rs"), &stats);
    assert!(result.is_passed());
}

#[test]
fn path_rule_warn_threshold_overrides_extension_rule() {
    let mut config = default_config();
    // Use V2 content.rules format: extension rule first, then specific path rule
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: Some(0.8),
        skip_comments: None,
        skip_blank: None,
    });
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 500,
        warn_threshold: Some(1.0), // Disable warnings
        skip_comments: None,
        skip_blank: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(450); // 90% of limit

    // path_rule warn_threshold=1.0 should override extension rule's 0.8 (last match wins)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed());

    // Non-matching path should use extension rule's warn_threshold=0.8
    let result2 = checker.check(Path::new("src/lib.rs"), &stats);
    assert!(result2.is_warning());
}

#[test]
fn get_skip_settings_from_path_rule() {
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = true;
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: Some(false),
        skip_blank: Some(false),
    });

    let checker = ThresholdChecker::new(config);

    // Matching path should use rule's skip settings
    let (skip_comments, skip_blank) =
        checker.get_skip_settings_for_path(Path::new("src/generated/parser.rs"));
    assert!(!skip_comments);
    assert!(!skip_blank);
}

#[test]
fn get_skip_settings_falls_back_to_global() {
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = false;
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: None, // Not specified
        skip_blank: None,    // Not specified
    });

    let checker = ThresholdChecker::new(config);

    // Should use global defaults when rule doesn't specify
    let (skip_comments, skip_blank) =
        checker.get_skip_settings_for_path(Path::new("src/generated/parser.rs"));
    assert!(skip_comments); // from global
    assert!(!skip_blank); // from global
}

#[test]
fn get_skip_settings_uses_global_when_no_rule_matches() {
    let mut config = default_config();
    config.content.skip_comments = false;
    config.content.skip_blank = true;

    let checker = ThresholdChecker::new(config);

    // Non-matching path should use global defaults
    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(Path::new("src/lib.rs"));
    assert!(!skip_comments);
    assert!(skip_blank);
}

#[test]
fn get_skip_settings_last_match_wins() {
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = true;
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/**".to_string(),
        max_lines: 500,
        warn_threshold: None,
        skip_comments: Some(false),
        skip_blank: Some(false),
    });
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: Some(true),
        skip_blank: Some(false),
    });

    let checker = ThresholdChecker::new(config);

    // Last matching rule should be used
    let (skip_comments, skip_blank) =
        checker.get_skip_settings_for_path(Path::new("src/generated/parser.rs"));
    assert!(skip_comments); // From last matching rule
    assert!(!skip_blank); // From last matching rule

    // First rule only for non-generated
    let (skip_comments2, skip_blank2) = checker.get_skip_settings_for_path(Path::new("src/lib.rs"));
    assert!(!skip_comments2);
    assert!(!skip_blank2);
}

#[test]
fn language_rule_expanded_skip_settings_work() {
    // This tests that language rules (expanded to content.rules by loader)
    // correctly apply skip_comments/skip_blank
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = true;

    // Simulate expanded language rule (what loader does)
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 300,
        warn_threshold: None,
        skip_comments: Some(false),
        skip_blank: Some(true),
    });

    let checker = ThresholdChecker::new(config);

    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(Path::new("src/lib.rs"));
    assert!(!skip_comments); // From rule
    assert!(skip_blank); // From rule
}

// Tests for should_process (Task 12.10: Rule Matching Overrides Extension Filter)

#[test]
fn should_process_allows_all_when_no_extensions_configured() {
    let mut config = default_config();
    config.content.extensions = vec![]; // Explicitly clear extensions

    let checker = ThresholdChecker::new(config);

    assert!(checker.should_process(Path::new("src/lib.rs")));
    assert!(checker.should_process(Path::new("Dockerfile")));
    assert!(checker.should_process(Path::new("Jenkinsfile")));
}

#[test]
fn should_process_filters_by_extension() {
    let mut config = default_config();
    config.content.extensions = vec!["rs".to_string(), "ts".to_string()];

    let checker = ThresholdChecker::new(config);

    assert!(checker.should_process(Path::new("src/lib.rs")));
    assert!(checker.should_process(Path::new("app.ts")));
    assert!(!checker.should_process(Path::new("app.js"))); // .js not in extensions
}

#[test]
fn should_process_extension_less_file_skipped_without_rule() {
    let mut config = default_config();
    config.content.extensions = vec!["rs".to_string()];

    let checker = ThresholdChecker::new(config);

    // Extension-less files should be skipped when no rule matches
    assert!(!checker.should_process(Path::new("Dockerfile")));
    assert!(!checker.should_process(Path::new("Jenkinsfile")));
    assert!(!checker.should_process(Path::new("Makefile")));
}

#[test]
fn should_process_extension_less_file_with_content_rule() {
    let mut config = default_config();
    config.content.extensions = vec!["rs".to_string()];
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/Dockerfile".to_string(),
        max_lines: 100,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
    });

    let checker = ThresholdChecker::new(config);

    // Dockerfile should be processed because it matches a rule
    assert!(checker.should_process(Path::new("Dockerfile")));
    assert!(checker.should_process(Path::new("docker/Dockerfile")));
    // Jenkinsfile still skipped (no matching rule)
    assert!(!checker.should_process(Path::new("Jenkinsfile")));
}

#[test]
fn should_process_extension_less_file_with_content_override() {
    let mut config = default_config();
    config.content.extensions = vec!["rs".to_string()];
    config.content.overrides.push(crate::config::ContentOverride {
        path: "Jenkinsfile".to_string(),
        max_lines: 200,
        reason: "CI pipeline".to_string(),
    });

    let checker = ThresholdChecker::new(config);

    // Jenkinsfile should be processed because it matches an override
    assert!(checker.should_process(Path::new("Jenkinsfile")));
    assert!(checker.should_process(Path::new("ci/Jenkinsfile")));
    // Dockerfile still skipped (no matching rule or override)
    assert!(!checker.should_process(Path::new("Dockerfile")));
}

#[test]
fn should_process_extension_less_file_with_legacy_override() {
    let mut config = default_config();
    config.content.extensions = vec!["rs".to_string()];
    config.overrides.push(crate::config::FileOverride {
        path: "Makefile".to_string(),
        max_lines: 300,
        reason: Some("Build config".to_string()),
    });

    let checker = ThresholdChecker::new(config);

    // Makefile should be processed because it matches a legacy override
    assert!(checker.should_process(Path::new("Makefile")));
    // Dockerfile still skipped
    assert!(!checker.should_process(Path::new("Dockerfile")));
}

#[test]
fn should_process_extension_less_file_with_glob_rule() {
    let mut config = default_config();
    config.content.extensions = vec!["rs".to_string()];
    // Rule that matches any file in scripts/ directory
    config.content.rules.push(crate::config::ContentRule {
        pattern: "scripts/**".to_string(),
        max_lines: 100,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
    });

    let checker = ThresholdChecker::new(config);

    // Files in scripts/ should be processed regardless of extension
    assert!(checker.should_process(Path::new("scripts/setup")));
    assert!(checker.should_process(Path::new("scripts/deploy.sh")));
    // Files outside scripts/ without .rs extension should be skipped
    assert!(!checker.should_process(Path::new("bin/setup")));
}
