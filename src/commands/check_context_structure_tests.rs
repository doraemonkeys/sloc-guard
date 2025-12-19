// Context injection and CLI structure parameters tests

use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

use crate::cache::Cache;
use crate::checker::ThresholdChecker;
use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, InitArgs};
use crate::config::Config;
use crate::language::LanguageRegistry;
use crate::output::OutputFormat;
use crate::scanner::{CompositeScanner, FileScanner};
use crate::EXIT_THRESHOLD_EXCEEDED;

use super::*;
use crate::commands::context::{CheckContext, FileReader, RealFileReader};

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

fn make_check_args(paths: Vec<PathBuf>) -> CheckArgs {
    CheckArgs {
        paths,
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: false,
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
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
    }
}

fn make_check_args_with_structure(
    paths: Vec<PathBuf>,
    max_files: Option<i64>,
    max_dirs: Option<i64>,
) -> CheckArgs {
    CheckArgs {
        ext: Some(vec!["rs".to_string()]),
        max_files,
        max_dirs,
        ..make_check_args(paths)
    }
}

// =============================================================================
// Context Injection Tests (Task 5.5.13: Testability Refactoring)
// =============================================================================

#[test]
fn check_context_from_config_creates_valid_context() {
    let config = Config::default();
    let ctx = CheckContext::from_config(&config, 0.9, Vec::new(), false).unwrap();

    if let Some(ref sc) = ctx.structure_checker {
        assert!(!sc.is_enabled());
    }
}

#[test]
fn check_context_new_allows_custom_injection() {
    let config = Config::default();
    let registry = LanguageRegistry::default();
    let threshold_checker = ThresholdChecker::new(config).with_warning_threshold(0.5);
    let scanner: Box<dyn FileScanner> = Box::new(CompositeScanner::new(Vec::new(), false));
    let file_reader: Box<dyn FileReader> = Box::new(RealFileReader);

    let ctx = CheckContext::new(registry, threshold_checker, None, scanner, file_reader);

    assert!(ctx.structure_checker.is_none());
}

#[test]
fn run_check_with_context_uses_injected_threshold_checker() {
    let temp_dir = TempDir::new().unwrap();

    let test_file = temp_dir.path().join("small.rs");
    std::fs::write(&test_file, "fn main() {}\n").unwrap();

    let mut config = Config::default();
    config.content.extensions = vec!["rs".to_string()];
    config.content.max_lines = 1;

    let ctx = CheckContext::from_config(&config, 0.9, Vec::new(), false).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));

    let args = make_check_args(vec![temp_dir.path().to_path_buf()]);

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);
    let paths = args.paths.clone();

    let result = run_check_with_context(&args, &cli, &paths, &config, &ctx, &cache, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), crate::EXIT_SUCCESS);
}

#[test]
fn run_check_with_context_uses_injected_structure_checker() {
    let temp_dir = TempDir::new().unwrap();

    let sub_dir = temp_dir.path().join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(sub_dir.join("b.rs"), "fn b() {}").unwrap();
    std::fs::write(sub_dir.join("c.rs"), "fn c() {}").unwrap();

    let mut config = Config::default();
    config.content.extensions = vec!["rs".to_string()];
    config.structure.max_files = Some(2);

    let ctx = CheckContext::from_config(&config, 0.9, Vec::new(), false).unwrap();
    let cache = Mutex::new(Cache::new(String::new()));

    assert!(ctx.structure_checker.is_some());
    assert!(ctx.structure_checker.as_ref().unwrap().is_enabled());

    let args = make_check_args(vec![sub_dir.clone()]);

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);
    let paths = vec![sub_dir];

    let result = run_check_with_context(&args, &cli, &paths, &config, &ctx, &cache, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Task 6.1: CLI Structure Parameters Tests
// =============================================================================

#[test]
fn validate_and_resolve_paths_no_args_defaults_to_current_dir() {
    let args = make_check_args(vec![]);

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_ok());
    let paths = result.unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], PathBuf::from("."));
}

#[test]
fn validate_and_resolve_paths_max_files_without_path_returns_error() {
    let args = make_check_args_with_structure(vec![], Some(10), None);

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("--max-files/--max-dirs/--max-depth require a target <PATH>")
    );
}

#[test]
fn validate_and_resolve_paths_max_dirs_without_path_returns_error() {
    let args = make_check_args_with_structure(vec![], None, Some(5));

    let result = validate_and_resolve_paths(&args);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("--max-files/--max-dirs/--max-depth require a target <PATH>")
    );
}

#[test]
fn validate_and_resolve_paths_max_files_with_path_succeeds() {
    let args = make_check_args_with_structure(vec![PathBuf::from("src")], Some(10), None);

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

    let args = make_check_args_with_structure(vec![PathBuf::from("src")], Some(10), None);

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.structure.max_files, Some(10));
}

#[test]
fn apply_cli_overrides_structure_max_dirs() {
    let mut config = Config::default();
    assert!(config.structure.max_dirs.is_none());

    let args = make_check_args_with_structure(vec![PathBuf::from("src")], None, Some(5));

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.structure.max_dirs, Some(5));
}

#[test]
fn run_check_with_cli_max_files_overrides_config() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::write(temp_dir.path().join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(temp_dir.path().join("b.rs"), "fn b() {}").unwrap();
    std::fs::write(temp_dir.path().join("c.rs"), "fn c() {}").unwrap();

    let args = make_check_args_with_structure(vec![temp_dir.path().to_path_buf()], Some(2), None);

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn run_check_with_cli_max_dirs_overrides_config() {
    let temp_dir = TempDir::new().unwrap();

    std::fs::create_dir(temp_dir.path().join("sub1")).unwrap();
    std::fs::create_dir(temp_dir.path().join("sub2")).unwrap();
    std::fs::create_dir(temp_dir.path().join("sub3")).unwrap();

    let args = make_check_args_with_structure(vec![temp_dir.path().to_path_buf()], None, Some(2));

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, true);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

