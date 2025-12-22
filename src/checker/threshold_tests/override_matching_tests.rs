//! Tests for override path matching behavior.

use std::path::Path;

use super::*;

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
