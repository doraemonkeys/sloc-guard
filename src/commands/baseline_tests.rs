use std::path::PathBuf;
use tempfile::TempDir;

use super::run_baseline_update_impl;
use crate::baseline::Baseline;
use crate::cli::{BaselineUpdateArgs, Cli, ColorChoice, Commands, InitArgs};

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
