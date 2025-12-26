// Baseline feature tests: loading, comparison, grandfathering, and update modes

use std::path::PathBuf;
use tempfile::TempDir;

use crate::baseline::Baseline;
use crate::checker::CheckResult;
use crate::cli::{CheckArgs, Cli, ColorChoice, Commands, InitArgs};
use crate::counter::LineStats;
use crate::output::OutputFormat;
use crate::{EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::*;
use crate::commands::check::run_check_impl;

fn make_cli_for_check(color: ColorChoice, verbose: u8, quiet: bool, no_config: bool) -> Cli {
    Cli {
        command: Commands::Init(InitArgs {
            output: PathBuf::from(".sloc-guard.toml"),
            force: false,
            detect: false,
        }),
        verbose,
        quiet,
        color,
        no_config,
        no_extends: false,
        offline: false,
    }
}

/// Builder for constructing `CheckArgs` in tests with sensible defaults.
#[derive(Default)]
struct CheckArgsBuilder {
    paths: Vec<PathBuf>,
    config: Option<PathBuf>,
    baseline: Option<PathBuf>,
    update_baseline: Option<crate::cli::BaselineUpdateMode>,
    ratchet: Option<crate::cli::RatchetMode>,
}

impl CheckArgsBuilder {
    fn paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = paths;
        self
    }

    fn config(mut self, config: PathBuf) -> Self {
        self.config = Some(config);
        self
    }

    fn baseline(mut self, baseline: PathBuf) -> Self {
        self.baseline = Some(baseline);
        self
    }

    fn update_baseline(mut self, mode: crate::cli::BaselineUpdateMode) -> Self {
        self.update_baseline = Some(mode);
        self
    }

    fn ratchet(mut self, mode: crate::cli::RatchetMode) -> Self {
        self.ratchet = Some(mode);
        self
    }

    fn build(self) -> CheckArgs {
        CheckArgs {
            paths: self.paths,
            config: self.config,
            max_lines: None,
            ext: Some(vec!["rs".to_string()]),
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
            baseline: self.baseline,
            update_baseline: self.update_baseline,
            ratchet: self.ratchet,
            no_cache: true,
            no_gitignore: true,
            suggest: false,
            max_files: None,
            max_dirs: None,
            max_depth: None,
            report_json: None,
            files: vec![],
        }
    }
}

fn make_check_args_with_baseline(
    paths: Vec<PathBuf>,
    config: Option<PathBuf>,
    baseline: Option<PathBuf>,
    update_baseline: Option<crate::cli::BaselineUpdateMode>,
) -> CheckArgs {
    let mut builder = CheckArgsBuilder::default().paths(paths);
    if let Some(c) = config {
        builder = builder.config(c);
    }
    if let Some(b) = baseline {
        builder = builder.baseline(b);
    }
    if let Some(u) = update_baseline {
        builder = builder.update_baseline(u);
    }
    builder.build()
}

// =============================================================================
// Baseline Loading Tests
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

// =============================================================================
// Baseline Comparison Tests
// =============================================================================

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
            raw_stats: None,
            limit: 500,
            override_reason: None,
            suggestions: None,
            violation_category: None,
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
            raw_stats: None,
            limit: 500,
            override_reason: None,
            violation_category: None,
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
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
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
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }];

    let mut baseline = Baseline::new();
    baseline.set_content("src/file.rs", 600, "hash123".to_string());

    apply_baseline_comparison(&mut results, &baseline);

    assert!(results[0].is_grandfathered());
}

// =============================================================================
// Baseline Integration Tests
// =============================================================================

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
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = make_check_args_with_baseline(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path),
        None,
    );

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
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = make_check_args_with_baseline(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        None,
        None,
    );

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
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = make_check_args_with_baseline(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path),
        None,
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

// =============================================================================
// Update Baseline Helper Tests
// =============================================================================

#[test]
fn is_structure_violation_returns_true_for_structure_violations() {
    assert!(super::is_structure_violation(Some(
        "structure: files count exceeded"
    )));
    assert!(super::is_structure_violation(Some(
        "structure: subdirs count exceeded"
    )));
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

// =============================================================================
// Update Baseline Mode Tests
// =============================================================================

#[test]
fn update_baseline_mode_all_creates_baseline_with_content_violations() {
    use crate::cli::BaselineUpdateMode;

    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n";
    std::fs::write(&config_path, config_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let args = make_check_args_with_baseline(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path.clone()),
        Some(BaselineUpdateMode::All),
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    let exit_code = result.expect("Check should succeed");

    assert_eq!(
        exit_code, EXIT_THRESHOLD_EXCEEDED,
        "Should detect violation"
    );

    assert!(baseline_path.exists(), "Baseline file should exist");

    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1, "Baseline should have 1 entry");

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

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file{i}.rs"));
        std::fs::write(&file_path, "fn main() {}\n").unwrap();
    }

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n\n[structure]\nmax_files = 5\n";
    std::fs::write(&config_path, config_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let args = make_check_args_with_baseline(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path.clone()),
        Some(BaselineUpdateMode::Content),
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok(), "Check failed: {:?}", result.err());

    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 1, "Should have 1 content violation");

    let keys: Vec<_> = baseline.files().keys().collect();
    assert!(keys[0].ends_with("large_file.rs"));
}

