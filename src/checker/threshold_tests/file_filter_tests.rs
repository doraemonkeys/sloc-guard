//! Tests for `should_process` extension and rule filtering.

use std::path::Path;

use super::*;

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
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);

    // Dockerfile should be processed because it matches a rule
    assert!(checker.should_process(Path::new("Dockerfile")));
    assert!(checker.should_process(Path::new("docker/Dockerfile")));
    // Jenkinsfile still skipped (no matching rule)
    assert!(!checker.should_process(Path::new("Jenkinsfile")));
}

#[test]
fn should_process_extension_less_file_with_rule_exact_path() {
    let mut config = default_config();
    config.content.extensions = vec!["rs".to_string()];
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/Jenkinsfile".to_string(),
        max_lines: 200,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: Some("CI pipeline".to_string()),
        expires: None,
    });

    let checker = ThresholdChecker::new(config);

    // Jenkinsfile should be processed because it matches a rule
    assert!(checker.should_process(Path::new("Jenkinsfile")));
    assert!(checker.should_process(Path::new("ci/Jenkinsfile")));
    // Dockerfile still skipped (no matching rule)
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
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);

    // Files in scripts/ should be processed regardless of extension
    assert!(checker.should_process(Path::new("scripts/setup")));
    assert!(checker.should_process(Path::new("scripts/deploy.sh")));
    // Files outside scripts/ without .rs extension should be skipped
    assert!(!checker.should_process(Path::new("bin/setup")));
}
