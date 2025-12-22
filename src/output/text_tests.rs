use std::path::PathBuf;

use super::*;
use crate::counter::LineStats;

fn make_passed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Passed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit,
        override_reason: None,
        violation_category: None,
    }
}

fn make_warning_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Warning {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

fn make_failed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Failed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

fn make_grandfathered_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Grandfathered {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit,
        override_reason: None,
        violation_category: None,
    }
}

#[test]
fn format_passed_result() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_passed_result("test.rs", 100, 500)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("1 passed"));
    assert!(output.contains("test.rs") || output.contains("Summary"));
}

#[test]
fn format_failed_result() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("FAILED"));
    assert!(output.contains("600"));
    assert!(output.contains("limit: 500"));
}

#[test]
fn format_warning_result() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_warning_result("test.rs", 460, 500)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("WARNING"));
}

#[test]
fn format_mixed_results() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![
        make_passed_result("passed.rs", 100, 500),
        make_warning_result("warning.rs", 460, 500),
        make_failed_result("failed.rs", 600, 500),
    ];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("1 passed"));
    assert!(output.contains("1 warnings"));
    assert!(output.contains("1 failed"));
}

#[test]
fn failed_results_shown_first() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![
        make_passed_result("passed.rs", 100, 500),
        make_failed_result("failed.rs", 600, 500),
    ];

    let output = formatter.format(&results).unwrap();
    let failed_pos = output.find("failed.rs").unwrap();
    let summary_pos = output.find("Summary").unwrap();

    assert!(failed_pos < summary_pos);
}

#[test]
fn summary_line_included() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_passed_result("test.rs", 100, 500)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Summary:"));
    assert!(output.contains("1 files checked"));
}

#[test]
fn color_mode_always_produces_colored_output() {
    let formatter = TextFormatter::new(ColorMode::Always);
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    // ANSI escape codes start with \x1b[
    assert!(
        output.contains("\x1b["),
        "Output should contain ANSI color codes"
    );
}

#[test]
fn color_mode_never_produces_plain_output() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    // Should not contain ANSI escape codes
    assert!(
        !output.contains("\x1b["),
        "Output should not contain ANSI color codes"
    );
}

#[test]
fn verbose_zero_hides_passed_files() {
    let formatter = TextFormatter::with_verbose(ColorMode::Never, 0);
    let results = vec![make_passed_result("passed.rs", 100, 500)];

    let output = formatter.format(&results).unwrap();

    // Passed file details should not appear (only in summary)
    assert!(!output.contains("PASSED: passed.rs"));
    assert!(output.contains("1 passed"));
}

#[test]
fn verbose_one_shows_passed_files() {
    let formatter = TextFormatter::with_verbose(ColorMode::Never, 1);
    let results = vec![make_passed_result("passed.rs", 100, 500)];

    let output = formatter.format(&results).unwrap();

    // Passed file details should appear
    assert!(output.contains("PASSED"));
    assert!(output.contains("passed.rs"));
    assert!(output.contains("Lines: 100"));
}

#[test]
fn default_formatter_hides_passed_files() {
    let formatter = TextFormatter::default();
    let results = vec![make_passed_result("passed.rs", 100, 500)];

    let output = formatter.format(&results).unwrap();

    // Default formatter should hide passed file details (verbose = 0)
    assert!(!output.contains("PASSED: passed.rs"));
    // Check summary contains "passed" count (color codes may be inserted between number and text)
    assert!(output.contains("passed,") || output.contains("passed\n"));
    assert!(output.contains("1 files checked"));
}

#[test]
fn color_mode_auto_produces_output() {
    // In CI/test environment, stdout is typically not a TTY, so Auto mode should produce plain output
    let formatter = TextFormatter::new(ColorMode::Auto);
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    // Should produce valid output regardless of color detection
    assert!(output.contains("FAILED"));
    assert!(output.contains("test.rs"));
}

#[test]
fn override_reason_shown_in_output() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![CheckResult::Warning {
        path: PathBuf::from("legacy.rs"),
        stats: LineStats {
            total: 760,
            code: 750,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit: 800,
        override_reason: Some("Legacy file from migration".to_string()),
        suggestions: None,
        violation_category: None,
    }];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Reason: Legacy file from migration"));
}

#[test]
fn no_reason_line_when_override_reason_is_none() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("Reason:"));
}

#[test]
fn with_suggestions_shows_split_suggestions() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let result = make_failed_result("big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string(), "func2".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
            SplitChunk {
                suggested_name: "big_file_part2".to_string(),
                functions: vec!["func3".to_string()],
                start_line: 301,
                end_line: 600,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = TextFormatter::new(ColorMode::Never).with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();

    assert!(output.contains("Split suggestions:"));
    assert!(output.contains("big_file_part1"));
    assert!(output.contains("big_file_part2"));
    assert!(output.contains("func1, func2"));
}

#[test]
fn without_suggestions_flag_hides_split_suggestions() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let result = make_failed_result("big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = TextFormatter::new(ColorMode::Never).with_suggestions(false);
    let output = formatter.format(&[result]).unwrap();

    assert!(!output.contains("Split suggestions:"));
}

#[test]
fn grandfathered_status_shown_in_summary() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![make_grandfathered_result("legacy.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("grandfathered"));
    assert!(output.contains("baseline:"));
}

#[test]
fn verbose_one_shows_grandfathered_files() {
    let formatter = TextFormatter::with_verbose(ColorMode::Never, 1);
    let results = vec![make_grandfathered_result("legacy.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("GRANDFATHERED"));
    assert!(output.contains("legacy.rs"));
}

#[test]
fn verbose_zero_hides_grandfathered_files() {
    let formatter = TextFormatter::with_verbose(ColorMode::Never, 0);
    let results = vec![make_grandfathered_result("legacy.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    // Grandfathered file details should not appear (only in summary)
    assert!(!output.contains("GRANDFATHERED: legacy.rs"));
    assert!(output.contains("grandfathered"));
}

#[test]
fn grandfathered_colored_output() {
    let formatter = TextFormatter::new(ColorMode::Always);
    let results = vec![make_grandfathered_result("legacy.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();

    // Should contain ANSI color codes (cyan for grandfathered)
    assert!(output.contains("\x1b["));
}

#[test]
fn format_structure_file_count_violation() {
    use crate::checker::{ViolationCategory, ViolationType};
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![CheckResult::Failed {
        path: PathBuf::from("."),
        stats: LineStats {
            total: 10,
            code: 10,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        limit: 5,
        override_reason: Some("structure: files count exceeded".to_string()),
        suggestions: None,
        violation_category: Some(ViolationCategory::Structure {
            violation_type: ViolationType::FileCount,
            triggering_rule: None,
        }),
    }];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Files: 10 (limit: 5)"));
    assert!(!output.contains("Lines:"));
    assert!(!output.contains("Breakdown:"));
}

#[test]
fn format_structure_dir_count_violation() {
    use crate::checker::{ViolationCategory, ViolationType};
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = vec![CheckResult::Failed {
        path: PathBuf::from("."),
        stats: LineStats {
            total: 25,
            code: 25,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        limit: 20,
        override_reason: Some("structure: subdirs count exceeded".to_string()),
        suggestions: None,
        violation_category: Some(ViolationCategory::Structure {
            violation_type: ViolationType::DirCount,
            triggering_rule: None,
        }),
    }];

    let output = formatter.format(&results).unwrap();

    assert!(output.contains("Directories: 25 (limit: 20)"));
    assert!(!output.contains("Lines:"));
    assert!(!output.contains("Breakdown:"));
}
