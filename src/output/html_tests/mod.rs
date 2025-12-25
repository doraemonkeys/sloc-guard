mod document;
mod escape;
mod file_table;
mod histogram;
mod language_breakdown;
mod suggestions;
mod summary;
mod trend;

use std::path::PathBuf;

use crate::checker::CheckResult;
use crate::counter::LineStats;

/// Creates a Passed check result for testing.
pub(super) fn make_passed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Passed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        violation_category: None,
    }
}

/// Creates a Warning check result for testing.
pub(super) fn make_warning_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Warning {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

/// Creates a Failed check result for testing.
pub(super) fn make_failed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Failed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

/// Creates a Grandfathered check result for testing.
pub(super) fn make_grandfathered_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Grandfathered {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10 + 5,
            code,
            comment: 10,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        violation_category: None,
    }
}
