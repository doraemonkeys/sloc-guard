//! Tests for `TrendLineChart`.

use super::*;

// Timestamp constants for test dates
const TS_2023_12_24: u64 = 1_703_376_000;
const TS_2023_12_25: u64 = 1_703_462_400;
const TS_2023_12_26: u64 = 1_703_548_800;
const TS_2024_02_29: u64 = 1_709_164_800;
const SECS_PER_DAY: u64 = 86400;

fn create_test_entry(timestamp: u64, code: usize) -> crate::stats::TrendEntry {
    crate::stats::TrendEntry {
        timestamp,
        total_files: 10,
        total_lines: code + 100,
        code,
        comment: 50,
        blank: 50,
        git_ref: None,
        git_branch: None,
    }
}

fn create_test_entry_with_git(
    timestamp: u64,
    code: usize,
    git_ref: Option<&str>,
    git_branch: Option<&str>,
) -> crate::stats::TrendEntry {
    crate::stats::TrendEntry {
        timestamp,
        total_files: 10,
        total_lines: code + 100,
        code,
        comment: 50,
        blank: 50,
        git_ref: git_ref.map(String::from),
        git_branch: git_branch.map(String::from),
    }
}

#[test]
fn empty_history_renders_no_data_message() {
    let history = TrendHistory::new();
    let chart = TrendLineChart::from_history(&history);

    assert!(!chart.has_data());

    let svg = chart.render();
    assert!(svg.contains("No trend data"));
}

#[test]
fn single_entry_renders_chart() {
    let mut history = TrendHistory::new();
    history.add_entry(create_test_entry(TS_2023_12_25, 500));

    let chart = TrendLineChart::from_history(&history);
    assert!(chart.has_data());

    let svg = chart.render();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("viewBox"));
    assert!(svg.contains("Code Lines Over Time"));
}

#[test]
fn multiple_entries_render_line() {
    let mut history = TrendHistory::new();
    history.add_entry(create_test_entry(TS_2023_12_24, 400));
    history.add_entry(create_test_entry(TS_2023_12_25, 450));
    history.add_entry(create_test_entry(TS_2023_12_26, 500));

    let chart = TrendLineChart::from_history(&history);
    assert!(chart.has_data());

    let svg = chart.render();
    // Should have path elements for line and area
    assert!(svg.contains("<path"));
    // Should have circle elements for data points
    assert!(svg.contains("<circle"));
}

#[test]
fn git_ref_appears_in_label() {
    let mut history = TrendHistory::new();
    history.add_entry(create_test_entry_with_git(
        TS_2023_12_25,
        500,
        Some("abc1234"),
        None,
    ));

    let chart = TrendLineChart::from_history(&history);
    let svg = chart.render();

    // Git ref should appear in tooltip
    assert!(svg.contains("abc1234"));
}

#[test]
fn git_branch_appears_when_no_ref() {
    let mut history = TrendHistory::new();
    history.add_entry(create_test_entry_with_git(
        TS_2023_12_25,
        500,
        None,
        Some("main"),
    ));

    let chart = TrendLineChart::from_history(&history);
    let svg = chart.render();

    // Branch should appear when no ref
    assert!(svg.contains("main"));
}

