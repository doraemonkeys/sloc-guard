use std::path::PathBuf;

use crate::cli::CheckArgs;
use crate::config::Config;
use crate::output::OutputFormat;

use super::*;

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
        count_comments: false,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        staged: false,
        strict: false,
        baseline: None,
        update_baseline: None,
        ratchet: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
        write_sarif: None,
        write_json: None,
        files: vec![],
    };

    apply_cli_overrides(&mut config, &args);
    assert_eq!(config.content.max_lines, 100);
}

#[test]
fn apply_cli_overrides_count_comments() {
    let mut config = Config::default();
    assert!(config.content.skip_comments);

    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        count_comments: true,
        count_blank: false,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        staged: false,
        strict: false,
        baseline: None,
        update_baseline: None,
        ratchet: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
        write_sarif: None,
        write_json: None,
        files: vec![],
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.content.skip_comments);
}

#[test]
fn apply_cli_overrides_count_blank() {
    let mut config = Config::default();
    assert!(config.content.skip_blank);

    let args = CheckArgs {
        paths: vec![PathBuf::from(".")],
        config: None,
        max_lines: None,
        ext: None,
        exclude: vec![],
        include: vec![],
        count_comments: false,
        count_blank: true,
        warn_threshold: None,
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        staged: false,
        strict: false,
        baseline: None,
        update_baseline: None,
        ratchet: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
        write_sarif: None,
        write_json: None,
        files: vec![],
    };

    apply_cli_overrides(&mut config, &args);
    assert!(!config.content.skip_blank);
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
        count_comments: false,
        count_blank: false,
        warn_threshold: Some(0.8),
        format: OutputFormat::Text,
        output: None,
        warn_only: false,
        diff: None,
        staged: false,
        strict: false,
        baseline: None,
        update_baseline: None,
        ratchet: None,
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
        write_sarif: None,
        write_json: None,
        files: vec![],
    };

    apply_cli_overrides(&mut config, &args);
    assert!((config.content.warn_threshold - 0.8).abs() < f64::EPSILON);
}
