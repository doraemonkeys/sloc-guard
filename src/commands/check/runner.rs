use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use crate::analyzer::generate_split_suggestions;
use crate::baseline::Baseline;
use crate::cache::{Cache, compute_config_hash};
use crate::checker::CheckResult;
use crate::cli::{CheckArgs, Cli};
use crate::config::collect_expired_rules;
use crate::git::GitContext;
use crate::output::{
    OutputFormat, ProjectStatistics, ScanProgress, StatsFormatter, StatsJsonFormatter,
};
use crate::state;
use crate::stats::TrendHistory;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::check_baseline_ops::{
    apply_baseline_comparison, check_baseline_ratchet, load_baseline, load_baseline_optional,
    tighten_baseline, update_baseline_from_results,
};
use super::check_git_diff::filter_by_git_diff;
use super::check_output::{format_output, structure_violation_to_check_result};
use super::check_processing::process_file_for_check;
use crate::commands::context::{
    CheckContext, color_choice_to_mode, load_cache, load_config, print_preset_info,
    resolve_scan_paths, save_cache, write_output,
};

/// Options for running a check with injected context.
///
/// Encapsulates all parameters needed by `run_check_with_context` to improve
/// readability and maintainability.
pub struct CheckOptions<'a> {
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
            crate::output::print_error_full(
                e.error_type(),
                &e.message(),
                e.detail().as_deref(),
                None,
            );
            EXIT_CONFIG_ERROR
        }
    }
}

pub fn run_check_impl(args: &CheckArgs, cli: &Cli) -> crate::Result<i32> {
    // 0. Validate structure params require explicit path
    let paths = validate_and_resolve_paths(args)?;

    // 0.1 Discover project root for consistent state file resolution
    let project_root = state::discover_project_root(Path::new("."));

    // 1. Load configuration
    let load_result = load_config(
        args.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        cli.offline,
    )?;
    let mut config = load_result.config;

    // 1.1 Print preset info if a preset was used
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_info(preset_name);
    }

    // 2. Apply CLI argument overrides
    apply_cli_overrides(&mut config, args);

    // 2.1 Check for expired rules and emit warnings
    let expired_rules = collect_expired_rules(&config);
    for expired in &expired_rules {
        let reason_suffix = expired
            .reason
            .as_ref()
            .map_or(String::new(), |r| format!(" (reason: {r})"));
        crate::output::print_warning_full(
            &format!(
                "{}.rules[{}] (pattern: '{}') expired on {}{}",
                expired.rule_type, expired.index, expired.pattern, expired.expires, reason_suffix
            ),
            None,
            Some("Update the expiration date or remove the rule"),
        );
    }

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
pub fn run_check_with_context(opts: &CheckOptions<'_>) -> crate::Result<i32> {
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
        let paths_to_scan = resolve_scan_paths(paths, &args.include);

        // 2. Scan directories using unified traversal (collects files + dir stats in one pass)
        let scan_result = ctx
            .scanner
            .scan_all_with_structure(&paths_to_scan, ctx.structure_scan_config.as_ref())?;

        // 2.1 Filter by git diff if --diff or --staged is specified
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
    // Clone is required because `tighten_baseline()` needs `&mut Baseline` for auto-update mode,
    // while the original `baseline` in CheckOptions is a shared reference (`Option<&Baseline>`).
    let mut baseline_for_ratchet = baseline.cloned();
    if let Some(ref baseline) = baseline_for_ratchet {
        apply_baseline_comparison(&mut results, baseline);
    }

    // 7.0.1 Check baseline ratchet (violations should only decrease)
    let ratchet_failed = handle_baseline_ratchet(
        args,
        config,
        &results,
        &mut baseline_for_ratchet,
        project_root,
        cli.quiet,
    )?;

    // 7.0.2 Update baseline if --update-baseline is specified
    if let Some(mode) = args.update_baseline {
        let baseline_path = args
            .baseline
            .clone()
            .unwrap_or_else(|| state::baseline_path(project_root));
        update_baseline_from_results(
            &results,
            mode,
            &baseline_path,
            baseline_for_ratchet.as_ref(),
        )?;
    }

    // 7.1 Generate split suggestions for failed files if --suggest is enabled
    if args.suggest {
        generate_split_suggestions(&mut results, &ctx.registry);
    }

    // 7.2 Build project statistics for report-json, HTML charts, or auto-snapshot
    let auto_snapshot_enabled = config.trend.auto_snapshot_on_check == Some(true);
    let needs_stats =
        args.report_json.is_some() || args.format == OutputFormat::Html || auto_snapshot_enabled;
    let project_stats = if needs_stats {
        Some(ProjectStatistics::new(file_stats).with_language_breakdown())
    } else {
        None
    };

    // 7.3 Write stats JSON if --report-json is specified
    if let Some(ref report_path) = args.report_json
        && let Some(ref stats) = project_stats
    {
        let stats_json = StatsJsonFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats)?;
        write_output(Some(report_path), &stats_json, cli.quiet)?;
    }

    // 8. Format output
    let color_mode = color_choice_to_mode(cli.color);
    let output = format_output(
        args.format,
        &results,
        color_mode,
        cli.verbose,
        args.suggest,
        project_stats.clone(),
        Some(project_root.to_path_buf()),
    )?;

    // 9. Write output
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    // 9.1 Write additional format outputs (single-run multi-format for CI efficiency)
    write_additional_formats(
        args,
        &results,
        color_mode,
        project_stats.clone(),
        project_root,
        cli,
    )?;

    // 10. Determine exit code (strict: CLI flag takes precedence, otherwise use config)
    let strict = args.strict || config.content.strict;
    let exit_code = determine_exit_code(&results, args.warn_only, strict, ratchet_failed);

    // 11. Auto-snapshot on successful check if enabled
    if exit_code == EXIT_SUCCESS
        && auto_snapshot_enabled
        && let Some(ref stats) = project_stats
    {
        perform_auto_snapshot(stats, config, project_root, cli.quiet, cli.verbose);
    }

    Ok(exit_code)
}

