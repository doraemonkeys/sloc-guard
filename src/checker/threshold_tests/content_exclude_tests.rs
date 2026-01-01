//! Tests for content exclude patterns.

use std::path::Path;

use super::*;

#[test]
fn content_exclude_skips_matching_files() {
    let mut config = default_config();
    config.content.exclude = vec!["**/*.generated.ts".to_string()];

    let checker = ThresholdChecker::new(config).unwrap();

    assert!(!checker.should_process(Path::new("src/api.generated.ts")));
    assert!(!checker.should_process(Path::new("lib/models.generated.ts")));
    // Non-matching files are still processed
    assert!(checker.should_process(Path::new("src/api.ts")));
}

#[test]
fn content_exclude_takes_priority_over_extension_match() {
    let mut config = default_config();
    config.content.extensions = vec!["ts".to_string()];
    config.content.exclude = vec!["**/*.generated.ts".to_string()];

    let checker = ThresholdChecker::new(config).unwrap();

    // .ts is in extensions, but excluded pattern takes priority
    assert!(!checker.should_process(Path::new("src/api.generated.ts")));
    // Regular .ts files are processed
    assert!(checker.should_process(Path::new("src/api.ts")));
}

#[test]
fn content_exclude_multiple_patterns() {
    let mut config = default_config();
    config.content.exclude = vec![
        "**/*.generated.ts".to_string(),
        "**/*.pb.go".to_string(),
        "**/vendor/**".to_string(),
    ];

    let checker = ThresholdChecker::new(config).unwrap();

    assert!(!checker.should_process(Path::new("src/api.generated.ts")));
    assert!(!checker.should_process(Path::new("proto/service.pb.go")));
    assert!(!checker.should_process(Path::new("vendor/github.com/pkg/lib.go")));
    // Non-matching files are processed
    assert!(checker.should_process(Path::new("src/main.go")));
}

#[test]
fn content_exclude_empty_has_no_effect() {
    let mut config = default_config();
    config.content.exclude = vec![];

    let checker = ThresholdChecker::new(config).unwrap();

    // All files with matching extensions are processed
    assert!(checker.should_process(Path::new("src/api.ts")));
    assert!(checker.should_process(Path::new("src/main.rs")));
}

#[test]
fn is_content_excluded_method() {
    let mut config = default_config();
    config.content.exclude = vec!["**/*.generated.ts".to_string()];

    let checker = ThresholdChecker::new(config).unwrap();

    assert!(checker.is_content_excluded(Path::new("src/api.generated.ts")));
    assert!(!checker.is_content_excluded(Path::new("src/api.ts")));
}

#[test]
fn explain_shows_excluded_status() {
    let mut config = default_config();
    config.content.exclude = vec!["**/*.generated.ts".to_string()];

    let checker = ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(Path::new("src/api.generated.ts"));

    assert!(explanation.is_excluded);
    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Excluded { .. }
    ));
    assert_eq!(explanation.effective_limit, 0);
}

#[test]
fn explain_shows_matching_exclude_pattern() {
    let mut config = default_config();
    config.content.exclude = vec!["**/*.generated.ts".to_string(), "**/*.pb.go".to_string()];

    let checker = ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(Path::new("src/api.generated.ts"));

    if let ContentRuleMatch::Excluded { pattern } = explanation.matched_rule {
        assert_eq!(pattern, "**/*.generated.ts");
    } else {
        panic!("Expected ContentRuleMatch::Excluded");
    }
}

#[test]
fn explain_non_excluded_file_has_is_excluded_false() {
    let mut config = default_config();
    config.content.exclude = vec!["**/*.generated.ts".to_string()];

    let checker = ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(Path::new("src/api.ts"));

    assert!(!explanation.is_excluded);
    assert!(!matches!(
        explanation.matched_rule,
        ContentRuleMatch::Excluded { .. }
    ));
}
