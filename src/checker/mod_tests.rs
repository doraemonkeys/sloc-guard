use std::path::Path;

use super::*;

#[test]
fn check_result_is_passed() {
    let result = CheckResult {
        path: Path::new("test.rs").to_path_buf(),
        status: CheckStatus::Passed,
        stats: LineStats::default(),
        limit: 500,
        override_reason: None,
        suggestions: None,
    };
    assert!(matches!(result.status, CheckStatus::Passed));
}

#[test]
fn check_result_is_failed() {
    let result = CheckResult {
        path: Path::new("test.rs").to_path_buf(),
        status: CheckStatus::Failed,
        stats: LineStats {
            total: 600,
            code: 550,
            comment: 30,
            blank: 20,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
        suggestions: None,
    };
    assert!(matches!(result.status, CheckStatus::Failed));
}
