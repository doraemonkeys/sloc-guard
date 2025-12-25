//! Tests for `StatsHtmlFormatter`.

use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::stats::{
    DirectoryStats, FileStatistics, LanguageStats, ProjectStatistics, StatsFormatter,
    StatsHtmlFormatter,
};
use crate::stats::{TrendDelta, TrendEntry, TrendHistory};

fn sample_stats() -> LineStats {
    LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    }
}

fn sample_file_statistics() -> Vec<FileStatistics> {
    vec![
        FileStatistics {
            path: PathBuf::from("src/main.rs"),
            stats: sample_stats(),
            language: "Rust".to_string(),
        },
        FileStatistics {
            path: PathBuf::from("src/lib.rs"),
            stats: LineStats {
                total: 50,
                code: 40,
                comment: 8,
                blank: 2,
                ignored: 0,
            },
            language: "Rust".to_string(),
        },
    ]
}

// ============================================================================
// Basic Formatting
// ============================================================================

#[test]
fn format_basic_stats() {
    let stats = ProjectStatistics::new(sample_file_statistics());
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Basic structure checks
    assert!(output.contains("<!DOCTYPE html>"));
    assert!(output.contains("</html>"));
    assert!(output.contains("SLOC Guard Report"));

    // Summary cards
    assert!(output.contains("Total Files"));
    assert!(output.contains("Total Lines"));
    assert!(output.contains("Code"));
    assert!(output.contains("Comments"));
    assert!(output.contains("Blanks"));

    // Values present
    assert!(output.contains(">2</span>")); // 2 files
    assert!(output.contains(">150</span>")); // 100 + 50 = 150 total lines
    assert!(output.contains(">120</span>")); // 80 + 40 = 120 code
}

#[test]
fn format_with_average() {
    let stats = ProjectStatistics::new(sample_file_statistics()).with_top_files(2);
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Average is shown when top_files is computed
    assert!(output.contains("Avg Code/File"));
    assert!(output.contains("60.0")); // (80 + 40) / 2 = 60
}

// ============================================================================
// Language Breakdown
// ============================================================================

#[test]
fn format_with_language_breakdown() {
    let stats = ProjectStatistics::new(sample_file_statistics()).with_language_breakdown();
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Section header
    assert!(output.contains("Language Breakdown"));

    // Table structure
    assert!(output.contains("<table>"));
    assert!(output.contains("<th>Language</th>"));

    // Rust data
    assert!(output.contains("Rust"));
}

#[test]
fn format_empty_language_breakdown_omitted() {
    // Create stats without files (empty project)
    let stats = ProjectStatistics {
        files: vec![],
        total_files: 0,
        total_lines: 0,
        total_code: 0,
        total_comment: 0,
        total_blank: 0,
        by_language: Some(vec![]), // Empty language breakdown
        by_directory: None,
        top_files: None,
        average_code_lines: None,
        trend: None,
    };

    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Empty breakdown should NOT show the section
    assert!(!output.contains("<h2>Language Breakdown</h2>"));
}

// ============================================================================
// Directory Breakdown
// ============================================================================

#[test]
fn format_with_directory_breakdown() {
    let stats = ProjectStatistics::new(sample_file_statistics()).with_directory_breakdown();
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Section header
    assert!(output.contains("Directory Breakdown"));

    // Table structure
    assert!(output.contains("<th>Directory</th>"));

    // Directory path
    assert!(output.contains("src"));
}

#[test]
fn format_directory_breakdown_with_relative_paths() {
    let stats = ProjectStatistics::new(sample_file_statistics())
        .with_directory_breakdown_relative(Some(std::path::Path::new(".")));
    let formatter = StatsHtmlFormatter::new().with_project_root(Some(PathBuf::from(".")));
    let output = formatter.format(&stats).unwrap();

    assert!(output.contains("Directory Breakdown"));
}

// ============================================================================
// Top Files
// ============================================================================

#[test]
fn format_with_top_files() {
    let stats = ProjectStatistics::new(sample_file_statistics()).with_top_files(5);
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Section header with count
    assert!(output.contains("Top 2 Largest Files"));

    // Table structure
    assert!(output.contains("<th>File</th>"));
    assert!(output.contains("<th>Language</th>"));

    // File paths
    assert!(output.contains("src/main.rs"));
    assert!(output.contains("src/lib.rs"));

    // Rank numbers
    assert!(output.contains(">1</td>"));
    assert!(output.contains(">2</td>"));
}

#[test]
fn format_top_files_with_project_root() {
    let stats = ProjectStatistics::new(vec![FileStatistics {
        path: PathBuf::from("/project/src/main.rs"),
        stats: sample_stats(),
        language: "Rust".to_string(),
    }])
    .with_top_files(1);

    let formatter = StatsHtmlFormatter::new().with_project_root(Some(PathBuf::from("/project")));
    let output = formatter.format(&stats).unwrap();

    // Should show relative path
    assert!(output.contains("src/main.rs"));
}

// ============================================================================
// Trend Section
// ============================================================================

#[test]
fn format_with_trend_delta() {
    let trend = TrendDelta {
        files_delta: 5,
        lines_delta: 100,
        code_delta: 80,
        comment_delta: 15,
        blank_delta: 5,
        previous_timestamp: None,
        previous_git_ref: Some("abc123".to_string()),
        previous_git_branch: Some("main".to_string()),
    };

    let stats = ProjectStatistics::new(sample_file_statistics()).with_trend(trend);
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Trend header with git context
    assert!(output.contains("Changes since commit abc123 on main"));

    // Delta cards
    assert!(output.contains("+5")); // files delta
    assert!(output.contains("+100")); // lines delta
    assert!(output.contains("+80")); // code delta
}

