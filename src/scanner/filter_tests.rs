use std::path::Path;

use super::*;

#[test]
fn filter_by_extension() {
    let filter = GlobFilter::new(vec!["rs".to_string()], &[]).unwrap();

    assert!(filter.should_include(Path::new("src/main.rs")));
    assert!(!filter.should_include(Path::new("src/main.py")));
}

#[test]
fn filter_multiple_extensions() {
    let filter = GlobFilter::new(vec!["rs".to_string(), "go".to_string()], &[]).unwrap();

    assert!(filter.should_include(Path::new("main.rs")));
    assert!(filter.should_include(Path::new("main.go")));
    assert!(!filter.should_include(Path::new("main.py")));
}

#[test]
fn filter_empty_extensions_accepts_all() {
    let filter = GlobFilter::new(vec![], &[]).unwrap();

    assert!(filter.should_include(Path::new("main.rs")));
    assert!(filter.should_include(Path::new("main.py")));
    assert!(filter.should_include(Path::new("readme.txt")));
}

#[test]
fn filter_exclude_patterns() {
    let filter = GlobFilter::new(
        vec!["rs".to_string()],
        &["**/target/**".to_string(), "**/generated/**".to_string()],
    )
    .unwrap();

    assert!(filter.should_include(Path::new("src/main.rs")));
    assert!(!filter.should_include(Path::new("target/debug/main.rs")));
    assert!(!filter.should_include(Path::new("src/generated/code.rs")));
}

#[test]
fn filter_exclude_specific_files() {
    let filter =
        GlobFilter::new(vec!["rs".to_string()], &["**/*.generated.rs".to_string()]).unwrap();

    assert!(filter.should_include(Path::new("src/main.rs")));
    assert!(!filter.should_include(Path::new("src/code.generated.rs")));
}

#[test]
fn filter_invalid_pattern_returns_error() {
    let result = GlobFilter::new(vec![], &["[invalid".to_string()]);
    assert!(result.is_err());
}

#[test]
fn filter_complex_exclude_patterns() {
    let filter = GlobFilter::new(
        vec!["rs".to_string()],
        &[
            "**/target/**".to_string(),
            "**/node_modules/**".to_string(),
            "**/.git/**".to_string(),
        ],
    )
    .unwrap();

    assert!(filter.should_include(Path::new("src/lib.rs")));
    assert!(!filter.should_include(Path::new("target/release/build/main.rs")));
    assert!(!filter.should_include(Path::new(".git/hooks/pre-commit.rs")));
}

#[test]
fn filter_file_without_extension_accepted_when_empty_extensions() {
    let filter = GlobFilter::new(vec![], &[]).unwrap();

    assert!(filter.should_include(Path::new("Makefile")));
    assert!(filter.should_include(Path::new("Dockerfile")));
    assert!(filter.should_include(Path::new(".gitignore")));
}

#[test]
fn filter_file_without_extension_rejected_when_extensions_set() {
    let filter = GlobFilter::new(vec!["rs".to_string()], &[]).unwrap();

    assert!(!filter.should_include(Path::new("Makefile")));
    assert!(!filter.should_include(Path::new("Dockerfile")));
}

#[test]
fn filter_exclude_by_filename() {
    let filter = GlobFilter::new(vec![], &["*.lock".to_string()]).unwrap();

    assert!(filter.should_include(Path::new("Cargo.toml")));
    assert!(!filter.should_include(Path::new("Cargo.lock")));
}
