use std::path::PathBuf;

use sloc_guard::cli::{CheckArgs, ColorChoice};
use sloc_guard::config::Config;
use sloc_guard::output::{ColorMode, OutputFormat};

use crate::{apply_cli_overrides, color_choice_to_mode, resolve_scan_paths};

#[test]
fn apply_cli_overrides_max_lines() {
    let mut config = Config::default();
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: Some(100),
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
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.default.max_lines, 100);
}

#[test]
fn apply_cli_overrides_no_skip_comments() {
    let mut config = Config::default();
    assert!(config.default.skip_comments);

    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: true,
        no_skip_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.default.skip_comments);
}

#[test]
fn apply_cli_overrides_no_skip_blank() {
    let mut config = Config::default();
    assert!(config.default.skip_blank);

    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: true,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.default.skip_blank);
}

#[test]
fn apply_cli_overrides_warn_threshold() {
    let mut config = Config::default();
    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        no_skip_comments: false,
        no_skip_blank: false,
        warn_threshold: Some(0.8),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        strict: false,
        baseline: None,
        no_cache: true,
        no_gitignore: false,
        fix: false,
    };

    apply_cli_overrides(&mut config, &args);
    assert!((config.default.warn_threshold - 0.8).abs() < f64::EPSILON);
}

#[test]
fn resolve_scan_paths_uses_include_override() {
    let config = Config::default();
    let paths = vec![PathBuf::from(".")];
    let include = vec!["src".to_string(), "lib".to_string()];

    let result = resolve_scan_paths(&paths, &include, &config);
    assert_eq!(result, vec![PathBuf::from("src"), PathBuf::from("lib")]);
}

#[test]
fn resolve_scan_paths_uses_cli_paths() {
    let config = Config::default();
    let paths = vec![PathBuf::from("src"), PathBuf::from("tests")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include, &config);
    assert_eq!(result, vec![PathBuf::from("src"), PathBuf::from("tests")]);
}

#[test]
fn resolve_scan_paths_uses_config_include_paths() {
    let mut config = Config::default();
    config.default.include_paths = vec!["src".to_string()];

    let paths = vec![PathBuf::from(".")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include, &config);
    assert_eq!(result, vec![PathBuf::from("src")]);
}

#[test]
fn resolve_scan_paths_defaults_to_current_dir() {
    let config = Config::default();
    let paths = vec![PathBuf::from(".")];
    let include: Vec<String> = vec![];

    let result = resolve_scan_paths(&paths, &include, &config);
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
