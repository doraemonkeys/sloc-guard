//! Tests for retention policy: `should_add`, `apply_retention`, `save_with_retention`.

use super::*;

#[test]
fn test_should_add_empty_history_always_true() {
    let history = TrendHistory::new();
    let config = TrendConfig {
        min_interval_secs: Some(3600),
        ..Default::default()
    };

    assert!(history.should_add(&config, 1000));
}

#[test]
fn test_should_add_no_min_interval_always_true() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(1000));

    let config = TrendConfig::default();
    assert!(history.should_add(&config, 1001));
}

#[test]
fn test_should_add_within_interval_returns_false() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(1000));

    let config = TrendConfig {
        min_interval_secs: Some(3600), // 1 hour
        ..Default::default()
    };

    // Only 1800 seconds passed (30 minutes)
    assert!(!history.should_add(&config, 2800));
}

#[test]
fn test_should_add_after_interval_returns_true() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(1000));

    let config = TrendConfig {
        min_interval_secs: Some(3600), // 1 hour
        ..Default::default()
    };

    // 3600 seconds passed (exactly 1 hour)
    assert!(history.should_add(&config, 4600));
}

#[test]
fn test_should_add_exactly_at_interval_returns_true() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(1000));

    let config = TrendConfig {
        min_interval_secs: Some(100),
        ..Default::default()
    };

    assert!(history.should_add(&config, 1100));
}

#[test]
fn test_apply_retention_max_entries() {
    let mut history = TrendHistory::new();
    for i in 0..10 {
        history.add_entry(make_entry(i * 100));
    }
    assert_eq!(history.len(), 10);

    let config = TrendConfig {
        max_entries: Some(5),
        ..Default::default()
    };

    let removed = history.apply_retention(&config, 10000);

    assert_eq!(removed, 5);
    assert_eq!(history.len(), 5);
    // Should keep the 5 newest (timestamps 500-900)
    assert_eq!(history.entries()[0].timestamp, 500);
    assert_eq!(history.entries()[4].timestamp, 900);
}

#[test]
fn test_apply_retention_max_entries_no_excess() {
    let mut history = TrendHistory::new();
    for i in 0..3 {
        history.add_entry(make_entry(i * 100));
    }

    let config = TrendConfig {
        max_entries: Some(10),
        ..Default::default()
    };

    let removed = history.apply_retention(&config, 10000);

    assert_eq!(removed, 0);
    assert_eq!(history.len(), 3);
}

#[test]
fn test_apply_retention_max_age_days() {
    let mut history = TrendHistory::new();
    // Add entries at various ages (use large enough timestamp to avoid overflow)
    let current_time = 10_000_000u64;
    let one_day = SECONDS_PER_DAY;

    history.add_entry(make_entry(current_time - 10 * one_day)); // 10 days old
    history.add_entry(make_entry(current_time - 5 * one_day)); // 5 days old
    history.add_entry(make_entry(current_time - 2 * one_day)); // 2 days old
    history.add_entry(make_entry(current_time - one_day)); // 1 day old

    let config = TrendConfig {
        max_age_days: Some(7),
        ..Default::default()
    };

    let removed = history.apply_retention(&config, current_time);

    assert_eq!(removed, 1); // Only the 10-day-old entry removed
    assert_eq!(history.len(), 3);
}

#[test]
fn test_apply_retention_max_age_days_removes_all_old() {
    let mut history = TrendHistory::new();
    // Use large enough timestamp to avoid overflow
    let current_time = 100_000_000u64;
    let one_day = SECONDS_PER_DAY;

    // All entries older than 30 days
    history.add_entry(make_entry(current_time - 100 * one_day));
    history.add_entry(make_entry(current_time - 50 * one_day));

    let config = TrendConfig {
        max_age_days: Some(30),
        ..Default::default()
    };

    let removed = history.apply_retention(&config, current_time);

    assert_eq!(removed, 2);
    assert!(history.is_empty());
}

#[test]
fn test_apply_retention_combined_age_and_entries() {
    let mut history = TrendHistory::new();
    // Use large enough timestamp to avoid overflow
    let current_time = 100_000_000u64;
    let one_day = SECONDS_PER_DAY;

    // 10 entries, some old, some recent (index 0 = oldest, index 9 = newest)
    for i in 0u64..10 {
        let age_days = 10 - i;
        history.add_entry(make_entry(current_time - age_days * one_day));
    }

    // First remove entries older than 7 days (entries 0-2), then trim to 3 entries
    let config = TrendConfig {
        max_age_days: Some(7),
        max_entries: Some(3),
        ..Default::default()
    };

    let removed = history.apply_retention(&config, current_time);

    // 3 removed by age, then 4 more by max_entries (7 total entries → 3)
    assert_eq!(removed, 7);
    assert_eq!(history.len(), 3);
}

