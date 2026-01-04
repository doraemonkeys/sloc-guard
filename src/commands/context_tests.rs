use crate::cli::ColorChoice;
use crate::config::{Config, FetchPolicy, LoadResult, StructureConfig, StructureRule};
use crate::output::ColorMode;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED, Result, SlocGuardError};
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

use super::*;
use crate::cache::Cache;
use crate::language::LanguageRegistry;

// =============================================================================
// Test Helpers
// =============================================================================

/// Helper to load config from inline content without manual `TempDir` boilerplate.
///
/// Returns the `TempDir` guard alongside the result to ensure the temp directory
/// lives long enough for the test assertions.
fn load_config_from_content(content: &str) -> (TempDir, Result<LoadResult>) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");
    std::fs::write(&config_path, content).unwrap();
    let result = load_config(Some(&config_path), false, false, FetchPolicy::Normal);
    (temp_dir, result)
}

/// Asserts that the result is a `SlocGuardError::Config` containing the expected substring.
fn assert_config_error_contains(result: Result<LoadResult>, expected_substring: &str) {
    match result {
        Err(SlocGuardError::Config(msg)) => {
            assert!(
                msg.contains(expected_substring),
                "expected error containing '{expected_substring}', got: {msg}"
            );
        }
        Err(other) => panic!(
            "expected SlocGuardError::Config containing '{expected_substring}', got: {other}"
        ),
        Ok(_) => {
            panic!("expected SlocGuardError::Config containing '{expected_substring}', but got Ok")
        }
    }
}

#[test]
fn exit_codes_documented() {
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(EXIT_THRESHOLD_EXCEEDED, 1);
    assert_eq!(EXIT_CONFIG_ERROR, 2);
}

#[test]
fn load_config_no_config_returns_default() {
    let result = load_config(None, true, false, FetchPolicy::Normal).unwrap();
    assert_eq!(result.config.content.max_lines, 600);
    assert!(result.preset_used.is_none());
}

