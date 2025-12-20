use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::cache::Cache;
use crate::checker::ThresholdChecker;
use crate::cli::CheckArgs;
use crate::config::Config;
use crate::counter::LineStats;
use crate::language::LanguageRegistry;
use crate::output::{ColorMode, OutputFormat};

use super::*;
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

#[test]
fn format_output_text() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let output = format_output(OutputFormat::Text, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_output_json() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let output = format_output(OutputFormat::Json, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_output_sarif_works() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let result = format_output(OutputFormat::Sarif, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("$schema"));
    assert!(output.contains("2.1.0"));
}

#[test]
fn format_output_markdown_works() {
    let results: Vec<crate::checker::CheckResult> = vec![];
    let result = format_output(OutputFormat::Markdown, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Guard Results"));
    assert!(output.contains("| Total Files | 0 |"));
}

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
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
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
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
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
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
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
        no_cache: true,
        no_gitignore: false,
        suggest: false,
        max_files: None,
        max_dirs: None,
        max_depth: None,
        report_json: None,
        files: vec![],
    };

    apply_cli_overrides(&mut config, &args);
    assert!((config.content.warn_threshold - 0.8).abs() < f64::EPSILON);
}

// ============================================================================
// Override Path Validation Tests
// ============================================================================

#[test]
fn path_matches_override_exact_match() {
    use super::path_matches_override;
    let path = PathBuf::from("src/main.rs");
    assert!(path_matches_override(&path, "src/main.rs"));
    assert!(path_matches_override(&path, "main.rs"));
}

#[test]
fn path_matches_override_suffix_match() {
    use super::path_matches_override;
    let path = PathBuf::from("project/src/components/button.rs");
    assert!(path_matches_override(&path, "button.rs"));
    assert!(path_matches_override(&path, "components/button.rs"));
    assert!(path_matches_override(&path, "src/components/button.rs"));
    assert!(path_matches_override(
        &path,
        "project/src/components/button.rs"
    ));
}

#[test]
fn path_matches_override_no_match() {
    use super::path_matches_override;
    let path = PathBuf::from("src/main.rs");
    assert!(!path_matches_override(&path, "other.rs"));
    assert!(!path_matches_override(&path, "src/other.rs"));
    assert!(!path_matches_override(&path, "deep/nested/src/main.rs"));
}

#[test]
fn path_matches_override_partial_component_no_match() {
    use super::path_matches_override;
    let path = PathBuf::from("src/main.rs");
    // Should not match partial component names
    assert!(!path_matches_override(&path, "ain.rs"));
    assert!(!path_matches_override(&path, "rc/main.rs"));
}

#[test]
fn validate_override_paths_valid_content_override() {
    use super::validate_override_paths;
    use crate::checker::DirStats;
    use crate::config::{ContentOverride, StructureOverride};

    let content_overrides = vec![ContentOverride {
        path: "src/main.rs".to_string(),
        max_lines: 1000,
        reason: "Legacy file".to_string(),
    }];
    let structure_overrides: Vec<StructureOverride> = vec![];
    let files = vec![PathBuf::from("src/main.rs")];
    let directories: HashMap<PathBuf, DirStats> = HashMap::new();

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_ok());
}

#[test]
fn validate_override_paths_content_override_matches_directory() {
    use super::validate_override_paths;
    use crate::checker::DirStats;
    use crate::config::{ContentOverride, StructureOverride};

    let content_overrides = vec![ContentOverride {
        path: "src/components".to_string(),
        max_lines: 1000,
        reason: "Legacy file".to_string(),
    }];
    let structure_overrides: Vec<StructureOverride> = vec![];
    let files = vec![PathBuf::from("src/main.rs")];
    let mut directories: HashMap<PathBuf, DirStats> = HashMap::new();
    directories.insert(
        PathBuf::from("src/components"),
        DirStats {
            file_count: 5,
            dir_count: 0,
            depth: 1,
        },
    );

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("content.override[0]"));
    assert!(err.contains("matches directory"));
    assert!(err.contains("content overrides only apply to files"));
}

#[test]
fn validate_override_paths_valid_structure_override() {
    use super::validate_override_paths;
    use crate::checker::DirStats;
    use crate::config::{ContentOverride, StructureOverride};

    let content_overrides: Vec<ContentOverride> = vec![];
    let structure_overrides = vec![StructureOverride {
        path: "src/components".to_string(),
        max_files: Some(100),
        max_dirs: None,
        max_depth: None,
        reason: "Large component directory".to_string(),
    }];
    let files = vec![PathBuf::from("src/main.rs")];
    let mut directories: HashMap<PathBuf, DirStats> = HashMap::new();
    directories.insert(
        PathBuf::from("src/components"),
        DirStats {
            file_count: 50,
            dir_count: 2,
            depth: 1,
        },
    );

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_ok());
}

#[test]
fn validate_override_paths_structure_override_matches_file() {
    use super::validate_override_paths;
    use crate::checker::DirStats;
    use crate::config::{ContentOverride, StructureOverride};

    let content_overrides: Vec<ContentOverride> = vec![];
    let structure_overrides = vec![StructureOverride {
        path: "src/main.rs".to_string(),
        max_files: Some(100),
        max_dirs: None,
        max_depth: None,
        reason: "Misconfig".to_string(),
    }];
    let files = vec![PathBuf::from("src/main.rs")];
    let directories: HashMap<PathBuf, DirStats> = HashMap::new();

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("structure.override[0]"));
    assert!(err.contains("matches file"));
    assert!(err.contains("structure overrides only apply to directories"));
}

#[test]
fn validate_override_paths_suffix_matching() {
    use super::validate_override_paths;
    use crate::checker::DirStats;
    use crate::config::{ContentOverride, StructureOverride};

    // ContentOverride path "legacy" should match directory "project/src/legacy"
    let content_overrides = vec![ContentOverride {
        path: "legacy".to_string(),
        max_lines: 1000,
        reason: "Legacy".to_string(),
    }];
    let structure_overrides: Vec<StructureOverride> = vec![];
    let files: Vec<PathBuf> = vec![];
    let mut directories: HashMap<PathBuf, DirStats> = HashMap::new();
    directories.insert(
        PathBuf::from("project/src/legacy"),
        DirStats {
            file_count: 10,
            dir_count: 0,
            depth: 2,
        },
    );

    let result = validate_override_paths(
        &content_overrides,
        &structure_overrides,
        &files,
        &directories,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("legacy"));
}
