use std::path::Path;

use super::*;

#[test]
fn check_result_is_passed() {
    let result = CheckResult::Passed {
        path: Path::new("test.rs").to_path_buf(),
        stats: LineStats::default(),
        raw_stats: None,
        limit: 500,
        override_reason: None,
        violation_category: None,
    };
    assert!(result.is_passed());
}

#[test]
fn check_result_is_failed() {
    let result = CheckResult::Failed {
        path: Path::new("test.rs").to_path_buf(),
        stats: LineStats {
            total: 600,
            code: 550,
            comment: 30,
            blank: 20,
            ignored: 0,
        },
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    };
    assert!(result.is_failed());
}
