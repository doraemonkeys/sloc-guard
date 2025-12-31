//! Tests for auto-snapshot functionality during check.
//!
//! These tests verify `auto_snapshot_on_check` functionality by directly testing
//! the `TrendConfig` parsing and the `perform_auto_snapshot` helper.
//! Integration tests that require changing the current directory are in
//! `tests/check_integration_tests.rs`.

#[test]
fn auto_snapshot_config_parsing() {
    // Test that auto_snapshot_on_check is correctly parsed from TOML
    let toml_str = r#"
version = "2"

[trend]
auto_snapshot_on_check = true
"#;
    let config: crate::config::Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.trend.auto_snapshot_on_check, Some(true));
}

#[test]
fn auto_snapshot_config_default_is_none() {
    // Test that auto_snapshot_on_check defaults to None
    let toml_str = r#"
version = "2"
"#;
    let config: crate::config::Config = toml::from_str(toml_str).unwrap();
    assert!(config.trend.auto_snapshot_on_check.is_none());
}

#[test]
fn auto_snapshot_respects_min_interval() {
    use crate::output::ProjectStatistics;
    use crate::stats::TrendHistory;

    // Create initial history with a recent entry
    let mut history = TrendHistory::new();
    let stats = ProjectStatistics::new(vec![]);
    history.add(&stats);

    // Configure with min_interval_secs
    let config = crate::config::Config {
        trend: crate::config::TrendConfig {
            min_interval_secs: Some(3600), // 1 hour
            ..Default::default()
        },
        ..Default::default()
    };

    // Try to add another entry immediately - should be skipped
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    assert!(
        !history.should_add(&config.trend, current_time),
        "Should not add entry when min_interval_secs not elapsed"
    );
}
