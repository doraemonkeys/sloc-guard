use std::path::PathBuf;

use super::*;
use crate::checker::{CheckResult, CheckStatus};
use crate::counter::LineStats;
use crate::language::LanguageRegistry;

fn make_result(path: &str, code: usize, limit: usize, status: CheckStatus) -> CheckResult {
    CheckResult {
        path: PathBuf::from(path),
        status,
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
    }
}

#[test]
fn generate_split_suggestions_skips_passed() {
    let registry = LanguageRegistry::default();
    let mut results = vec![make_result("test.rs", 100, 500, CheckStatus::Passed)];

    generate_split_suggestions(&mut results, &registry);

    assert!(results[0].suggestions.is_none());
}

#[test]
fn generate_split_suggestions_skips_unknown_extension() {
    let registry = LanguageRegistry::default();
    let mut results = vec![make_result("test.xyz", 600, 500, CheckStatus::Failed)];

    generate_split_suggestions(&mut results, &registry);

    assert!(results[0].suggestions.is_none());
}

#[test]
fn generate_split_suggestions_skips_missing_file() {
    let registry = LanguageRegistry::default();
    let mut results = vec![make_result(
        "nonexistent_file.rs",
        600,
        500,
        CheckStatus::Failed,
    )];

    generate_split_suggestions(&mut results, &registry);

    assert!(results[0].suggestions.is_none());
}