#[test]
fn update_baseline_mode_structure_only_excludes_content() {
    use crate::baseline::BaselineEntry;
    use crate::cli::BaselineUpdateMode;

    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file{i}.rs"));
        std::fs::write(&file_path, "fn main() {}\n").unwrap();
    }

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n\n[structure]\nmax_files = 5\n";
    std::fs::write(&config_path, config_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");

    let args = make_check_args_with_baseline(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path.clone()),
        Some(BaselineUpdateMode::Structure),
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok(), "Check failed: {:?}", result.err());

    let baseline = Baseline::load(&baseline_path).unwrap();
    assert!(!baseline.is_empty(), "Should have structure violations");
    for entry in baseline.files().values() {
        assert!(matches!(entry, BaselineEntry::Structure { .. }));
    }
}

#[test]
fn update_baseline_mode_new_preserves_existing_entries() {
    use crate::baseline::BaselineEntry;
    use crate::cli::BaselineUpdateMode;

    let temp_dir = TempDir::new().unwrap();

    let test_file_path = temp_dir.path().join("new_large_file.rs");
    let large_content = "fn main() {\n".to_string() + &"let x = 1;\n".repeat(100) + "}\n";
    std::fs::write(&test_file_path, &large_content).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 10\nextensions = [\"rs\"]\n";
    std::fs::write(&config_path, config_content).unwrap();

    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut existing_baseline = Baseline::new();
    existing_baseline.set_content("old_file.rs", 200, "oldhash".to_string());
    existing_baseline.save(&baseline_path).unwrap();

    let args = make_check_args_with_baseline(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path.clone()),
        Some(BaselineUpdateMode::New),
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok(), "Check failed: {:?}", result.err());

    let baseline = Baseline::load(&baseline_path).unwrap();
    assert_eq!(baseline.len(), 2, "Should have 2 entries: old + new");
    assert!(
        baseline.contains("old_file.rs"),
        "Should contain old_file.rs"
    );

    let has_new_file = baseline
        .files()
        .keys()
        .any(|k| k.ends_with("new_large_file.rs"));
    assert!(has_new_file, "Should contain new_large_file.rs");

    match baseline.get("old_file.rs").unwrap() {
        BaselineEntry::Content { lines, hash } => {
            assert_eq!(*lines, 200);
            assert_eq!(hash, "oldhash");
        }
        BaselineEntry::Structure { .. } => panic!("Expected Content entry"),
    }
}

// =============================================================================
// Baseline Ratchet Tests
// =============================================================================

fn make_check_args_with_ratchet(
    paths: Vec<PathBuf>,
    config: Option<PathBuf>,
    baseline: Option<PathBuf>,
    ratchet: Option<crate::cli::RatchetMode>,
) -> CheckArgs {
    let mut builder = CheckArgsBuilder::default().paths(paths);
    if let Some(c) = config {
        builder = builder.config(c);
    }
    if let Some(b) = baseline {
        builder = builder.baseline(b);
    }
    if let Some(r) = ratchet {
        builder = builder.ratchet(r);
    }
    builder.build()
}

#[test]
fn check_baseline_ratchet_detects_stale_entries() {
    use super::check_baseline_ratchet;

    // Current results: only file1.rs is failing
    let results = vec![CheckResult::Failed {
        path: PathBuf::from("file1.rs"),
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }];

    // Baseline has file1.rs and file2.rs (file2.rs was fixed)
    let mut baseline = Baseline::new();
    baseline.set_content("file1.rs", 600, "hash1".to_string());
    baseline.set_content("file2.rs", 700, "hash2".to_string());

    let ratchet_result = check_baseline_ratchet(&results, &baseline);

    assert!(ratchet_result.is_outdated());
    assert_eq!(ratchet_result.stale_entries, 1);
    assert_eq!(ratchet_result.stale_paths, vec!["file2.rs".to_string()]);
}

