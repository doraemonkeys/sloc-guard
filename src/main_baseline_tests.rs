use std::path::PathBuf;

use sloc_guard::baseline::Baseline;
use sloc_guard::checker::{CheckResult, CheckStatus};
use sloc_guard::cli::{BaselineUpdateArgs, CheckArgs, Cli, ColorChoice, Commands, InitArgs};
use sloc_guard::config::Config;
use sloc_guard::counter::LineStats;
use sloc_guard::output::OutputFormat;
use sloc_guard::{EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};
use tempfile::TempDir;

use crate::{
    apply_baseline_comparison, get_baseline_scan_paths, load_baseline, run_baseline_update_impl,
    run_check_impl,
};

fn make_cli_for_baseline(quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
        }),
        verbose: 0,
        quiet,
        color: ColorChoice::Never,
        no_config,
        no_extends: false,
    }
}

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

#[test]
fn get_baseline_scan_paths_uses_include_override() {
    let config = Config::default();
    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec!["src".to_string(), "lib".to_string()],
        no_gitignore: false,
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("lib")]);
}

#[test]
fn get_baseline_scan_paths_uses_cli_paths() {
    let config = Config::default();
    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from("src"), PathBuf::from("tests")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec![],
        no_gitignore: false,
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src"), PathBuf::from("tests")]);
}

#[test]
fn get_baseline_scan_paths_uses_config_include_paths() {
    let mut config = Config::default();
    config.default.include_paths = vec!["src".to_string()];

    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec![],
        no_gitignore: false,
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from("src")]);
}

#[test]
fn get_baseline_scan_paths_defaults_to_current_dir() {
    let config = Config::default();
    let args = BaselineUpdateArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        output: PathBuf::from(".sloc-guard-baseline.json"),
        ext: None,
        exclude: vec![],
        include: vec![],
        no_gitignore: false,
    };

    let paths = get_baseline_scan_paths(&args, &config);
    assert_eq!(paths, vec![PathBuf::from(".")]);
}

#[test]
fn run_baseline_update_creates_empty_baseline_when_no_violations() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let test_file_path = temp_dir.path().join("small_file.rs");
    std::fs::write(&test_file_path, "fn main() {}\n").unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: None,
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_gitignore: false,
    };

    let cli = make_cli_for_baseline(true, true);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());

    assert!(baseline_path.exists());
    let baseline = Baseline::load(&baseline_path).unwrap();
    assert!(baseline.is_empty());
}

#[test]
fn run_baseline_update_captures_violations() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_gitignore: true,
    };

    let cli = make_cli_for_baseline(true, false);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    assert!(baseline_path.exists());
    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1);
}

#[test]
fn run_baseline_update_with_exclude_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let src_dir = temp_dir.path().join("src");
    let vendor_dir = temp_dir.path().join("vendor");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&vendor_dir).unwrap();

    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(src_dir.join("main.rs"), &large_content).unwrap();
    std::fs::write(vendor_dir.join("lib.rs"), &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec!["**/vendor/**".to_string()],
        include: vec![],
        no_gitignore: true,
    };

    let cli = make_cli_for_baseline(true, false);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1);
    let files: Vec<_> = baseline.files().keys().collect();
    assert!(files.iter().any(|f| f.contains("main.rs")));
    assert!(!files.iter().any(|f| f.contains("vendor")));
}

#[test]
fn baseline_file_contains_correct_hash() {
    let temp_dir = TempDir::new().unwrap();
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let test_file_path = temp_dir.path().join("test.rs");
    let content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "[default]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = BaselineUpdateArgs {
        paths: vec![temp_dir.path().to_path_buf()],
        config: Some(config_path),
        output: baseline_path.clone(),
        ext: Some(vec!["rs".to_string()]),
        exclude: vec![],
        include: vec![],
        no_gitignore: true,
    };

    let cli = make_cli_for_baseline(true, false);

    let result = run_baseline_update_impl(&args, &cli);
    assert!(result.is_ok());

    let baseline = Baseline::load(&baseline_path).unwrap();
    let entry = baseline.files().values().next().unwrap();

    assert_eq!(entry.hash.len(), 64);
    assert!(entry.hash.chars().all(|c| c.is_ascii_hexdigit()));
}

// Baseline comparison tests

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
    baseline.set("test/file.rs", 100, "abc123".to_string());
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
        CheckResult {
            path: PathBuf::from("src/file.rs"),
            status: CheckStatus::Failed,
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
        CheckResult {
            path: PathBuf::from("src/other.rs"),
            status: CheckStatus::Passed,
            stats: LineStats {
                total: 100,
                code: 100,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            limit: 500,
            override_reason: None,
            suggestions: None,
        },
    ];

    let mut baseline = Baseline::new();
    baseline.set("src/file.rs", 600, "hash123".to_string());

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_grandfathered());
    assert!(results[1].is_passed());
}

#[test]
fn apply_baseline_comparison_does_not_mark_new_violations() {
    let mut results = vec![CheckResult {
        path: PathBuf::from("src/new_file.rs"),
        status: CheckStatus::Failed,
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
    let mut results = vec![CheckResult {
        path: PathBuf::from("src\\file.rs"),
        status: CheckStatus::Failed,
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
    baseline.set("src/file.rs", 600, "hash123".to_string());

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
    baseline.set(&file_path_str, 102, "dummy_hash".to_string());
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
        no_cache: true,
        no_gitignore: false,
        fix: false,
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
        no_cache: true,
        no_gitignore: true,
        fix: false,
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
        no_cache: true,
        no_gitignore: true,
        fix: false,
    };

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}