#[test]
fn format_without_trend_omits_section() {
    let stats = ProjectStatistics::new(sample_file_statistics());
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // No trend section
    assert!(!output.contains("Changes since"));
    assert!(!output.contains("Changes from"));
}

// ============================================================================
// Charts Section
// ============================================================================

#[test]
fn format_with_trend_history_shows_chart() {
    let stats = ProjectStatistics::new(sample_file_statistics()).with_language_breakdown();

    // Create trend history with entries
    let mut history = TrendHistory::new();
    history.add_entry(TrendEntry {
        timestamp: 1_700_000_000,
        total_files: 1,
        total_lines: 100,
        code: 80,
        comment: 15,
        blank: 5,
        git_ref: None,
        git_branch: None,
    });
    history.add_entry(TrendEntry {
        timestamp: 1_700_100_000,
        total_files: 2,
        total_lines: 150,
        code: 120,
        comment: 23,
        blank: 7,
        git_ref: None,
        git_branch: None,
    });

    let formatter = StatsHtmlFormatter::new().with_trend_history(history);
    let output = formatter.format(&stats).unwrap();

    // Charts section present
    assert!(output.contains("Visualizations"));
    assert!(output.contains("Code Lines Over Time"));
    assert!(output.contains("<svg"));
}

#[test]
fn format_with_language_chart() {
    let stats = ProjectStatistics::new(sample_file_statistics()).with_language_breakdown();
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Language chart present
    assert!(output.contains("Language Breakdown"));
    assert!(output.contains("<svg")); // SVG chart rendered
}

#[test]
fn format_no_charts_when_no_data() {
    // Stats without language breakdown or trend history
    let stats = ProjectStatistics::new(sample_file_statistics());
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // No visualizations section
    assert!(!output.contains("Visualizations"));
}

// ============================================================================
// HTML Escaping
// ============================================================================

#[test]
fn format_escapes_special_characters_in_language() {
    let stats = ProjectStatistics {
        files: vec![],
        total_files: 0,
        total_lines: 0,
        total_code: 0,
        total_comment: 0,
        total_blank: 0,
        by_language: Some(vec![LanguageStats {
            language: "C++ <special>".to_string(),
            files: 1,
            total_lines: 100,
            code: 80,
            comment: 15,
            blank: 5,
        }]),
        by_directory: None,
        top_files: None,
        average_code_lines: None,
        trend: None,
    };

    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Special characters should be escaped
    assert!(output.contains("C++ &lt;special&gt;"));
    assert!(!output.contains("<special>"));
}

#[test]
fn format_escapes_special_characters_in_directory() {
    let stats = ProjectStatistics {
        files: vec![],
        total_files: 0,
        total_lines: 0,
        total_code: 0,
        total_comment: 0,
        total_blank: 0,
        by_language: None,
        by_directory: Some(vec![DirectoryStats {
            directory: "src/<test>".to_string(),
            files: 1,
            total_lines: 100,
            code: 80,
            comment: 15,
            blank: 5,
        }]),
        top_files: None,
        average_code_lines: None,
        trend: None,
    };

    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Special characters should be escaped
    assert!(output.contains("src/&lt;test&gt;"));
}

// ============================================================================
// CSS Classes and Structure
// ============================================================================

#[test]
fn format_includes_css_variables() {
    let stats = ProjectStatistics::new(sample_file_statistics());
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // CSS variables from template
    assert!(output.contains("--color-passed"));
    assert!(output.contains("--color-warning"));
    assert!(output.contains("--color-failed"));
}

#[test]
fn format_includes_number_class_for_alignment() {
    let stats = ProjectStatistics::new(sample_file_statistics()).with_language_breakdown();
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Number cells should have class for right-alignment
    assert!(output.contains(r#"class="number""#));
}

#[test]
fn format_includes_file_path_class() {
    let stats = ProjectStatistics::new(sample_file_statistics())
        .with_directory_breakdown()
        .with_top_files(1);
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // File path cells should have monospace styling class
    assert!(output.contains(r#"class="file-path""#));
}

#[test]
fn format_trend_delta_css_classes() {
    // Create trend with mixed positive and negative deltas
    let trend = TrendDelta {
        files_delta: 5,   // positive → delta-increase
        lines_delta: -10, // negative → delta-decrease
        code_delta: 0,    // zero → no class
        comment_delta: 3, // positive → delta-increase
        blank_delta: -2,  // negative → delta-decrease
        previous_timestamp: None,
        previous_git_ref: None,
        previous_git_branch: None,
    };

    let stats = ProjectStatistics::new(sample_file_statistics()).with_trend(trend);
    let formatter = StatsHtmlFormatter::new();
    let output = formatter.format(&stats).unwrap();

    // Verify CSS classes are present for styling positive/negative deltas
    assert!(output.contains("delta-increase"));
    assert!(output.contains("delta-decrease"));

    // Verify positive values have delta-increase class
    // Files delta (+5) should be in a delta-increase card
    assert!(output.contains(r#"class="summary-card delta-increase">"#));

    // Verify negative values have delta-decrease class
    // Lines delta (-10) should be in a delta-decrease card
    assert!(output.contains(r#"class="summary-card delta-decrease">"#));
}

// ============================================================================
// Default Implementation
// ============================================================================

#[test]
fn default_is_new() {
    let default_formatter = StatsHtmlFormatter::default();
    let new_formatter = StatsHtmlFormatter::new();

    // Both should produce same output for same stats
    let stats = ProjectStatistics::new(sample_file_statistics());
    let default_output = default_formatter.format(&stats).unwrap();
    let new_output = new_formatter.format(&stats).unwrap();

    assert_eq!(default_output, new_output);
}