#[test]
fn check_baseline_ratchet_no_stale_when_all_still_failing() {
    use super::check_baseline_ratchet;

    let results = vec![
        CheckResult::Failed {
            path: PathBuf::from("file1.rs"),
            stats: LineStats {
                total: 600,
                code: 600,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            raw_stats: None,
            limit: 500,
            override_reason: None,
            suggestions: None,
            violation_category: None,
        },
        CheckResult::Grandfathered {
            path: PathBuf::from("file2.rs"),
            stats: LineStats {
                total: 700,
                code: 700,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            raw_stats: None,
            limit: 500,
            override_reason: None,
            violation_category: None,
        },
    ];

    let mut baseline = Baseline::new();
    baseline.set_content("file1.rs", 600, "hash1".to_string());
    baseline.set_content("file2.rs", 700, "hash2".to_string());

    let ratchet_result = check_baseline_ratchet(&results, &baseline);

    assert!(!ratchet_result.is_outdated());
    assert_eq!(ratchet_result.stale_entries, 0);
}

#[test]
fn ratchet_strict_fails_when_baseline_outdated() {
    use crate::cli::RatchetMode;

    let temp_dir = TempDir::new().unwrap();

    // Create a small compliant file
    let test_file = temp_dir.path().join("small_file.rs");
    std::fs::write(&test_file, "fn main() {}\n").unwrap();

    // Create baseline with an entry for this file (simulating previously violated)
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut baseline = Baseline::new();
    let file_path_str = test_file.to_string_lossy().replace('\\', "/");
    baseline.set_content(&file_path_str, 100, "dummy_hash".to_string());
    baseline.save(&baseline_path).unwrap();

    // Config with lenient limit (file now passes)
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 500\nextensions = [\"rs\"]\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = make_check_args_with_ratchet(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path),
        Some(RatchetMode::Strict),
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Strict mode should fail because baseline has stale entry
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}

#[test]
fn ratchet_warn_succeeds_but_warns_when_outdated() {
    use crate::cli::RatchetMode;

    let temp_dir = TempDir::new().unwrap();

    // Create a small compliant file
    let test_file = temp_dir.path().join("small_file.rs");
    std::fs::write(&test_file, "fn main() {}\n").unwrap();

    // Create baseline with an entry for this file
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut baseline = Baseline::new();
    let file_path_str = test_file.to_string_lossy().replace('\\', "/");
    baseline.set_content(&file_path_str, 100, "dummy_hash".to_string());
    baseline.save(&baseline_path).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 500\nextensions = [\"rs\"]\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = make_check_args_with_ratchet(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path),
        Some(RatchetMode::Warn),
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Warn mode should succeed (just emit warning)
    assert_eq!(result.unwrap(), EXIT_SUCCESS);
}

#[test]
fn ratchet_auto_updates_baseline_when_outdated() {
    use crate::cli::RatchetMode;

    let temp_dir = TempDir::new().unwrap();

    // Create a small compliant file
    let test_file = temp_dir.path().join("small_file.rs");
    std::fs::write(&test_file, "fn main() {}\n").unwrap();

    // Create baseline with an entry for this file
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut baseline = Baseline::new();
    let file_path_str = test_file.to_string_lossy().replace('\\', "/");
    baseline.set_content(&file_path_str, 100, "dummy_hash".to_string());
    baseline.save(&baseline_path).unwrap();

    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = "version = \"2\"\n\n[content]\nmax_lines = 500\nextensions = [\"rs\"]\n";
    std::fs::write(&config_path, config_content).unwrap();

    let args = make_check_args_with_ratchet(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path.clone()),
        Some(RatchetMode::Auto),
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), EXIT_SUCCESS);

    // Baseline should be updated (stale entry removed)
    let updated_baseline = Baseline::load(&baseline_path).unwrap();
    assert!(
        updated_baseline.is_empty(),
        "Baseline should be empty after auto-tighten"
    );
}

#[test]
fn ratchet_config_works_without_cli_flag() {
    let temp_dir = TempDir::new().unwrap();

    // Create a small compliant file
    let test_file = temp_dir.path().join("small_file.rs");
    std::fs::write(&test_file, "fn main() {}\n").unwrap();

    // Create baseline with an entry for this file
    let baseline_path = temp_dir.path().join(".sloc-guard-baseline.json");
    let mut baseline = Baseline::new();
    let file_path_str = test_file.to_string_lossy().replace('\\', "/");
    baseline.set_content(&file_path_str, 100, "dummy_hash".to_string());
    baseline.save(&baseline_path).unwrap();

    // Config with ratchet = "strict"
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = r#"version = "2"

[content]
max_lines = 500
extensions = ["rs"]

[baseline]
ratchet = "strict"
"#;
    std::fs::write(&config_path, config_content).unwrap();

    // No CLI ratchet flag - should use config
    let args = make_check_args_with_ratchet(
        vec![temp_dir.path().to_path_buf()],
        Some(config_path),
        Some(baseline_path),
        None, // No CLI flag
    );

    let cli = make_cli_for_check(ColorChoice::Never, 0, true, false);

    let result = run_check_impl(&args, &cli);
    assert!(result.is_ok());
    // Should fail because config has ratchet = "strict" and baseline is stale
    assert_eq!(result.unwrap(), EXIT_THRESHOLD_EXCEEDED);
}
