use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::analyzer::generate_split_suggestions;
use crate::baseline::{Baseline, StructureViolationType, compute_file_hash};
use crate::cache::{Cache, compute_config_hash};
use crate::checker::{CheckResult, Checker, StructureViolation, ThresholdChecker, ViolationType};
use crate::cli::{BaselineUpdateMode, CheckArgs, Cli};
use crate::counter::LineStats;
use crate::git::{ChangedFiles, GitDiff};
use crate::language::LanguageRegistry;
use crate::output::{
    FileStatistics, HtmlFormatter, JsonFormatter, MarkdownFormatter, OutputFormat, OutputFormatter,
    ProjectStatistics, SarifFormatter, ScanProgress, StatsFormatter, StatsJsonFormatter,
    TextFormatter,
};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::context::{
    CheckContext, FileReader, color_choice_to_mode, load_cache, load_config, process_file_with_cache,
    resolve_scan_paths, save_cache, write_output,
};

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

    // 1. Load configuration
    let mut config = load_config(args.config.as_deref(), cli.no_config, cli.no_extends)?;

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
    let config_hash = compute_config_hash(&config);
    let cache = if args.no_cache {
        None
    } else {
        load_cache(&config_hash)
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
    run_check_with_context(args, cli, &paths, &config, &ctx, &cache, baseline.as_ref())
}