#[test]
fn load_config_with_nonexistent_path_returns_error() {
    let result = load_config(
        Some(std::path::Path::new("nonexistent.toml")),
        false,
        false,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
}

#[test]
fn load_config_without_no_config_searches_defaults() {
    // Use temp dir with a known config to ensure test isolation
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let content = r#"
version = "2"
[content]
max_lines = 999
"#;
    std::fs::write(&config_path, content).unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let result = load_config(None, false, false, FetchPolicy::Normal).unwrap();
    assert_eq!(result.config.content.max_lines, 999);

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn write_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.txt");

    let result = write_output(Some(&output_path), "test content", false);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn write_output_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    // Use nested directories that don't exist yet
    let output_path = temp_dir.path().join("nested/deep/dir/output.txt");

    // Parent directory should not exist yet
    assert!(!output_path.parent().unwrap().exists());

    let result = write_output(Some(&output_path), "test content", false);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn write_output_quiet_mode() {
    let result = write_output(None, "test content", true);
    assert!(result.is_ok());
}

#[test]
fn write_output_normal_mode() {
    let result = write_output(None, "", false);
    assert!(result.is_ok());
}

#[test]
fn check_config_from_file() {
    let content = r#"
version = "2"

[check]
warnings_as_errors = true
fail_fast = true
"#;

    let config: Config = toml::from_str(content).unwrap();
    assert!(config.check.warnings_as_errors);
    assert!(config.check.fail_fast);
}

#[test]
fn check_config_default_false() {
    let config = Config::default();
    assert!(!config.check.warnings_as_errors);
    assert!(!config.check.fail_fast);
}

#[test]
fn load_config_with_no_extends_returns_config_without_merging() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("child.toml");
    let content = r#"
extends = "https://example.com/base.toml"

[content]
max_lines = 200
"#;
    std::fs::write(&config_path, content).unwrap();

    let result = load_config(Some(&config_path), false, true, FetchPolicy::Normal).unwrap();
    assert_eq!(result.config.content.max_lines, 200);
    assert_eq!(
        result.config.extends,
        Some("https://example.com/base.toml".to_string())
    );
    // preset_used is None when extends is not resolved
    assert!(result.preset_used.is_none());
}

// =============================================================================
// load_config Semantic Validation Tests
// =============================================================================

#[test]
fn load_config_rejects_invalid_content_warn_threshold_too_high() {
    let (_temp_dir, result) = load_config_from_content(
        r#"
version = "2"
[content]
warn_threshold = 1.5
"#,
    );
    assert_config_error_contains(result, "warn_threshold");
}

#[test]
fn load_config_rejects_invalid_content_warn_threshold_negative() {
    let (_temp_dir, result) = load_config_from_content(
        r#"
version = "2"
[content]
warn_threshold = -0.1
"#,
    );
    assert_config_error_contains(result, "warn_threshold");
}

#[test]
fn load_config_rejects_invalid_structure_warn_threshold() {
    let (_temp_dir, result) = load_config_from_content(
        r#"
version = "2"
[structure]
warn_threshold = 2.0
"#,
    );
    assert_config_error_contains(result, "structure.warn_threshold");
}

#[test]
fn load_config_rejects_warn_at_exceeds_max_lines() {
    let (_temp_dir, result) = load_config_from_content(
        r#"
version = "2"
[content]
max_lines = 500
warn_at = 600
"#,
    );
    assert_config_error_contains(result, "warn_at");
}

#[test]
fn load_config_rejects_invalid_glob_pattern() {
    let (_temp_dir, result) = load_config_from_content(
        r#"
version = "2"
[scanner]
exclude = ["[invalid"]
"#,
    );
    // Glob pattern errors use SlocGuardError::InvalidPattern variant
    match result {
        Err(SlocGuardError::InvalidPattern { pattern, .. }) => {
            assert!(
                pattern.contains("[invalid"),
                "expected pattern '[invalid', got: {pattern}"
            );
        }
        Err(other) => panic!("expected SlocGuardError::InvalidPattern, got: {other}"),
        Ok(_) => panic!("expected SlocGuardError::InvalidPattern, but got Ok"),
    }
}

#[test]
fn load_config_accepts_valid_config() {
    let (_temp_dir, result) = load_config_from_content(
        r#"
version = "2"
[content]
max_lines = 500
warn_threshold = 0.8
warn_at = 400
[structure]
warn_threshold = 0.9
"#,
    );
    assert!(result.is_ok(), "valid config should be accepted");
    let config = result.unwrap().config;
    assert_eq!(config.content.max_lines, 500);
    assert!((config.content.warn_threshold - 0.8).abs() < f64::EPSILON);
}

/// Validates that semantic errors from parent configs are caught after extends resolution.
///
/// The "two-phase" design:
/// 1. Config loading resolves extends chain and merges values
/// 2. Semantic validation runs on the final merged config
///
/// This test confirms that invalid values inherited from a parent config
/// are properly rejected during semantic validation.
#[test]
fn load_config_rejects_invalid_threshold_from_extended_parent() {
    let temp_dir = TempDir::new().unwrap();

    // Parent config with invalid warn_threshold (out of 0.0-1.0 range)
    let parent_path = temp_dir.path().join("parent.toml");
    std::fs::write(
        &parent_path,
        r#"
version = "2"
[content]
warn_threshold = 1.5
"#,
    )
    .unwrap();

    // Child extends parent using relative path (inherits invalid warn_threshold)
    let child_path = temp_dir.path().join("child.toml");
    std::fs::write(
        &child_path,
        r#"
version = "2"
extends = "parent.toml"
[content]
max_lines = 300
"#,
    )
    .unwrap();

    // Load child config - should fail due to inherited invalid warn_threshold
    let result = load_config(Some(&child_path), false, false, FetchPolicy::Normal);
    assert_config_error_contains(result, "warn_threshold");
}

#[test]
fn resolve_scan_paths_uses_include_override() {
    let paths = vec![PathBuf::from(".")];
    let include = vec!["src".to_string(), "lib".to_string()];

    let result = resolve_scan_paths(&paths, &include);
    assert_eq!(result, vec![PathBuf::from("src"), PathBuf::from("lib")]);
}

#[test]
fn resolve_scan_paths_uses_cli_paths() {
    let paths = vec![PathBuf::from("src"), PathBuf::from("tests")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include);
    assert_eq!(result, vec![PathBuf::from("src"), PathBuf::from("tests")]);
}

#[test]
fn resolve_scan_paths_defaults_to_current_dir() {
    let paths = vec![PathBuf::from(".")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include);
    assert_eq!(result, vec![PathBuf::from(".")]);
}

#[test]
fn color_choice_to_mode_auto() {
    assert_eq!(color_choice_to_mode(ColorChoice::Auto), ColorMode::Auto);
}

#[test]
fn color_choice_to_mode_always() {
    assert_eq!(color_choice_to_mode(ColorChoice::Always), ColorMode::Always);
}

#[test]
fn color_choice_to_mode_never() {
    assert_eq!(color_choice_to_mode(ColorChoice::Never), ColorMode::Never);
}

// =============================================================================
// FileReader Tests
// =============================================================================

#[test]
fn real_file_reader_reads_file_contents() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"hello world").unwrap();

    let reader = RealFileReader;
    let content = reader.read(&file_path).unwrap();

    assert_eq!(content, b"hello world");
}

