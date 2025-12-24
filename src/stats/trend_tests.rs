use std::path::PathBuf;

use super::*;
use crate::config::TrendConfig;
use crate::counter::LineStats;
use crate::output::{FileStatistics, ProjectStatistics};

fn sample_project_stats(total_files: usize, total_code: usize) -> ProjectStatistics {
    let files: Vec<FileStatistics> = (0..total_files)
        .map(|i| FileStatistics {
            path: PathBuf::from(format!("file{i}.rs")),
            stats: LineStats {
                total: total_code / total_files + 20,
                code: total_code / total_files,
                comment: 10,
                blank: 10,
                ignored: 0,
            },
            language: "Rust".to_string(),
        })
        .collect();
    ProjectStatistics::new(files)
}

#[test]
fn test_trend_entry_new() {
    let stats = sample_project_stats(5, 100);
    let entry = TrendEntry::new(&stats);

    assert_eq!(entry.total_files, 5);
    assert_eq!(entry.code, 100);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_trend_entry_with_timestamp() {
    let stats = sample_project_stats(5, 100);
    let entry = TrendEntry::new(&stats).with_timestamp(12345);

    assert_eq!(entry.timestamp, 12345);
}

#[test]
fn test_trend_delta_compute_increase() {
    let prev = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    };
    let current = sample_project_stats(15, 600);

    let delta = TrendDelta::compute(&prev, &current);

    assert_eq!(delta.files_delta, 5);
    assert_eq!(delta.code_delta, 100);
    assert!(delta.has_changes());
    assert_eq!(delta.previous_timestamp, Some(1000));
}

#[test]
fn test_trend_delta_compute_decrease() {
    let prev = TrendEntry {
        timestamp: 1000,
        total_files: 20,
        total_lines: 2000,
        code: 1000,
        comment: 600,
        blank: 400,
    };
    let current = sample_project_stats(10, 500);

    let delta = TrendDelta::compute(&prev, &current);

    assert_eq!(delta.files_delta, -10);
    assert_eq!(delta.code_delta, -500);
    assert!(delta.has_changes());
}

#[test]
fn test_trend_delta_no_changes() {
    let prev = TrendEntry {
        timestamp: 1000,
        total_files: 5,
        total_lines: 150,
        code: 100,
        comment: 50,
        blank: 50,
    };

    // Create stats with same totals
    let files = vec![FileStatistics {
        path: PathBuf::from("file.rs"),
        stats: LineStats {
            total: 150,
            code: 100,
            comment: 50,
            blank: 50,
            ignored: 0,
        },
        language: "Rust".to_string(),
    }];

    // Manually create stats with 5 files
    let mut stats = ProjectStatistics::new(files);
    // Override to match previous
    stats.total_files = 5;
    stats.total_lines = 150;
    stats.total_code = 100;
    stats.total_comment = 50;
    stats.total_blank = 50;

    let delta = TrendDelta::compute(&prev, &stats);

    assert!(!delta.has_changes());
}

#[test]
fn test_trend_history_new() {
    let history = TrendHistory::new();

    assert!(history.is_empty());
    assert_eq!(history.len(), 0);
    assert_eq!(history.version(), 1);
}

#[test]
fn test_trend_history_add() {
    let mut history = TrendHistory::new();
    let stats = sample_project_stats(5, 100);

    history.add(&stats);

    assert!(!history.is_empty());
    assert_eq!(history.len(), 1);
    assert!(history.latest().is_some());
}

#[test]
fn test_trend_history_add_entry() {
    let mut history = TrendHistory::new();
    let entry = TrendEntry {
        timestamp: 12345,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    };

    history.add_entry(entry);

    assert_eq!(history.len(), 1);
    assert_eq!(history.latest().unwrap().timestamp, 12345);
}

#[test]
fn test_trend_history_compute_delta_empty() {
    let history = TrendHistory::new();
    let stats = sample_project_stats(5, 100);

    let delta = history.compute_delta(&stats);

    assert!(delta.is_none());
}

#[test]
fn test_trend_history_compute_delta_with_entry() {
    let mut history = TrendHistory::new();
    let entry = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    };
    history.add_entry(entry);

    let current = sample_project_stats(15, 600);
    let delta = history.compute_delta(&current);

    assert!(delta.is_some());
    let delta = delta.unwrap();
    assert_eq!(delta.files_delta, 5);
    assert_eq!(delta.code_delta, 100);
}

#[test]
fn test_trend_history_entries() {
    let mut history = TrendHistory::new();

    history.add_entry(TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    });
    history.add_entry(TrendEntry {
        timestamp: 2000,
        total_files: 15,
        total_lines: 150,
        code: 75,
        comment: 45,
        blank: 30,
    });

    let entries = history.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].timestamp, 1000);
    assert_eq!(entries[1].timestamp, 2000);
}

#[test]
fn test_trend_history_save_and_load() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let history_path = temp_dir.path().join("history.json");

    // Create and save history
    let mut history = TrendHistory::new();
    history.add_entry(TrendEntry {
        timestamp: 12345,
        total_files: 10,
        total_lines: 1000,
        code: 500,
        comment: 300,
        blank: 200,
    });
    history.save(&history_path).unwrap();

    // Load and verify
    let loaded = TrendHistory::load(&history_path).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.latest().unwrap().timestamp, 12345);
    assert_eq!(loaded.latest().unwrap().code, 500);
}

