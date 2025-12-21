use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use crate::analyzer::generate_split_suggestions;
use crate::baseline::Baseline;
use crate::cache::{Cache, compute_config_hash};
use crate::checker::{
    CheckResult, Checker, DirStats, StructureViolation, ThresholdChecker, ViolationType,
};
use crate::cli::{CheckArgs, Cli};
use crate::config::{ContentOverride, StructureOverride};
use crate::counter::LineStats;
use crate::git::GitDiff;
use crate::language::LanguageRegistry;
use crate::output::{
    FileStatistics, HtmlFormatter, JsonFormatter, MarkdownFormatter, OutputFormat, OutputFormatter,
    ProjectStatistics, SarifFormatter, ScanProgress, StatsFormatter, StatsJsonFormatter,
    TextFormatter,
};
use crate::path_utils::path_matches_override;
use crate::state;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::check_baseline_ops::{
    apply_baseline_comparison, load_baseline, load_baseline_optional, update_baseline_from_results,
};
use super::context::{
    CheckContext, FileReader, color_choice_to_mode, load_cache, load_config,
    process_file_with_cache, resolve_scan_paths, save_cache, write_output,
};

/// Options for running a check with injected context.
///
/// Encapsulates all parameters needed by `run_check_with_context` to improve
/// readability and maintainability.
pub(crate) struct CheckOptions<'a> {
    pub args: &'a CheckArgs,
    pub cli: &'a Cli,
    pub paths: &'a [PathBuf],
    pub config: &'a crate::config::Config,
    pub ctx: &'a CheckContext,
    pub cache: &'a Mutex<Cache>,
    pub baseline: Option<&'a Baseline>,
    pub project_root: &'a Path,
}

