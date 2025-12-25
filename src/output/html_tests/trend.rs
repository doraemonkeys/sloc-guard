//! Tests for trend line chart in HTML output.

use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::{FileStatistics, HtmlFormatter, OutputFormatter, ProjectStatistics};
use crate::stats::{TrendEntry, TrendHistory};

use super::make_passed_result;

// Timestamp constants for test dates
const TS_2023_12_24: u64 = 1_703_376_000;
const TS_2023_12_25: u64 = 1_703_462_400;
const TS_2023_12_26: u64 = 1_703_548_800;

fn make_trend_entry(timestamp: u64, code: usize) -> TrendEntry {
    TrendEntry {
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

fn make_file_stats(path: &str, code: usize, language: &str) -> FileStatistics {
    FileStatistics {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        language: language.to_string(),
    }
}

fn make_project_stats_with_languages(files: Vec<FileStatistics>) -> ProjectStatistics {
    ProjectStatistics::new(files).with_language_breakdown()
}

#[test]
fn trend_chart_not_shown_without_history() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("Code Lines Over Time"));
}

#[test]
fn trend_chart_not_shown_with_empty_history() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let history = TrendHistory::new();

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Empty history should not show chart section
    assert!(!output.contains("Code Lines Over Time"));
    assert!(!output.contains("Visualizations"));
}

#[test]
fn trend_chart_shown_with_history() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Visualizations"));
    assert!(output.contains("Code Lines Over Time"));
    assert!(output.contains("<svg"));
}

#[test]
fn trend_chart_has_line_path() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));
    history.add_entry(make_trend_entry(TS_2023_12_26, 500));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Should have path element for the line
    assert!(output.contains("<path"));
}

#[test]
fn trend_chart_has_data_points() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Should have circle elements for data points
    assert!(output.contains("<circle"));
}

#[test]
fn trend_chart_with_stats_shows_both() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    // Trend history
    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    // Project stats
    let files = vec![
        make_file_stats("a.rs", 100, "Rust"),
        make_file_stats("b.rs", 200, "Rust"),
        make_file_stats("c.rs", 150, "Rust"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new()
        .with_stats(stats)
        .with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // All three charts should be present
    assert!(output.contains("Code Lines Over Time"));
    assert!(output.contains("File Size Distribution"));
    assert!(output.contains("Language Breakdown"));
}

#[test]
fn trend_chart_appears_first_in_visualizations() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let files = vec![
        make_file_stats("a.rs", 100, "Rust"),
        make_file_stats("b.rs", 200, "Rust"),
        make_file_stats("c.rs", 150, "Rust"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new()
        .with_stats(stats)
        .with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Trend chart should appear before histogram
    let trend_pos = output.find("Code Lines Over Time").unwrap();
    let hist_pos = output.find("File Size Distribution").unwrap();
    assert!(trend_pos < hist_pos, "Trend chart should appear first");
}

#[test]
fn only_trend_chart_when_no_stats() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let mut history = TrendHistory::new();
    history.add_entry(make_trend_entry(TS_2023_12_24, 400));
    history.add_entry(make_trend_entry(TS_2023_12_25, 450));

    let formatter = HtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&results).unwrap();

    // Only trend chart should appear
    assert!(output.contains("Visualizations"));
    assert!(output.contains("Code Lines Over Time"));
    assert!(!output.contains("File Size Distribution"));
    assert!(!output.contains("Language Breakdown"));
}
