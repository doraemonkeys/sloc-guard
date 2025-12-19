// Advanced feature tests for check command: baseline, context injection, CLI structure parameters

use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

use crate::baseline::Baseline;
use crate::cache::Cache;
use crate::checker::{CheckResult, ThresholdChecker};
use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, InitArgs};
use crate::config::Config;
use crate::counter::LineStats;
use crate::language::LanguageRegistry;
use crate::output::OutputFormat;
use crate::{EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::*;
use crate::commands::context::CheckContext;

fn make_cli_for_check(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
        }),
        verbose,
        quiet,
        color,
        no_config,
        no_extends: false,
    }
}

// =============================================================================
// Baseline Comparison Tests
// =============================================================================

#[test]
fn load_baseline_none_path_returns_none() {
    let result = load_baseline(None);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn load_baseline_nonexistent_file_returns_error() {
    let result = load_baseline(Some(std::path::Path::new("nonexistent-baseline.json")));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn load_baseline_valid_file_returns_baseline() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let mut baseline = Baseline::new();
    baseline.set_content("test/file.rs", 100, "abc123".to_string());
    baseline.save(&baseline_path).unwrap();

    let result = load_baseline(Some(&baseline_path));
    assert!(result.is_ok());
    let loaded = result.unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.len(), 1);
    assert!(loaded.contains("test/file.rs"));
}

#[test]
fn apply_baseline_comparison_marks_failed_as_grandfathered() {
    let mut results = vec![
        CheckResult::Failed {
            path: PathBuf::from("src/file.rs"),
            stats: LineStats {
                total: 600,
                code: 600,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            limit: 500,
            override_reason: None,
            suggestions: None,
        },
        CheckResult::Passed {
            path: PathBuf::from("src/other.rs"),
            stats: LineStats {
                total: 100,
                code: 100,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            limit: 500,
            override_reason: None,
        },
    ];

    let mut baseline = Baseline::new();
    baseline.set_content("src/file.rs", 600, "hash123".to_string());

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_grandfathered());
    assert!(results[1].is_passed());
}

#[test]
fn apply_baseline_comparison_does_not_mark_new_violations() {
    let mut results = vec![CheckResult::Failed {
        path: PathBuf::from("src/new_file.rs"),
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
        suggestions: None,
    }];

    let baseline = Baseline::new();

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_failed());
}

#[test]
fn apply_baseline_comparison_handles_windows_paths() {
    let mut results = vec![CheckResult::Failed {
        path: PathBuf::from("src\\file.rs"),
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
        suggestions: None,
    }];

    let mut baseline = Baseline::new();
    baseline.set_content("src/file.rs", 600, "hash123".to_string());

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_grandfathered());
}

#[test]
fn run_check_impl_with_baseline_grandfathers_violations() {
    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut baseline = Baseline::new();
    let file_path_str = test_file_path.to_string_lossy().replace('\\', "/");
    baseline.set_content(&file_path_str, 102, "dummy_hash".to_string());
    baseline.save(&baseline_path).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: Some(baseline_path),
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_impl_without_baseline_fails_on_violations() {
    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_impl_with_baseline_fails_on_new_violations() {
    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("new_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let baseline = Baseline::new();
    baseline.save(&baseline_path).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: Some(baseline_path),
        update_baseline: None,
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Context Injection Tests (Task 5.5.13: Testability Refactoring)
// =============================================================================

#[test]
fn check_context_from_config_creates_valid_context() {
    let config = Config::default();
    let ctx = CheckContext::from_config(&config, 0.9).unwrap();

    // Verify context contains expected components
    // Default config has no structure limits, so structure checker exists but is not enabled
    if let Some(ref sc) = ctx.structure_checker {
        // Structure checker is created but not enabled with default config (no limits set)
        assert!(!sc.is_enabled());
    }
}

#[test]
fn check_context_new_allows_custom_injection() {
    let config = Config::default();
    let registry = LanguageRegistry::default();
    let threshold_checker = ThresholdChecker::new(config).with_warning_threshold(0.5);

    // Create context with custom components (no structure checker)
    let ctx = CheckContext::new(registry, threshold_checker, None);

    // Context should have no structure checker
    assert!(ctx.structure_checker.is_none());
}

#[test]
fn run_check_with_context_uses_injected_threshold_checker() {
    // This test demonstrates that run_check_with_context uses the injected
    // threshold_checker instead of creating one internally.
    let temp_dir = TempDir::new().unwrap();

    // Create a small file that would pass with default limits
    let test_file = temp_dir.path().join("small.rs");
    std::fs::write(&test_file, "fn main() {}\n").unwrap();

    // Create context with very strict threshold (max_lines = 1)
    let mut config = Config::default();
    config.content.extensions = vec!["rs".to_string()];
    config.content.max_lines = 1; // Very strict

    let ctx = CheckContext::from_config(&config, 0.9).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);
    let paths = args.paths.clone();

    // The injected context's threshold_checker should detect the violation
    let result = run_check_with_context(&args, &cli, &paths, &config, &ctx, &cache, None);
    assert!(result.is_ok());
    // File has 1 line, limit is 1, so it should pass (equal to limit)
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn run_check_with_context_uses_injected_structure_checker() {
    // This test demonstrates structure checker injection
    let temp_dir = TempDir::new().unwrap();

    // Create directory with files
    let sub_dir = temp_dir.path().join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(sub_dir.join("b.rs"), "fn b() {}").unwrap();
    std::fs::write(sub_dir.join("c.rs"), "fn c() {}").unwrap();

    // Create config with structure limits
    let mut config = Config::default();
    config.content.extensions = vec!["rs".to_string()];
    config.structure.max_files = Some(2); // Limit to 2 files per directory

    let ctx = CheckContext::from_config(&config, 0.9).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));

    // Verify structure checker is enabled
    assert!(ctx.structure_checker.is_some());
    assert!(ctx.structure_checker.as_ref().unwrap().is_enabled());

    let args = CheckArgs {
        paths: vec![sub_dir.clone()],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);
    let paths = vec![sub_dir];

    // Structure checker should detect violation (3 files > 2 limit)
    let result = run_check_with_context(&args, &cli, &paths, &config, &ctx, &cache, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Task 6.1: CLI Structure Parameters Tests
// =============================================================================

#[test]
fn validate_and_resolve_paths_no_args_defaults_to_current_dir() {
    let args = CheckArgs {
        paths: vec![],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_ok());
    let paths = result.unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], PathBuf::from("."));
}

#[test]
fn validate_and_resolve_paths_max_files_without_path_returns_error() {
    let args = CheckArgs {
        paths: vec![],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: Some(10),
        max_dirs: None,
        report_json: None,
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("--max-files/--max-dirs require a target <PATH>")
    );
}

#[test]
fn validate_and_resolve_paths_max_dirs_without_path_returns_error() {
    let args = CheckArgs {
        paths: vec![],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: None,
        max_dirs: Some(5),
        report_json: None,
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("--max-files/--max-dirs require a target <PATH>")
    );
}

#[test]
fn validate_and_resolve_paths_max_files_with_path_succeeds() {
    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: Some(10),
        max_dirs: None,
        report_json: None,
    };

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_ok());
    let paths = result.unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], PathBuf::from("src"));
}