#[test]
fn real_file_reader_returns_error_for_nonexistent_file() {
    let reader = RealFileReader;
    let result = reader.read(std::path::Path::new("nonexistent_file.txt"));

    assert!(result.is_err());
}

#[test]
fn real_file_reader_metadata_returns_size() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"hello").unwrap();

    let reader = RealFileReader;
    let (_, size) = reader.metadata(&file_path).unwrap();

    assert_eq!(size, 5); // "hello" is 5 bytes
}

#[test]
fn read_file_with_hash_returns_content_and_hash() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"test content").unwrap();

    let reader = RealFileReader;
    let result = read_file_with_hash(&reader, &file_path);

    assert!(result.is_some());
    let (hash, content) = result.unwrap();
    assert_eq!(content, b"test content");
    assert!(!hash.is_empty());
}

#[test]
fn read_file_with_hash_returns_none_for_nonexistent() {
    let reader = RealFileReader;
    let result = read_file_with_hash(&reader, std::path::Path::new("nonexistent.txt"));

    assert!(result.is_none());
}

// =============================================================================
// Error Propagation Tests (Phase 23.2)
// =============================================================================

#[test]
fn check_context_from_config_propagates_invalid_structure_pattern() {
    // Invalid glob pattern in structure rules should propagate error
    let config = Config {
        structure: StructureConfig {
            max_files: Some(10),
            rules: vec![StructureRule {
                scope: "[invalid".to_string(), // Unclosed bracket is invalid glob
                max_files: Some(20),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = CheckContext::from_config(&config, 0.9, Vec::new(), false);

    match result {
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("Invalid glob pattern"),
                "Expected 'Invalid glob pattern' in: {msg}"
            );
            assert!(msg.contains("[invalid"), "Expected '[invalid' in: {msg}");
        }
        Ok(_) => panic!("Expected error for invalid structure pattern, but got Ok"),
    }
}

// =============================================================================
// resolve_project_root Tests (Phase 23.4)
// =============================================================================

#[test]
fn resolve_project_root_discovers_project_root() {
    // resolve_project_root uses state::discover_project_root to find the project root.
    // This ensures consistent state file locations (cache, history, remote config cache).

    let project_root = resolve_project_root();

    // Should discover the actual project root (walks up to find .git or .sloc-guard.toml)
    // In the test environment, this will find the workspace root with .git
    assert!(project_root.exists());
}

// =============================================================================
// FileProcessResult Tests
// =============================================================================

#[test]
fn file_skip_reason_display_no_extension() {
    let reason = FileSkipReason::NoExtension;
    assert_eq!(reason.to_string(), "file has no extension");
}

#[test]
fn file_skip_reason_display_unrecognized_extension() {
    let reason = FileSkipReason::UnrecognizedExtension("xyz".to_string());
    assert_eq!(reason.to_string(), "unrecognized extension: .xyz");
}

#[test]
fn file_skip_reason_display_ignored_by_directive() {
    let reason = FileSkipReason::IgnoredByDirective;
    assert_eq!(reason.to_string(), "ignored by sloc-guard directive");
}

#[test]
fn file_process_error_display_metadata() {
    let error = FileProcessError::MetadataError {
        path: PathBuf::from("test.rs"),
        source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
    };
    let msg = error.to_string();
    assert!(msg.contains("test.rs"), "should contain path: {msg}");
    assert!(msg.contains("metadata"), "should mention metadata: {msg}");
}

#[test]
fn file_process_error_display_cache_lock() {
    let error = FileProcessError::CacheLockError {
        path: PathBuf::from("test.rs"),
    };
    let msg = error.to_string();
    assert!(msg.contains("test.rs"), "should contain path: {msg}");
    assert!(msg.contains("lock"), "should mention lock: {msg}");
}

#[test]
fn file_process_error_display_read() {
    let error = FileProcessError::ReadError {
        path: PathBuf::from("test.rs"),
        source: io::Error::new(io::ErrorKind::PermissionDenied, "access denied"),
    };
    let msg = error.to_string();
    assert!(msg.contains("test.rs"), "should contain path: {msg}");
    assert!(msg.contains("read"), "should mention read: {msg}");
}

#[test]
fn file_process_error_path_accessor() {
    let error = FileProcessError::MetadataError {
        path: PathBuf::from("some/path.rs"),
        source: io::Error::new(io::ErrorKind::NotFound, "not found"),
    };
    assert_eq!(error.path(), std::path::Path::new("some/path.rs"));
}

#[test]
fn file_process_error_source_metadata() {
    use std::error::Error;
    let error = FileProcessError::MetadataError {
        path: PathBuf::from("test.rs"),
        source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
    };
    let source = error.source();
    assert!(source.is_some(), "MetadataError should have a source");
    assert!(
        source.unwrap().to_string().contains("file not found"),
        "source should contain the original error message"
    );
}

#[test]
fn file_process_error_source_read() {
    use std::error::Error;
    let error = FileProcessError::ReadError {
        path: PathBuf::from("test.rs"),
        source: io::Error::new(io::ErrorKind::PermissionDenied, "access denied"),
    };
    let source = error.source();
    assert!(source.is_some(), "ReadError should have a source");
    assert!(
        source.unwrap().to_string().contains("access denied"),
        "source should contain the original error message"
    );
}

#[test]
fn file_process_error_source_cache_lock() {
    use std::error::Error;
    let error = FileProcessError::CacheLockError {
        path: PathBuf::from("test.rs"),
    };
    assert!(
        error.source().is_none(),
        "CacheLockError should have no source"
    );
}

#[test]
fn process_file_with_cache_returns_skipped_for_no_extension() {
    let registry = LanguageRegistry::default();
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;

    // Create a temp file without extension
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("noext");
    std::fs::write(&file_path, "content").unwrap();

    let result = process_file_with_cache(&file_path, &registry, &cache, &reader);
    assert!(
        matches!(
            result,
            FileProcessResult::Skipped(FileSkipReason::NoExtension)
        ),
        "expected Skipped(NoExtension), got {result:?}"
    );
}

#[test]
fn process_file_with_cache_returns_skipped_for_unrecognized_extension() {
    let registry = LanguageRegistry::default();
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;

    // Create a temp file with unrecognized extension
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.xyz123");
    std::fs::write(&file_path, "content").unwrap();

    let result = process_file_with_cache(&file_path, &registry, &cache, &reader);
    assert!(
        matches!(
            result,
            FileProcessResult::Skipped(FileSkipReason::UnrecognizedExtension(_))
        ),
        "expected Skipped(UnrecognizedExtension), got {result:?}"
    );
}

#[test]
fn process_file_with_cache_returns_error_for_nonexistent_file() {
    let registry = LanguageRegistry::default();
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;

    let result = process_file_with_cache(
        std::path::Path::new("nonexistent_file.rs"),
        &registry,
        &cache,
        &reader,
    );
    assert!(
        matches!(
            result,
            FileProcessResult::Error(FileProcessError::MetadataError { .. })
        ),
        "expected Error(MetadataError), got {result:?}"
    );
}

#[test]
fn process_file_with_cache_returns_success_for_valid_file() {
    let registry = LanguageRegistry::default();
    let cache = Mutex::new(Cache::new(String::new()));
    let reader = RealFileReader;

    // Use an existing rust file from the project
    let file_path = std::path::Path::new("src/lib.rs");
    let result = process_file_with_cache(file_path, &registry, &cache, &reader);
    match result {
        FileProcessResult::Success { stats, language } => {
            assert!(stats.code > 0, "should have some code lines");
            assert_eq!(language, "Rust");
        }
        other => panic!("expected Success, got {other:?}"),
    }
}