#[test]
fn test_trend_history_load_or_default_nonexistent() {
    let history = TrendHistory::load_or_default(Path::new("nonexistent_file.json"));
    assert!(history.is_empty());
}

#[test]
fn test_trend_delta_default() {
    let delta = TrendDelta::default();

    assert_eq!(delta.files_delta, 0);
    assert_eq!(delta.lines_delta, 0);
    assert_eq!(delta.code_delta, 0);
    assert_eq!(delta.comment_delta, 0);
    assert_eq!(delta.blank_delta, 0);
    assert!(delta.previous_timestamp.is_none());
    assert!(!delta.has_changes());
}

#[test]
fn test_trend_history_load_nonexistent_returns_error() {
    let result = TrendHistory::load(Path::new("this_file_does_not_exist_12345.json"));
    assert!(result.is_err());
}

#[test]
fn test_trend_history_load_or_default_invalid_json() {
    use std::io::Write;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let invalid_path = temp_dir.path().join("invalid_history.json");

    // Write invalid JSON to the file
    let mut file = std::fs::File::create(&invalid_path).unwrap();
    file.write_all(b"{ invalid json }").unwrap();
    drop(file);

    // load_or_default should return default (empty) history when file is invalid
    let history = TrendHistory::load_or_default(&invalid_path);
    assert!(history.is_empty());
}

#[test]
fn test_trend_history_default_trait() {
    let history = TrendHistory::default();
    assert!(history.is_empty());
    assert_eq!(history.len(), 0);
    assert_eq!(history.version(), 1);
}

#[test]
fn test_trend_entry_equality() {
    let entry1 = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    };
    let entry2 = TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    };
    let entry3 = TrendEntry {
        timestamp: 2000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    };

    assert_eq!(entry1, entry2);
    assert_ne!(entry1, entry3);
}

#[test]
fn test_trend_history_equality() {
    let mut history1 = TrendHistory::new();
    let mut history2 = TrendHistory::new();

    assert_eq!(history1, history2);

    history1.add_entry(TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    });

    assert_ne!(history1, history2);

    history2.add_entry(TrendEntry {
        timestamp: 1000,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    });

    assert_eq!(history1, history2);
}

#[test]
fn test_trend_history_load_or_default_valid_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let history_path = temp_dir.path().join("history.json");

    // Create and save a valid history file
    let mut history = TrendHistory::new();
    history.add_entry(TrendEntry {
        timestamp: 5000,
        total_files: 20,
        total_lines: 2000,
        code: 1000,
        comment: 600,
        blank: 400,
    });
    history.save(&history_path).unwrap();

    // load_or_default should load the file successfully
    let loaded = TrendHistory::load_or_default(&history_path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.latest().unwrap().timestamp, 5000);
}

// ============================================================================
// Retention Policy Tests
// ============================================================================

fn make_entry(timestamp: u64) -> TrendEntry {
    TrendEntry {
        timestamp,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    }
}

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
fn test_trend_config_serialization() {
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
fn test_trend_config_toml_deserialization() {
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
fn test_trend_config_partial_toml() {
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

// ============================================================================
// Significance Threshold Tests
// ============================================================================

#[test]
fn test_is_significant_file_change_always_significant() {
    let delta = TrendDelta {
        files_delta: 1, // File added
        code_delta: 0,  // No code change
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(delta.is_significant(&config));
}

#[test]
fn test_is_significant_file_removed_always_significant() {
    let delta = TrendDelta {
        files_delta: -1, // File removed
        code_delta: 5,   // Small code change
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(delta.is_significant(&config));
}

#[test]
fn test_is_significant_code_above_default_threshold() {
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
fn test_is_significant_code_at_default_threshold_not_significant() {
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
fn test_is_significant_code_below_default_threshold_not_significant() {
    let delta = TrendDelta {
        files_delta: 0,
        code_delta: 5,
        ..Default::default()
    };
    let config = TrendConfig::default();

    assert!(!delta.is_significant(&config));
}

#[test]
fn test_is_significant_negative_code_delta_uses_absolute_value() {
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
fn test_is_significant_negative_code_below_threshold() {
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
fn test_is_significant_custom_threshold() {
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
fn test_is_significant_custom_threshold_exceeded() {
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
fn test_is_significant_zero_threshold_always_significant() {
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
fn test_is_significant_zero_delta_with_zero_threshold() {
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
fn test_is_significant_no_changes_not_significant() {
    let delta = TrendDelta::default();
    let config = TrendConfig::default();

    assert!(!delta.is_significant(&config));
}

#[test]
fn test_is_significant_only_comment_blank_changes_not_significant() {
    // Only comment and blank changes, no code or file changes
    let delta = TrendDelta {
        files_delta: 0,
        lines_delta: 100,
        code_delta: 0,
        comment_delta: 50,
        blank_delta: 50,
        previous_timestamp: Some(1000),
    };
    let config = TrendConfig::default();

    // Comment/blank changes don't count toward significance threshold
    assert!(!delta.is_significant(&config));
}
