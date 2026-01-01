use std::path::PathBuf;
use std::sync::Mutex;

use crate::cache::Cache;
use crate::checker::{CheckResult, ThresholdChecker};
use crate::config::Config;
use crate::counter::LineStats;
use crate::language::LanguageRegistry;
use crate::output::FileStatistics;

use super::{CheckFileResult, compute_effective_stats, process_file_for_check};
use crate::commands::context::{FileSkipReason, RealFileReader};

/// Asserts that boxing the Success variant data is worthwhile.
/// The combined size of `CheckResult` + `FileStatistics` justifies the Box allocation
/// to keep `CheckFileResult` enum small. This test catches regressions if these types grow.
#[test]
fn check_result_and_file_stats_size_justifies_boxing() {
    let check_result_size = std::mem::size_of::<CheckResult>();
    let file_stats_size = std::mem::size_of::<FileStatistics>();
    let combined_size = check_result_size + file_stats_size;

    // Boxing is justified when combined size exceeds typical cache line (64 bytes).
    // Current sizes: CheckResult ~400+ bytes, FileStatistics ~50+ bytes.
    // If this assertion fails, re-evaluate whether boxing is still worthwhile.
    assert!(
        combined_size > 64,
        "Combined size ({combined_size} bytes) is small enough that boxing may be unnecessary. \
         Consider removing Box if performance profiling supports it."
    );
}

#[test]
fn compute_effective_stats_skip_both() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, true, true);
    assert_eq!(effective.code, 80);
    assert_eq!(effective.comment, 15);
    assert_eq!(effective.blank, 5);
}

#[test]
fn compute_effective_stats_include_comments() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, false, true);
    assert_eq!(effective.code, 95);
    assert_eq!(effective.comment, 0);
    assert_eq!(effective.blank, 5);
}

#[test]
fn compute_effective_stats_include_blanks() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, true, false);
    assert_eq!(effective.code, 85);
    assert_eq!(effective.comment, 15);
    assert_eq!(effective.blank, 0);
}

#[test]
fn compute_effective_stats_include_both() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    let effective = compute_effective_stats(&stats, false, false);
    assert_eq!(effective.code, 100);
    assert_eq!(effective.comment, 0);
    assert_eq!(effective.blank, 0);
}

#[test]
fn process_file_nonexistent_returns_error() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;
    let path = PathBuf::from("nonexistent_file.rs");

    let result = process_file_for_check(&path, &registry, &checker, &cache, &reader);
    // Nonexistent file should return an error (not skipped, because extension is recognized)
    assert!(
        matches!(result, CheckFileResult::Error(_)),
        "expected Error, got {result:?}"
    );
}

#[test]
fn process_file_unknown_extension_returns_skipped() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;
    let path = PathBuf::from("Cargo.toml");

    let result = process_file_for_check(&path, &registry, &checker, &cache, &reader);
    // Unknown extension should be skipped (not an error)
    assert!(
        matches!(
            result,
            CheckFileResult::Skipped(FileSkipReason::UnrecognizedExtension(_))
        ),
        "expected Skipped(UnrecognizedExtension), got {result:?}"
    );
}

#[test]
fn process_file_valid_rust_file() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;
    let path = PathBuf::from("src/lib.rs");

    let result = process_file_for_check(&path, &registry, &checker, &cache, &reader);
    match result {
        CheckFileResult::Success {
            check_result,
            file_stats,
        } => {
            // check_result is boxed to reduce enum size
            assert!(check_result.is_passed());
            assert_eq!(file_stats.path, path);
            assert_eq!(file_stats.language, "Rust");
        }
        other => panic!("expected Success, got {other:?}"),
    }
}