#[must_use]
pub fn run_check(args: &CheckArgs, cli: &Cli) -> i32 {
    match run_check_impl(args, cli) {
        Ok(exit_code) => exit_code,
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

pub(crate) fn run_check_impl(args: &CheckArgs, cli: &Cli) -> crate::Result<i32> {
    // 0. Validate structure params require explicit path
    let paths = validate_and_resolve_paths(args)?;

    // 0.1 Discover project root for consistent state file resolution
    let project_root = state::discover_project_root(Path::new("."));

    // 1. Load configuration
    let mut config = load_config(
        args.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        cli.offline,
    )?;

    // 2. Apply CLI argument overrides
    apply_cli_overrides(&mut config, args);

    // 3. Load baseline if specified (allow non-existent if update-baseline is specified)
    let baseline = if args.update_baseline.is_some() {
        // When updating baseline, allow non-existent file
        load_baseline_optional(args.baseline.as_deref())?
    } else {
        load_baseline(args.baseline.as_deref())?
    };

    // 3.1 Load cache if not disabled
    let cache_path = state::cache_path(&project_root);
    let config_hash = compute_config_hash(&config);
    let cache = if args.no_cache {
        None
    } else {
        load_cache(&cache_path, &config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // Apply CLI extensions override to config.content if provided
    if let Some(ref cli_extensions) = args.ext {
        config.content.extensions.clone_from(cli_extensions);
    }

    // 4. Build check context with dependencies
    let warn_threshold = args.warn_threshold.unwrap_or(config.content.warn_threshold);
    let mut exclude_patterns = config.scanner.exclude.clone();
    exclude_patterns.extend(args.exclude.clone());
    let use_gitignore = config.scanner.gitignore && !args.no_gitignore;
    let ctx = CheckContext::from_config(&config, warn_threshold, exclude_patterns, use_gitignore)?;

    // 5. Run check with context
    let options = CheckOptions {
        args,
        cli,
        paths: &paths,
        config: &config,
        ctx: &ctx,
        cache: &cache,
        baseline: baseline.as_ref(),
        project_root: &project_root,
    };
    run_check_with_context(&options)
}

/// Internal implementation accepting injectable context (for testing).
///
/// This function contains the core check logic and accepts pre-built dependencies,
/// enabling unit testing with custom/mock components.
pub(crate) fn run_check_with_context(opts: &CheckOptions<'_>) -> crate::Result<i32> {
    // Destructure options for convenient access
    let args = opts.args;
    let cli = opts.cli;
    let paths = opts.paths;
    let config = opts.config;
    let ctx = opts.ctx;
    let cache = opts.cache;
    let baseline = opts.baseline;
    let project_root = opts.project_root;
    // Check if pure incremental mode (--files provided)
    let (all_files, scan_result, skip_structure_checks) = if args.files.is_empty() {
        // Normal mode: scan directories
        // 1. Determine paths to scan
        let paths_to_scan = resolve_scan_paths(paths, &args.include, config);

        // 2. Scan directories using unified traversal (collects files + dir stats in one pass)
        let scan_result = ctx
            .scanner
            .scan_all_with_structure(&paths_to_scan, ctx.structure_scan_config.as_ref())?;

        // 2.1 Validate override paths against scanned files/directories
        validate_override_paths(
            &config.content.overrides,
            &config.structure.overrides,
            &scan_result.files,
            &scan_result.dir_stats,
        )?;

        // 2.2 Filter by git diff if --diff or --staged is specified
        let files = filter_by_git_diff(
            scan_result.files.clone(),
            args.diff.as_deref(),
            args.staged,
            project_root,
        )?;
        (files, Some(scan_result), false)
    } else {
        // Pure incremental mode: process only listed files, skip structure checks
        let files: Vec<_> = args.files.iter().filter(|f| f.exists()).cloned().collect();
        (files, None, true)
    };

    // 3. Process each file (parallel with rayon) using injected context
    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let processed: Vec<_> = all_files
        .par_iter()
        .filter(|file_path| ctx.threshold_checker.should_process(file_path)) // Filter by extension
        .filter_map(|file_path| {
            let result = process_file_for_check(
                file_path,
                &ctx.registry,
                &ctx.threshold_checker,
                cache,
                ctx.file_reader.as_ref(),
            );
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    // Separate check results and file statistics
    let (mut results, file_stats): (Vec<_>, Vec<_>) = processed.into_iter().unzip();

    // 5. Run structure checks if enabled (using pre-collected dir_stats from unified scan)
    // Skip in pure incremental mode (--files) since no directory scan was performed
    if !skip_structure_checks
        && let Some(ref scan_result) = scan_result
        && let Some(ref structure_checker) = ctx.structure_checker
        && structure_checker.is_enabled()
    {
        // Use dir_stats collected during unified scan
        let violations = structure_checker.check(&scan_result.dir_stats);
        let structure_results: Vec<_> = violations
            .iter()
            .map(structure_violation_to_check_result)
            .collect();
        results.extend(structure_results);

        // Add allowlist violations collected during scan
        let allowlist_results: Vec<_> = scan_result
            .allowlist_violations
            .iter()
            .map(structure_violation_to_check_result)
            .collect();
        results.extend(allowlist_results);

        // Check for missing sibling files (co-location enforcement)
        let sibling_violations = structure_checker.check_siblings(&scan_result.files);
        let sibling_results: Vec<_> = sibling_violations
            .iter()
            .map(structure_violation_to_check_result)
            .collect();
        results.extend(sibling_results);
    }

    // 6. Save cache if not disabled
    if !args.no_cache
        && let Ok(cache_guard) = cache.lock()
    {
        let cache_path = state::cache_path(project_root);
        save_cache(&cache_path, &cache_guard);
    }

    // 7. Apply baseline comparison: mark failures as grandfathered if in baseline
    if let Some(baseline) = baseline {
        apply_baseline_comparison(&mut results, baseline);
    }

    // 7.0.1 Update baseline if --update-baseline is specified
    if let Some(mode) = args.update_baseline {
        let baseline_path = args
            .baseline
            .clone()
            .unwrap_or_else(|| state::baseline_path(project_root));
        update_baseline_from_results(&results, mode, &baseline_path, baseline)?;
    }

    // 7.1 Generate split suggestions for failed files if --suggest is enabled
    if args.suggest {
        generate_split_suggestions(&mut results, &ctx.registry);
    }

    // 7.2 Write stats JSON if --report-json is specified
    if let Some(ref report_path) = args.report_json {
        let stats = ProjectStatistics::new(file_stats).with_language_breakdown();
        let stats_json = StatsJsonFormatter.format(&stats)?;
        write_output(Some(report_path), &stats_json, cli.quiet)?;
    }

    // 8. Format output
    let color_mode = color_choice_to_mode(cli.color);
    let output = format_output(args.format, &results, color_mode, cli.verbose, args.suggest)?;

    // 9. Write output
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    // 10. Determine exit code
    let has_failures = results.iter().any(CheckResult::is_failed);
    let has_warnings = results.iter().any(CheckResult::is_warning);

    if args.warn_only {
        return Ok(EXIT_SUCCESS);
    }

    // Strict mode: CLI flag takes precedence, otherwise use config
    let strict = args.strict || config.content.strict;

    if has_failures || (strict && has_warnings) {
        Ok(EXIT_THRESHOLD_EXCEEDED)
    } else {
        Ok(EXIT_SUCCESS)
    }
}

/// Validate structure params require explicit path and return resolved paths.
///
/// - If `--max-files`, `--max-dirs`, or `--max-depth` is specified, paths must be explicitly provided
/// - If no paths are provided and no structure params, defaults to current directory
fn validate_and_resolve_paths(args: &CheckArgs) -> crate::Result<Vec<std::path::PathBuf>> {
    let has_structure_params =
        args.max_files.is_some() || args.max_dirs.is_some() || args.max_depth.is_some();

    if args.paths.is_empty() {
        if has_structure_params {
            return Err(crate::SlocGuardError::Config(
                "--max-files/--max-dirs/--max-depth require a target <PATH>".to_string(),
            ));
        }
        // Default to current directory when no paths and no structure params
        Ok(vec![std::path::PathBuf::from(".")])
    } else {
        Ok(args.paths.clone())
    }
}

pub(crate) const fn apply_cli_overrides(config: &mut crate::config::Config, args: &CheckArgs) {
    if let Some(max_lines) = args.max_lines {
        config.content.max_lines = max_lines;
    }

    if args.count_comments {
        config.content.skip_comments = false;
    }

    if args.count_blank {
        config.content.skip_blank = false;
    }

    if let Some(warn_threshold) = args.warn_threshold {
        config.content.warn_threshold = warn_threshold;
    }

    // Apply CLI structure overrides (override defaults, not rules)
    if let Some(max_files) = args.max_files {
        config.structure.max_files = Some(max_files);
    }

    if let Some(max_dirs) = args.max_dirs {
        config.structure.max_dirs = Some(max_dirs);
    }

    if let Some(max_depth) = args.max_depth {
        config.structure.max_depth = Some(max_depth);
    }
}

/// Represents a parsed diff range (base..target).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiffRange {
    pub base: String,
    pub target: String,
}

/// Parse a diff reference string into a `DiffRange`.
///
/// Supports:
/// - `ref` → base=ref, target=HEAD
/// - `base..target` → base=base, target=target
/// - `base..` → base=base, target=HEAD
///
/// # Errors
/// Returns an error if:
/// - Input starts with `..` (no base specified)
/// - Input is empty
pub(crate) fn parse_diff_range(diff_ref: &str) -> crate::Result<DiffRange> {
    if diff_ref.is_empty() {
        return Err(crate::SlocGuardError::Config(
            "--diff requires a git reference".to_string(),
        ));
    }

    // Check for range syntax (contains "..")
    if let Some(pos) = diff_ref.find("..") {
        let base = &diff_ref[..pos];
        let target = &diff_ref[pos + 2..];

        // Error if no base specified
        if base.is_empty() {
            return Err(crate::SlocGuardError::Config(
                "--diff range requires a base reference (e.g., 'main..feature', not '..feature')"
                    .to_string(),
            ));
        }

        // If target is empty, default to HEAD
        let target = if target.is_empty() {
            "HEAD".to_string()
        } else {
            target.to_string()
        };

        Ok(DiffRange {
            base: base.to_string(),
            target,
        })
    } else {
        // Single reference: compare to HEAD
        Ok(DiffRange {
            base: diff_ref.to_string(),
            target: "HEAD".to_string(),
        })
    }
}

fn filter_by_git_diff(
    files: Vec<std::path::PathBuf>,
    diff_ref: Option<&str>,
    staged_only: bool,
    project_root: &Path,
) -> crate::Result<Vec<std::path::PathBuf>> {
    if !staged_only && diff_ref.is_none() {
        return Ok(files);
    }

    // Discover git repository from project root
    let git_diff = GitDiff::discover(project_root)?;
    let changed_files = if staged_only {
        git_diff.get_staged_files()?
    } else {
        let range = parse_diff_range(diff_ref.expect("diff_ref checked above"))?;
        git_diff.get_changed_files_range(&range.base, &range.target)?
    };

    // Canonicalize paths for comparison
    let changed_canonical: std::collections::HashSet<_> = changed_files
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();

    // Filter to only include changed files
    let filtered: Vec<_> = files
        .into_iter()
        .filter(|f| {
            f.canonicalize()
                .ok()
                .is_some_and(|canon| changed_canonical.contains(&canon))
        })
        .collect();

    Ok(filtered)
}

pub(crate) fn process_file_for_check(
    file_path: &Path,
    registry: &LanguageRegistry,
    checker: &ThresholdChecker,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> Option<(CheckResult, FileStatistics)> {
    let (stats, language) = process_file_with_cache(file_path, registry, cache, reader)?;
    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(file_path);
    let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);
    let check_result = checker.check(file_path, &effective_stats);
    let file_stats = FileStatistics {
        path: file_path.to_path_buf(),
        stats,
        language,
    };
    Some((check_result, file_stats))
}

#[must_use]
pub(crate) fn compute_effective_stats(
    stats: &LineStats,
    skip_comments: bool,
    skip_blank: bool,
) -> LineStats {
    let mut effective = stats.clone();

    // If not skipping comments, add them to code count
    if !skip_comments {
        effective.code += effective.comment;
        effective.comment = 0;
    }

    // If not skipping blanks, add them to code count
    if !skip_blank {
        effective.code += effective.blank;
        effective.blank = 0;
    }

    effective
}

/// Convert a structure violation to a check result for unified output.
fn structure_violation_to_check_result(violation: &StructureViolation) -> CheckResult {
    // Create synthetic LineStats representing the violation
    // We use 'code' to represent the actual count for display purposes
    let stats = LineStats {
        total: violation.actual,
        code: violation.actual,
        comment: 0,
        blank: 0,
        ignored: 0,
    };

    let override_reason = match &violation.violation_type {
        ViolationType::FileCount => Some("structure: files count exceeded".to_string()),
        ViolationType::DirCount => Some("structure: subdirs count exceeded".to_string()),
        ViolationType::MaxDepth => Some("structure: depth count exceeded".to_string()),
        ViolationType::DisallowedFile => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            Some(format!("structure: disallowed file (rule: {rule})"))
        }
        ViolationType::NamingConvention { expected_pattern } => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            Some(format!(
                "structure: naming convention violation (expected: {expected_pattern}, rule: {rule})"
            ))
        }
        ViolationType::MissingSibling {
            expected_sibling_pattern,
        } => {
            let rule = violation
                .triggering_rule_pattern
                .as_deref()
                .unwrap_or("unknown");
            Some(format!(
                "structure: missing sibling (expected: {expected_sibling_pattern}, rule: {rule})"
            ))
        }
    };

    if violation.is_warning {
        CheckResult::Warning {
            path: violation.path.clone(),
            stats,
            limit: violation.limit,
            override_reason,
            suggestions: None,
        }
    } else {
        CheckResult::Failed {
            path: violation.path.clone(),
            stats,
            limit: violation.limit,
            override_reason,
            suggestions: None,
        }
    }
}

pub(crate) fn format_output(
    format: OutputFormat,
    results: &[CheckResult],
    color_mode: crate::output::ColorMode,
    verbose: u8,
    show_suggestions: bool,
) -> crate::Result<String> {
    match format {
        OutputFormat::Text => TextFormatter::with_verbose(color_mode, verbose)
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Json => JsonFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Sarif => SarifFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Markdown => MarkdownFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
        OutputFormat::Html => HtmlFormatter::new()
            .with_suggestions(show_suggestions)
            .format(results),
    }
}

/// Validate that override paths are correctly configured.
///
/// - `ContentOverride` paths must point to files, not directories
/// - `StructureOverride` paths must point to directories, not files
///
/// Returns an error if any override path is misconfigured.
pub(crate) fn validate_override_paths(
    content_overrides: &[ContentOverride],
    structure_overrides: &[StructureOverride],
    files: &[PathBuf],
    directories: &HashMap<PathBuf, DirStats>,
) -> crate::Result<()> {
    // Check ContentOverrides don't match directories
    for (i, ovr) in content_overrides.iter().enumerate() {
        for dir_path in directories.keys() {
            if path_matches_override(dir_path, &ovr.path) {
                return Err(crate::SlocGuardError::Config(format!(
                    "content.override[{}] path '{}' matches directory '{}', \
                     but content overrides only apply to files. \
                     Use [[structure.override]] for directory overrides.",
                    i,
                    ovr.path,
                    dir_path.display()
                )));
            }
        }
    }

    // Check StructureOverrides don't match files
    for (i, ovr) in structure_overrides.iter().enumerate() {
        for file_path in files {
            if path_matches_override(file_path, &ovr.path) {
                return Err(crate::SlocGuardError::Config(format!(
                    "structure.override[{}] path '{}' matches file '{}', \
                     but structure overrides only apply to directories. \
                     Use [[content.override]] for file overrides.",
                    i,
                    ovr.path,
                    file_path.display()
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "check_run_tests.rs"]
mod run_tests;

#[cfg(test)]
#[path = "check_conversion_tests.rs"]
mod conversion_tests;

#[cfg(test)]
#[path = "check_context_structure_tests.rs"]
mod context_structure_tests;