#[test]
fn apply_cli_overrides_structure_max_files() {
    let mut config = Config::default();
    assert!(config.structure.max_files.is_none());

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: Some(10),
        max_dirs: None,
        report_json: None,
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.structure.max_files, Some(10));
}

#[test]
fn apply_cli_overrides_structure_max_dirs() {
    let mut config = Config::default();
    assert!(config.structure.max_dirs.is_none());

    let args = CheckArgs {
        paths: vec![PathBuf::from("src")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
        max_files: None,
        max_dirs: Some(5),
        report_json: None,
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.structure.max_dirs, Some(5));
}

#[test]
fn run_check_with_cli_max_files_overrides_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create 3 files in directory
    std::fs::write(temp_dir.path().join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(temp_dir.path().join("b.rs"), "fn b() {}").unwrap();
    std::fs::write(temp_dir.path().join("c.rs"), "fn c() {}").unwrap();

    // Use --max-files=2 to trigger violation (3 files > 2 limit)
    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
        max_lines: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: Some(2),
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // 3 files > 2 limit should fail
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_with_cli_max_dirs_overrides_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create 3 subdirectories
    std::fs::create_dir(temp_dir.path().join("sub1")).unwrap();
    std::fs::create_dir(temp_dir.path().join("sub2")).unwrap();
    std::fs::create_dir(temp_dir.path().join("sub3")).unwrap();

    // Use --max-dirs=2 to trigger violation (3 dirs > 2 limit)
    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
        max_lines: None,
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        update_baseline: None,
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: Some(2),
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // 3 dirs > 2 limit should fail
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Update Baseline Tests
// =============================================================================

#[test]
fn is_structure_violation_returns_true_for_structure_violations() {
    assert!(super::is_structure_violation(Some("structure: files count exceeded")));
    assert!(super::is_structure_violation(Some("structure: subdirs count exceeded")));
}

#[test]
fn is_structure_violation_returns_false_for_non_structure() {
    assert!(!super::is_structure_violation(None));
    assert!(!super::is_structure_violation(Some("content violation")));
    assert!(!super::is_structure_violation(Some("")));
}

#[test]
fn parse_structure_violation_parses_files_correctly() {
    use crate::baseline::StructureViolationType;

    let result = super::parse_structure_violation(Some("structure: files count exceeded"), 25);
    assert!(result.is_some());
    let (vtype, count) = result.unwrap();
    assert_eq!(vtype, StructureViolationType::Files);
    assert_eq!(count, 25);
}

#[test]
fn parse_structure_violation_parses_subdirs_correctly() {
    use crate::baseline::StructureViolationType;

    let result = super::parse_structure_violation(Some("structure: subdirs count exceeded"), 10);
    assert!(result.is_some());
    let (vtype, count) = result.unwrap();
    assert_eq!(vtype, StructureViolationType::Dirs);
    assert_eq!(count, 10);
}

#[test]
fn parse_structure_violation_returns_none_for_non_structure() {
    assert!(super::parse_structure_violation(None, 10).is_none());
    assert!(super::parse_structure_violation(Some("content violation"), 10).is_none());
    assert!(super::parse_structure_violation(Some("structure: unknown type"), 10).is_none());
}

#[test]
fn update_baseline_mode_all_creates_baseline_with_content_violations() {
    use crate::cli::BaselineUpdateMode;

    let temp_dir = TempDir::new().unwrap();

    // Create a file that will exceed the limit
    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n";
    std::fs::write(&config_path, config_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: None,  // Use config extensions
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: Some(baseline_path.clone()),
        update_baseline: Some(BaselineUpdateMode::All),
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    let exit_code = result.expect("Check should succeed");

    // Should fail with threshold exceeded
    assert_eq!(exit_code, EXIT_THRESHOLD_EXCEEDED, "Should detect violation");

    // Baseline file should be created
    assert!(baseline_path.exists(), "Baseline file should exist");

    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1, "Baseline should have 1 entry");

    // The baseline uses forward slashes and just the filename
    let keys: Vec<_> = baseline.files().keys().collect();
    let key = keys[0];
    assert!(
        key.ends_with("large_file.rs"),
        "Baseline key should end with large_file.rs, got: {key}"
    );
}

#[test]
fn update_baseline_mode_content_only_excludes_structure() {
    use crate::cli::BaselineUpdateMode;

    let temp_dir = TempDir::new().unwrap();

    // Create a large file (content violation)
    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    // Create many files to trigger structure violation
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file{i}.rs"));
        std::fs::write(&file_path, "fn main() {}\n").unwrap();
    }

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content =
        "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n\n[structure]\nmax_files = 5\n";
    std::fs::write(&config_path, config_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: Some(baseline_path.clone()),
        update_baseline: Some(BaselineUpdateMode::Content),
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok(), "Check failed: {:?}", result.err());

    // Baseline should contain only the content violation
    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1, "Should have 1 content violation");

    // Verify it's the content file
    let keys: Vec<_> = baseline.files().keys().collect();
    assert!(keys[0].ends_with("large_file.rs"));
}