#[test]
fn git_ref_preferred_over_branch() {
    let mut history = TrendHistory::new();
    history.add_entry(create_test_entry_with_git(
        TS_2023_12_25,
        500,
        Some("abc1234"),
        Some("main"),
    ));

    let chart = TrendLineChart::from_history(&history);
    let svg = chart.render();

    // Git ref should appear
    assert!(svg.contains("abc1234"));
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn downsampling_preserves_first_and_last() {
    let mut history = TrendHistory::new();

    // Add 50 entries (more than MAX_POINTS of 30)
    for i in 0..50 {
        let timestamp = TS_2023_12_24 + i * SECS_PER_DAY;
        let code = 400 + i as usize * 10;
        history.add_entry(create_test_entry(timestamp, code));
    }

    let chart = TrendLineChart::from_history(&history);
    let svg = chart.render();

    // Should still render successfully
    assert!(chart.has_data());
    assert!(svg.contains("<svg"));
    assert!(svg.contains("<circle"));
}

#[test]
fn timestamp_to_date_conversion() {
    // Test a known date: 2023-12-25 00:00:00 UTC
    let (year, month, day) = TrendLineChart::timestamp_to_date(TS_2023_12_25);
    assert_eq!(year, 2023);
    assert_eq!(month, 12);
    assert_eq!(day, 25);
}

#[test]
fn timestamp_to_date_unix_epoch() {
    // 1970-01-01 00:00:00 UTC = 0
    let (year, month, day) = TrendLineChart::timestamp_to_date(0);
    assert_eq!(year, 1970);
    assert_eq!(month, 1);
    assert_eq!(day, 1);
}

#[test]
fn timestamp_to_date_leap_year() {
    // 2024-02-29 00:00:00 UTC
    let (year, month, day) = TrendLineChart::timestamp_to_date(TS_2024_02_29);
    assert_eq!(year, 2024);
    assert_eq!(month, 2);
    assert_eq!(day, 29);
}

#[test]
fn is_leap_year_check() {
    assert!(TrendLineChart::is_leap_year(2000)); // Divisible by 400
    assert!(!TrendLineChart::is_leap_year(1900)); // Divisible by 100 but not 400
    assert!(TrendLineChart::is_leap_year(2024)); // Divisible by 4, not by 100
    assert!(!TrendLineChart::is_leap_year(2023)); // Not divisible by 4
}

#[test]
fn format_timestamp_produces_mm_dd() {
    let formatted = TrendLineChart::format_timestamp(TS_2023_12_25);
    assert_eq!(formatted, "12/25");
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn downsample_less_than_max_returns_all() {
    let entries: Vec<_> = (0..10)
        .map(|i| create_test_entry(TS_2023_12_24 + i * SECS_PER_DAY, 400 + i as usize * 10))
        .collect();

    let result = TrendLineChart::downsample(&entries, 30);
    assert_eq!(result.len(), 10);
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn downsample_exactly_max_returns_all() {
    let entries: Vec<_> = (0..30)
        .map(|i| create_test_entry(TS_2023_12_24 + i * SECS_PER_DAY, 400 + i as usize * 10))
        .collect();

    let result = TrendLineChart::downsample(&entries, 30);
    assert_eq!(result.len(), 30);
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn downsample_more_than_max_reduces() {
    let entries: Vec<_> = (0..50)
        .map(|i| create_test_entry(TS_2023_12_24 + i * SECS_PER_DAY, 400 + i as usize * 10))
        .collect();

    let result = TrendLineChart::downsample(&entries, 30);
    assert_eq!(result.len(), 30);

    // First and last should be preserved
    assert_eq!(result[0].timestamp, entries[0].timestamp);
    assert_eq!(result[29].timestamp, entries[49].timestamp);
}

#[test]
fn chart_customization() {
    let mut history = TrendHistory::new();
    history.add_entry(create_test_entry(TS_2023_12_25, 500));

    let chart = TrendLineChart::from_history(&history)
        .with_size(600.0, 300.0)
        .with_color(ChartColor::hex("#ff0000"));

    let svg = chart.render();
    assert!(svg.contains("600")); // Width in viewBox
    assert!(svg.contains("300")); // Height in viewBox
    assert!(svg.contains("#ff0000")); // Custom color
}

#[test]
fn chart_renders_accessible_title() {
    let mut history = TrendHistory::new();
    history.add_entry(create_test_entry(TS_2023_12_25, 500));

    let chart = TrendLineChart::from_history(&history);
    let svg = chart.render();

    assert!(svg.contains("<title>"));
    assert!(svg.contains("Code Lines Over Time"));
    assert!(svg.contains(r#"role="img""#));
}