#[test]
fn test_apply_retention_no_config_no_changes() {
    let mut history = TrendHistory::new();
    for i in 0..5 {
        history.add_entry(make_entry(i * 100));
    }

    let config = TrendConfig::default();
    let removed = history.apply_retention(&config, 10000);

    assert_eq!(removed, 0);
    assert_eq!(history.len(), 5);
}

#[test]
fn test_add_if_allowed_respects_interval() {
    // Note: add_if_allowed uses current system time, so we can't deterministically
    // test interval blocking. The should_add method is tested separately with
    // explicit timestamps for deterministic behavior.
    let mut history = TrendHistory::new();
    let stats = sample_project_stats(5, 100);
    let config = TrendConfig::default();

    // Empty history should always allow
    let added = history.add_if_allowed(&stats, &config);
    assert!(added);
    assert_eq!(history.len(), 1);
}

#[test]
fn test_save_with_retention_applies_cleanup() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let history_path = temp_dir.path().join("history.json");

    let mut history = TrendHistory::new();
    for i in 0..10 {
        history.add_entry(make_entry(i * 100));
    }

    let config = TrendConfig {
        max_entries: Some(3),
        ..Default::default()
    };

    history.save_with_retention(&history_path, &config).unwrap();

    // Verify file contains only 3 entries
    let loaded = TrendHistory::load(&history_path).unwrap();
    assert_eq!(loaded.len(), 3);
    // Should be the 3 newest
    assert_eq!(loaded.entries()[0].timestamp, 700);
    assert_eq!(loaded.entries()[2].timestamp, 900);
}

#[test]
fn test_save_with_retention_age_based() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let history_path = temp_dir.path().join("history.json");

    let mut history = TrendHistory::new();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let one_day = SECONDS_PER_DAY;
    // Add entries: 100 days old, 50 days old, 1 day old
    history.add_entry(make_entry(current_time - 100 * one_day));
    history.add_entry(make_entry(current_time - 50 * one_day));
    history.add_entry(make_entry(current_time - one_day));

    let config = TrendConfig {
        max_age_days: Some(30),
        ..Default::default()
    };

    history.save_with_retention(&history_path, &config).unwrap();

    let loaded = TrendHistory::load(&history_path).unwrap();
    assert_eq!(loaded.len(), 1); // Only the 1-day-old entry remains
}

#[test]
fn test_config_serialization() {
    let config = TrendConfig {
        max_entries: Some(1000),
        max_age_days: Some(365),
        min_interval_secs: Some(60),
        min_code_delta: Some(20),
    };

    let json = serde_json::to_string(&config).unwrap();
    let parsed: TrendConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.max_entries, Some(1000));
    assert_eq!(parsed.max_age_days, Some(365));
    assert_eq!(parsed.min_interval_secs, Some(60));
    assert_eq!(parsed.min_code_delta, Some(20));
}

#[test]
fn test_config_toml_deserialization() {
    let toml_str = r"
max_entries = 500
max_age_days = 90
min_interval_secs = 300
min_code_delta = 25
";
    let config: TrendConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(config.max_entries, Some(500));
    assert_eq!(config.max_age_days, Some(90));
    assert_eq!(config.min_interval_secs, Some(300));
    assert_eq!(config.min_code_delta, Some(25));
}

#[test]
fn test_config_partial_toml() {
    let toml_str = r"
max_entries = 100
";
    let config: TrendConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(config.max_entries, Some(100));
    assert!(config.max_age_days.is_none());
    assert!(config.min_interval_secs.is_none());
    assert!(config.min_code_delta.is_none());
}

#[test]
fn test_apply_retention_max_entries_zero() {
    let mut history = TrendHistory::new();
    for i in 0..5 {
        history.add_entry(make_entry(i * 100));
    }

    let config = TrendConfig {
        max_entries: Some(0),
        ..Default::default()
    };

    let removed = history.apply_retention(&config, 10000);

    assert_eq!(removed, 5);
    assert!(history.is_empty());
}

#[test]
fn test_apply_retention_max_age_zero() {
    let mut history = TrendHistory::new();
    let current_time = 10_000_000u64;
    history.add_entry(make_entry(current_time - 100)); // Very recent

    let config = TrendConfig {
        max_age_days: Some(0),
        ..Default::default()
    };

    let removed = history.apply_retention(&config, current_time);

    // Entry is within 0 days old (cutoff = current_time - 0 = current_time)
    // Entry timestamp < cutoff, so removed
    assert_eq!(removed, 1);
    assert!(history.is_empty());
}

#[test]
fn test_should_add_with_zero_interval() {
    let mut history = TrendHistory::new();
    history.add_entry(make_entry(1000));

    let config = TrendConfig {
        min_interval_secs: Some(0),
        ..Default::default()
    };

    // Zero interval means always allow
    assert!(history.should_add(&config, 1000));
}
