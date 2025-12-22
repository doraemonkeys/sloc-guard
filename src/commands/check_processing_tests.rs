use std::path::PathBuf;
use std::sync::Mutex;

use crate::cache::Cache;
use crate::checker::ThresholdChecker;
use crate::config::Config;
use crate::counter::LineStats;
use crate::language::LanguageRegistry;

use super::{compute_effective_stats, process_file_for_check};
use crate::commands::context::RealFileReader;

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
fn process_file_nonexistent_returns_none() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;
    let path = PathBuf::from("nonexistent_file.rs");

    let result = process_file_for_check(&path, &registry, &checker, &cache, &reader);
    assert!(result.is_none());
}

#[test]
fn process_file_unknown_extension_returns_none() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;
    let path = PathBuf::from("Cargo.toml");

    let result = process_file_for_check(&path, &registry, &checker, &cache, &reader);
    assert!(result.is_none());
}

#[test]
fn process_file_valid_rust_file() {
    let registry = LanguageRegistry::default();
    let config = Config::default();
    let checker = ThresholdChecker::new(config);
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;
    let path = PathBuf::from("src/lib.rs");

    let result = process_file_for_check(&path, &registry, &checker, &cache, &reader);
    assert!(result.is_some());
    let (check_result, file_stats) = result.unwrap();
    assert!(check_result.is_passed());
    assert_eq!(file_stats.path, path);
    assert_eq!(file_stats.language, "Rust");
}