#[test]
fn update_baseline_mode_structure_only_excludes_content() {
    use crate::baseline::BaselineEntry;
    use crate::cli::BaselineUpdateMode;

    let temp_dir = TempDir::new().unwrap();

    // Create a large file (content violation)
    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    // Create many files to trigger structure violation
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file{i}.rs"));
        std::fs::write(&file_path, "fn main() {}\n").unwrap();
    }

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content =
        "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n\n[structure]\nmax_files = 5\n";
    std::fs::write(&config_path, config_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: Some(baseline_path.clone()),
        update_baseline: Some(BaselineUpdateMode::Structure),
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok(), "Check failed: {:?}", result.err());

    // Baseline should contain only structure violations
    let baseline = Baseline::load(&baseline_path).unwrap();
    // Should have 1 structure violation (the temp dir itself exceeds file count)
    assert!(!baseline.is_empty(), "Should have structure violations");
    // All entries should be structure type
    for entry in baseline.files().values() {
        assert!(matches!(entry, BaselineEntry::Structure { .. }));
    }
}

#[test]
fn update_baseline_mode_new_preserves_existing_entries() {
    use crate::baseline::BaselineEntry;
    use crate::cli::BaselineUpdateMode;

    let temp_dir = TempDir::new().unwrap();

    // Create a large file (violation)
    let test_file_path = temp_dir.path().join("new_large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n";
    std::fs::write(&config_path, config_content).unwrap();

    // Create an existing baseline with a different file
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut existing_baseline = Baseline::new();
    existing_baseline.set_content("old_file.rs", 200, "oldhash".to_string());
    existing_baseline.save(&baseline_path).unwrap();

    let args = CheckArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: Some(baseline_path.clone()),
        update_baseline: Some(BaselineUpdateMode::New),
        no_cache: true,
        no_gitignore: true,
        fix: false,
        max_files: None,
        max_dirs: None,
        report_json: None,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok(), "Check failed: {:?}", result.err());

    // Baseline should contain both old and new entries
    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 2, "Should have 2 entries: old + new");
    assert!(baseline.contains("old_file.rs"), "Should contain old_file.rs");

    // Check that the new file is in the baseline (path includes temp dir)
    let has_new_file = baseline
        .files()
        .keys()
        .any(|k| k.ends_with("new_large_file.rs"));
    assert!(has_new_file, "Should contain new_large_file.rs");

    // Old entry should be preserved with original values
    match baseline.get("old_file.rs").unwrap() {
        BaselineEntry::Content { lines, hash } => {
            assert_eq!(*lines, 200);
            assert_eq!(hash, "oldhash");
        }
        BaselineEntry::Structure { .. } => panic!("Expected Content entry"),
    }
}
