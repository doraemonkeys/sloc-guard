//! Tests for `TrendDelta` significance thresholds.

use super::*;

#[test]
fn test_file_change_always_significant() {
    let delta = TrendDelta {
        files_delta: 1, // File added
        code_delta: 0,  // No code change
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(delta.is_significant(&config));
}

#[test]
fn test_file_removed_always_significant() {
    let delta = TrendDelta {
        files_delta: -1, // File removed
        code_delta: 5,   // Small code change
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(delta.is_significant(&config));
}

#[test]
fn test_code_above_default_threshold() {
    // Default threshold is 10, so 11 lines should be significant
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 11,
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(delta.is_significant(&config));
}

#[test]
fn test_code_at_default_threshold_not_significant() {
    // Exactly 10 lines is NOT significant (threshold is >10, not >=10)
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 10,
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(!delta.is_significant(&config));
}

#[test]
fn test_code_below_default_threshold_not_significant() {
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 5,
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(!delta.is_significant(&config));
}

#[test]
fn test_negative_code_delta_uses_absolute_value() {
    // -15 lines should be significant (abs > 10)
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: -15,
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(delta.is_significant(&config));
}

#[test]
fn test_negative_code_below_threshold() {
    // -5 lines should NOT be significant (abs <= 10)
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: -5,
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(!delta.is_significant(&config));
}

#[test]
fn test_custom_threshold() {
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 50,
        ..Default::default()
    };
    let config = TrendConfig {
        min_code_delta: Some(100), // Custom high threshold
        ..Default::default()
    };

    // 50 lines is below the 100-line threshold
    assert!(!delta.is_significant(&config));
}

#[test]
fn test_custom_threshold_exceeded() {
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 150,
        ..Default::default()
    };
    let config = TrendConfig {
        min_code_delta: Some(100),
        ..Default::default()
    };

    assert!(delta.is_significant(&config));
}

#[test]
fn test_zero_threshold_always_significant() {
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 1,
        ..Default::default()
    };
    let config = TrendConfig {
        min_code_delta: Some(0), // Any change is significant
        ..Default::default()
    };

    assert!(delta.is_significant(&config));
}

#[test]
fn test_zero_delta_with_zero_threshold() {
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 0,
        ..Default::default()
    };
    let config = TrendConfig {
        min_code_delta: Some(0),
        ..Default::default()
    };

    // 0 > 0 is false, so not significant
    assert!(!delta.is_significant(&config));
}

#[test]
fn test_no_changes_not_significant() {
    let delta = TrendDelta::default();
    let config = TrendConfig::default();

    assert!(!delta.is_significant(&config));
}

#[test]
fn test_only_comment_blank_changes_not_significant() {
    // Only comment and blank changes, no code or file changes
    let delta = TrendDelta {
        files_delta: 0,
        lines_delta: 100,
        code_delta: 0,
        comment_delta: 50,
        blank_delta: 50,
        previous_timestamp: Some(1000),
        previous_git_ref: None,
        previous_git_branch: None,
    };
    let config = TrendConfig::default();

    // Comment/blank changes don't count toward significance threshold
    assert!(!delta.is_significant(&config));
}
