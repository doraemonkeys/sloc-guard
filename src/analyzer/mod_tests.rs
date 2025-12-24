use std::path::PathBuf;

use super::*;
use crate::checker::CheckResult;
use crate::counter::LineStats;
use crate::language::LanguageRegistry;

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
        raw_stats: None,
        limit,
        override_reason: None,
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
        raw_stats: None,
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

#[test]
fn generate_split_suggestions_skips_passed() {
    let registry = LanguageRegistry::default();
    let mut results = vec![make_passed_result("test.rs", 100, 500)];

    generate_split_suggestions(&mut results, &registry);

    assert!(results[0].suggestions().is_none());
}

#[test]
fn generate_split_suggestions_skips_unknown_extension() {
    let registry = LanguageRegistry::default();
    let mut results = vec![make_failed_result("test.xyz", 600, 500)];

    generate_split_suggestions(&mut results, &registry);

    assert!(results[0].suggestions().is_none());
}

#[test]
fn generate_split_suggestions_skips_missing_file() {
    let registry = LanguageRegistry::default();
    let mut results = vec![make_failed_result("nonexistent_file.rs", 600, 500)];

    generate_split_suggestions(&mut results, &registry);

    assert!(results[0].suggestions().is_none());
}