/// Internal implementation accepting injectable context (for testing).
///
/// This function contains the core check logic and accepts pre-built dependencies,
/// enabling unit testing with custom/mock components.
pub(crate) fn run_check_with_context(
    args: &CheckArgs,
    cli: &Cli,
    paths: &[std::path::PathBuf],
    config: &crate::config::Config,
    ctx: &CheckContext,
    cache: &Mutex<Cache>,
    baseline: Option<&Baseline>,
) -> crate::Result<i32> {
    // 1. Determine paths to scan
    let paths_to_scan = resolve_scan_paths(paths, &args.include, config);

    // 2. Scan directories using injected scanner (respects .gitignore and exclude patterns)
    let all_files = ctx.scanner.scan_all(&paths_to_scan)?;

    // 2.1 Filter by git diff if --diff is specified
    let all_files = filter_by_git_diff(all_files, args.diff.as_deref())?;

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

    // 5. Run structure checks if enabled (using injected structure_checker)
    if let Some(ref structure_checker) = ctx.structure_checker
        && structure_checker.is_enabled()
    {
        for path in &paths_to_scan {
            if path.is_dir() {
                let violations = structure_checker.check_directory(path)?;
                let structure_results: Vec<_> = violations
                    .iter()
                    .map(structure_violation_to_check_result)
                    .collect();
                results.extend(structure_results);
            }
        }
    }

    // 6. Save cache if not disabled
    if !args.no_cache
        && let Ok(cache_guard) = cache.lock()
    {
        save_cache(&cache_guard);
    }

    // 7. Apply baseline comparison: mark failures as grandfathered if in baseline
    if let Some(baseline) = baseline {
        apply_baseline_comparison(&mut results, baseline);
    }

    // 7.0.1 Update baseline if --update-baseline is specified
    if let Some(mode) = args.update_baseline {
        let baseline_path = args
            .baseline
            .as_deref()
            .unwrap_or_else(|| Path::new(".sloc-guard-baseline.json"));
        update_baseline_from_results(&results, mode, baseline_path, baseline)?;
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

pub(crate) fn load_baseline(baseline_path: Option<&Path>) -> crate::Result<Option<Baseline>> {
    let Some(path) = baseline_path else {
        return Ok(None);
    };

    if !path.exists() {
        return Err(crate::SlocGuardError::Config(format!(
            "Baseline file not found: {}",
            path.display()
        )));
    }

    Ok(Some(Baseline::load(path)?))
}

/// Load baseline optionally - returns None if file doesn't exist (for update-baseline mode).
pub(crate) fn load_baseline_optional(
    baseline_path: Option<&Path>,
) -> crate::Result<Option<Baseline>> {
    let Some(path) = baseline_path else {
        return Ok(None);
    };

    if !path.exists() {
        return Ok(None);
    }

    Ok(Some(Baseline::load(path)?))
}

pub(crate) fn apply_baseline_comparison(results: &mut [CheckResult], baseline: &Baseline) {
    for result in results.iter_mut() {
        if !result.is_failed() {
            continue;
        }

        let path_str = result.path().to_string_lossy().replace('\\', "/");
        if baseline.contains(&path_str) {
            // Replace the result with its grandfathered version
            let owned = std::mem::replace(
                result,
                CheckResult::Passed {
                    path: std::path::PathBuf::new(),
                    stats: LineStats::default(),
                    limit: 0,
                    override_reason: None,
                },
            );
            *result = owned.into_grandfathered();
        }
    }
}

/// Update baseline file from check results based on the specified mode.
fn update_baseline_from_results(
    results: &[CheckResult],
    mode: BaselineUpdateMode,
    baseline_path: &Path,
    existing_baseline: Option<&Baseline>,
) -> crate::Result<()> {
    let mut new_baseline = match mode {
        BaselineUpdateMode::New => {
            // Start with existing baseline for add-only mode
            existing_baseline.cloned().unwrap_or_default()
        }
        _ => Baseline::new(),
    };

    for result in results {
        if !result.is_failed() {
            continue;
        }

        let path_str = result.path().to_string_lossy().replace('\\', "/");
        let is_structure = is_structure_violation(result.override_reason());

        // Apply mode filtering
        let should_include = match mode {
            BaselineUpdateMode::All => true,
            BaselineUpdateMode::Content => !is_structure,
            BaselineUpdateMode::Structure => is_structure,
            BaselineUpdateMode::New => {
                // In new mode, only add if not already in baseline
                !new_baseline.contains(&path_str)
            }
        };

        if !should_include {
            continue;
        }

        if is_structure {
            // Parse structure violation type from override_reason
            if let Some((vtype, count)) =
                parse_structure_violation(result.override_reason(), result.stats().code)
            {
                new_baseline.set_structure(&path_str, vtype, count);
            }
        } else {
            // Content violation - compute file hash
            let hash = compute_file_hash(result.path()).unwrap_or_default();
            new_baseline.set_content(&path_str, result.stats().code, hash);
        }
    }

    new_baseline.save(baseline_path)
}

/// Check if a check result represents a structure violation.
fn is_structure_violation(override_reason: Option<&str>) -> bool {
    override_reason.is_some_and(|r| r.starts_with("structure:"))
}

/// Parse structure violation type from `override_reason`.
/// Returns (`StructureViolationType`, count) if parseable.
fn parse_structure_violation(
    override_reason: Option<&str>,
    count: usize,
) -> Option<(StructureViolationType, usize)> {
    let reason = override_reason?;
    if !reason.starts_with("structure:") {
        return None;
    }

    let vtype = if reason.contains("files") {
        StructureViolationType::Files
    } else if reason.contains("subdirs") {
        StructureViolationType::Dirs
    } else {
        return None;
    };

    Some((vtype, count))
}

/// Validate structure params require explicit path and return resolved paths.
///
/// - If `--max-files` or `--max-dirs` is specified, paths must be explicitly provided
/// - If no paths are provided and no structure params, defaults to current directory
fn validate_and_resolve_paths(args: &CheckArgs) -> crate::Result<Vec<std::path::PathBuf>> {
    let has_structure_params = args.max_files.is_some() || args.max_dirs.is_some();

    if args.paths.is_empty() {
        if has_structure_params {
            return Err(crate::SlocGuardError::Config(
                "--max-files/--max-dirs require a target <PATH>".to_string(),
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
}

fn filter_by_git_diff(
    files: Vec<std::path::PathBuf>,
    diff_ref: Option<&str>,
) -> crate::Result<Vec<std::path::PathBuf>> {
    let Some(base_ref) = diff_ref else {
        return Ok(files);
    };

    // Discover git repository from current directory
    let git_diff = GitDiff::discover(Path::new("."))?;
    let changed_files = git_diff.get_changed_files(base_ref)?;

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

    let violation_label = match violation.violation_type {
        ViolationType::FileCount => "files",
        ViolationType::DirCount => "subdirs",
    };

    CheckResult::Failed {
        path: violation.path.clone(),
        stats,
        limit: violation.limit,
        override_reason: Some(format!("structure: {violation_label} count exceeded")),
        suggestions: None,
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

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "check_baseline_tests.rs"]
mod baseline_tests;

#[cfg(test)]
#[path = "check_context_structure_tests.rs"]
mod context_structure_tests;