/// Determine exit code based on results and mode flags.
fn determine_exit_code(
    results: &[CheckResult],
    warn_only: bool,
    strict: bool,
    ratchet_failed: bool,
) -> i32 {
    if warn_only {
        return EXIT_SUCCESS;
    }
    let has_failures = results.iter().any(CheckResult::is_failed);
    let has_warnings = results.iter().any(CheckResult::is_warning);
    if has_failures || (strict && has_warnings) || ratchet_failed {
        EXIT_THRESHOLD_EXCEEDED
    } else {
        EXIT_SUCCESS
    }
}

/// Handle baseline ratchet enforcement.
///
/// Returns `true` if ratchet check failed (for strict mode exit code).
fn handle_baseline_ratchet(
    args: &CheckArgs,
    config: &crate::config::Config,
    results: &[CheckResult],
    baseline: &mut Option<Baseline>,
    project_root: &Path,
    quiet: bool,
) -> crate::Result<bool> {
    use crate::config::RatchetMode;

    // Determine effective ratchet mode: CLI takes precedence over config
    let ratchet_mode: Option<RatchetMode> = match (&args.ratchet, &config.baseline.ratchet) {
        (Some(cli_mode), _) => Some((*cli_mode).into()),
        (None, config_mode) => *config_mode,
    };

    // If no ratchet mode is set, skip ratchet check
    let Some(mode) = ratchet_mode else {
        return Ok(false);
    };

    // Ratchet requires a baseline
    let Some(current_baseline) = baseline else {
        // If ratchet is enabled but no baseline exists, emit warning
        if !quiet {
            crate::output::print_warning_full(
                "--ratchet specified but no baseline found",
                None,
                Some("Use --baseline to specify a baseline file"),
            );
        }
        return Ok(false);
    };

    // Check for stale entries
    let ratchet_result = check_baseline_ratchet(results, current_baseline);

    if !ratchet_result.is_outdated() {
        return Ok(false);
    }

    let baseline_path = args
        .baseline
        .clone()
        .unwrap_or_else(|| state::baseline_path(project_root));

    match mode {
        RatchetMode::Warn => {
            if !quiet {
                let paths_str = ratchet_result
                    .stale_paths
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                crate::output::print_warning_full(
                    &format!(
                        "Baseline can be tightened - {} violation(s) resolved",
                        ratchet_result.stale_entries
                    ),
                    Some(&paths_str),
                    Some("Run with --ratchet=auto to auto-update, or --update-baseline to replace"),
                );
            }
            Ok(false)
        }
        RatchetMode::Auto => {
            tighten_baseline(
                current_baseline,
                &ratchet_result.stale_paths,
                &baseline_path,
            )?;
            if !quiet {
                eprintln!(
                    "Baseline tightened: {} stale entry/entries removed.",
                    ratchet_result.stale_entries
                );
            }
            Ok(false)
        }
        RatchetMode::Strict => {
            let paths_str = ratchet_result
                .stale_paths
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            crate::output::print_error_full(
                "Baseline",
                &format!(
                    "outdated - {} violation(s) resolved but not removed",
                    ratchet_result.stale_entries
                ),
                Some(&paths_str),
                Some("Update the baseline with --update-baseline or use --ratchet=auto"),
            );
            Ok(true) // Signal failure
        }
    }
}

