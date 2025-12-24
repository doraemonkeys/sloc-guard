//! Tests for warning threshold behavior.

use std::path::Path;

use super::*;

#[test]
fn path_rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: Some(1.0), // Disable warnings
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(999); // 99.9% of limit

    // With warn_threshold=1.0, should not warn
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats, None);
    assert!(result.is_passed());
}

#[test]
fn path_rule_without_warn_threshold_uses_default() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(950); // 95% of limit, above 90%

    // Without custom warn_threshold, should use default (0.9)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats, None);
    assert!(result.is_warning());
}

#[test]
fn rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    // Use V2 content.rules format
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: Some(0.8),
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // With rule warn_threshold=0.8, should warn at 82%
    let result = checker.check(Path::new("test.rs"), &stats, None);
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
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // Without rule warn_threshold, should use default 0.9 (no warning at 82%)
    let result = checker.check(Path::new("test.rs"), &stats, None);
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
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 500,
        warn_threshold: Some(1.0), // Disable warnings
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(450); // 90% of limit

    // path_rule warn_threshold=1.0 should override extension rule's 0.8 (last match wins)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats, None);
    assert!(result.is_passed());

    // Non-matching path should use extension rule's warn_threshold=0.8
    let result2 = checker.check(Path::new("src/lib.rs"), &stats, None);
    assert!(result2.is_warning());
}

#[test]
fn multiple_rules_winner_takes_all_warn_threshold() {
    let mut config = default_config();
    config.content.warn_threshold = 0.9;

    // Rule 1: Warn strict (0.5)
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 1000,
        warn_threshold: Some(0.5),
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    // Rule 2: Override for specific file, default warn threshold (None)
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/main.rs".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600); // 60% of limit.
    // If inherit 0.5 -> Warn.
    // If winner takes all 0.9 -> Pass.

    let result = checker.check(Path::new("src/main.rs"), &stats, None);

    // With fix, this should PASS (uses 0.9).
    assert!(result.is_passed());
}

// ============================================================================
// warn_at (absolute threshold) tests
// ============================================================================

#[test]
fn rule_warn_at_takes_precedence_over_percentage_threshold() {
    let mut config = default_config();
    config.content.warn_threshold = 0.9; // Would warn at 450 for 500 lines
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: Some(0.8), // Would warn at 400
        warn_at: Some(350),        // Absolute: warn at 350 (takes precedence)
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);

    // 360 lines: above 350 (absolute), below 400 (percentage) → should warn
    let result = checker.check(Path::new("test.rs"), &stats_with_code(360), None);
    assert!(
        result.is_warning(),
        "Should warn at 360 (above 350 absolute)"
    );

    // 340 lines: below 350 → should pass
    let result = checker.check(Path::new("test.rs"), &stats_with_code(340), None);
    assert!(
        result.is_passed(),
        "Should pass at 340 (below 350 absolute)"
    );
}

#[test]
fn global_warn_at_takes_precedence_over_global_percentage() {
    let mut config = default_config();
    config.content.max_lines = 500;
    config.content.warn_threshold = 0.9; // Would warn at 450
    config.content.warn_at = Some(400); // Absolute: warn at 400 (takes precedence)

    let checker = ThresholdChecker::new(config);

    // 420 lines: above 400 (absolute), below 450 (percentage) → should warn
    let result = checker.check(Path::new("test.rs"), &stats_with_code(420), None);
    assert!(
        result.is_warning(),
        "Should warn at 420 (above 400 absolute)"
    );

    // 380 lines: below 400 → should pass
    let result = checker.check(Path::new("test.rs"), &stats_with_code(380), None);
    assert!(
        result.is_passed(),
        "Should pass at 380 (below 400 absolute)"
    );
}

#[test]
fn rule_warn_at_overrides_global_warn_at() {
    let mut config = default_config();
    config.content.max_lines = 500;
    config.content.warn_at = Some(450); // Global absolute

    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/**".to_string(),
        max_lines: 500,
        warn_threshold: None,
        warn_at: Some(350), // Rule absolute (overrides global)
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);

    // Matching file: should use rule's warn_at (350)
    let result = checker.check(Path::new("src/lib.rs"), &stats_with_code(360), None);
    assert!(
        result.is_warning(),
        "Rule file should warn at 360 (above 350)"
    );

    let result = checker.check(Path::new("src/lib.rs"), &stats_with_code(340), None);
    assert!(
        result.is_passed(),
        "Rule file should pass at 340 (below 350)"
    );

    // Non-matching file: should use global warn_at (450)
    let result = checker.check(Path::new("other.rs"), &stats_with_code(460), None);
    assert!(
        result.is_warning(),
        "Non-rule file should warn at 460 (above 450)"
    );

    let result = checker.check(Path::new("other.rs"), &stats_with_code(440), None);
    assert!(
        result.is_passed(),
        "Non-rule file should pass at 440 (below 450)"
    );
}

#[test]
fn rule_warn_threshold_used_when_no_warn_at() {
    let mut config = default_config();
    config.content.max_lines = 500;
    config.content.warn_at = Some(400); // Global absolute

    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/**".to_string(),
        max_lines: 1000,
        warn_threshold: Some(0.8), // Should warn at 800 (1000 * 0.8)
        warn_at: None,             // No rule absolute → use rule percentage
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);

    // Rule file: should use rule's warn_threshold (800)
    let result = checker.check(Path::new("src/lib.rs"), &stats_with_code(810), None);
    assert!(result.is_warning(), "Should warn at 810 (above 800)");

    let result = checker.check(Path::new("src/lib.rs"), &stats_with_code(790), None);
    assert!(result.is_passed(), "Should pass at 790 (below 800)");
}

#[test]
fn warn_at_explain_shows_effective_warn_at() {
    let mut config = default_config();
    config.content.max_lines = 500;
    config.content.warn_at = Some(400);

    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        warn_at: Some(750),
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);

    // Matching file: effective_warn_at should be 750
    let exp = checker.explain(Path::new("src/lib.rs"));
    assert_eq!(exp.effective_warn_at, 750);
    assert_eq!(exp.effective_limit, 1000);

    // Non-matching file: effective_warn_at should be 400 (global warn_at)
    let exp = checker.explain(Path::new("other.rs"));
    assert_eq!(exp.effective_warn_at, 400);
    assert_eq!(exp.effective_limit, 500);
}