/// Validate structure params require explicit path and return resolved paths.
///
/// - If `--max-files`, `--max-dirs`, or `--max-depth` is specified, paths must be explicitly provided
/// - If no paths are provided and no structure params, defaults to current directory
pub fn validate_and_resolve_paths(args: &CheckArgs) -> crate::Result<Vec<PathBuf>> {
    let has_structure_params =
        args.max_files.is_some() || args.max_dirs.is_some() || args.max_depth.is_some();

    if args.paths.is_empty() {
        if has_structure_params {
            return Err(crate::SlocGuardError::Config(
                "--max-files/--max-dirs/--max-depth require a target <PATH>".to_string(),
            ));
        }
        // Default to current directory when no paths and no structure params
        Ok(vec![PathBuf::from(".")])
    } else {
        Ok(args.paths.clone())
    }
}

pub const fn apply_cli_overrides(config: &mut crate::config::Config, args: &CheckArgs) {
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

/// Perform auto-snapshot after a successful check.
///
/// Records current statistics to trend history, respecting retention policies.
/// Skips (with verbose log) if:
/// - `min_interval_secs` hasn't elapsed since last entry
/// - History file cannot be written (logs warning instead of failing)
fn perform_auto_snapshot(
    project_stats: &ProjectStatistics,
    config: &crate::config::Config,
    project_root: &Path,
    quiet: bool,
    verbose: u8,
) {
    // Get git context for the snapshot
    let git_context = GitContext::from_path(project_root);

    // Load history
    let history_path = state::history_path(project_root);
    let mut history = TrendHistory::load_or_default(&history_path);

    // Check if we should add (respects min_interval_secs)
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_secs();

    if !history.should_add(&config.trend, current_time) {
        if verbose > 0 && !quiet {
            eprintln!(
                "Skipping auto-snapshot: min_interval_secs ({}) not elapsed since last entry",
                config.trend.min_interval_secs.unwrap_or(0)
            );
        }
        return;
    }

    // Add entry with git context
    history.add_with_context(project_stats, git_context.as_ref());

    // Save with retention policy applied
    if let Err(e) = history.save_with_retention(&history_path, &config.trend) {
        // Log warning but don't fail the check
        if !quiet {
            crate::output::print_warning_full(
                "Auto-snapshot failed to save",
                Some(&format!("{}: {e}", history_path.display())),
                None,
            );
        }
        return;
    }

    if !quiet {
        eprintln!("Auto-snapshot recorded to {}", history_path.display());
    }
}

/// Write additional format outputs for single-run multi-format CI efficiency.
///
/// Supports `--write-sarif` and `--write-json` flags that write extra output files
/// while the primary `--format` output goes to stdout.
fn write_additional_formats(
    args: &CheckArgs,
    results: &[CheckResult],
    color_mode: crate::output::ColorMode,
    project_stats: Option<ProjectStatistics>,
    project_root: &Path,
    cli: &Cli,
) -> crate::Result<()> {
    if let Some(ref sarif_path) = args.write_sarif {
        let sarif_output = format_output(
            OutputFormat::Sarif,
            results,
            color_mode,
            cli.verbose,
            args.suggest,
            project_stats.clone(),
            Some(project_root.to_path_buf()),
        )?;
        write_output(Some(sarif_path), &sarif_output, cli.quiet)?;
    }

    if let Some(ref json_path) = args.write_json {
        let json_output = format_output(
            OutputFormat::Json,
            results,
            color_mode,
            cli.verbose,
            args.suggest,
            project_stats,
            Some(project_root.to_path_buf()),
        )?;
        write_output(Some(json_path), &json_output, cli.quiet)?;
    }

    Ok(())
}
