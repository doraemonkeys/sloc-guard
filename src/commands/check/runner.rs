use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::analyzer::generate_split_suggestions;
use crate::baseline::Baseline;
use crate::cache::{Cache, compute_config_hash};
use crate::checker::CheckResult;
use crate::cli::{CheckArgs, Cli};
use crate::config::{FetchPolicy, collect_expired_rules};
use crate::output::{
    OutputFormat, ProjectStatistics, ScanProgress, StatsFormatter, StatsJsonFormatter,
};
use crate::state;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::check_args::{apply_cli_overrides, validate_and_resolve_paths};
use super::check_baseline_ops::{
    apply_baseline_comparison, handle_baseline_ratchet, load_baseline, load_baseline_optional,
    update_baseline_from_results,
};
use super::check_exit::determine_exit_code;
use super::check_output::{
    format_output, structure_violation_to_check_result, write_additional_formats,
};
use super::check_processing::process_file_for_check;
use super::check_scan::{partition_file_results, scan_or_filter_files};
use super::check_snapshot::perform_auto_snapshot;
use crate::commands::context::{
    CheckContext, color_choice_to_mode, load_cache, load_config, print_preset_info, save_cache,
    write_output,
};
use crate::output::ColorMode;

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

/// Parameters for output writing operations.
#[derive(Debug)]
struct OutputParams<'a> {
    results: &'a [CheckResult],
    project_stats: Option<ProjectStatistics>,
    project_root: &'a Path,
    color_mode: ColorMode,
}

/// Write check outputs: stats JSON, main output, and additional formats.
///
/// Note: `write_output` only suppresses stdout; file writes always proceed regardless
/// of the quiet flag. This means `--report-json` and `--output` both always write to
/// their target files—the quiet flag only affects console output.
fn write_check_outputs(
    args: &CheckArgs,
    cli: &Cli,
    params: &OutputParams<'_>,
) -> crate::Result<()> {
    // Write stats JSON if --report-json is specified
    // (file writes always proceed; quiet only suppresses stdout)
    if let Some(ref report_path) = args.report_json
        && let Some(ref stats) = params.project_stats
    {
        let stats_json = StatsJsonFormatter::new()
            .with_project_root(Some(params.project_root.to_path_buf()))
            .format(stats)?;
        write_output(Some(report_path), &stats_json, cli.quiet)?;
    }

    // Format main output
    let output = format_output(
        args.format,
        params.results,
        params.color_mode,
        cli.verbose,
        args.suggest,
        params.project_stats.clone(),
        Some(params.project_root.to_path_buf()),
    )?;

    // Write main output with "suppress success, preserve failure" semantics for stdout.
    // File writes always proceed regardless of quiet flag.
    let has_issues = params.results.iter().any(CheckResult::is_issue);
    let quiet_for_stdout = cli.quiet && !has_issues;
    write_output(args.output.as_deref(), &output, quiet_for_stdout)?;

    // Write additional format outputs (single-run multi-format for CI efficiency)
    write_additional_formats(
        args,
        params.results,
        params.color_mode,
        params.project_stats.clone(),
        params.project_root,
        cli,
    )?;

    Ok(())
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
        FetchPolicy::from_cli(cli.extends_policy),
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
    let cache = if args.no_sloc_cache {
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

    // Scan or filter files based on mode
    let (all_files, scan_result, skip_structure_checks) =
        scan_or_filter_files(args, cli, paths, ctx, project_root)?;

    // Determine fail_fast mode from CLI or config
    let fail_fast = args.fail_fast || config.check.fail_fast;
    let failure_detected = AtomicBool::new(false);

    // 3. Process each file (parallel with rayon) using injected context
    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let file_results: Vec<_> = all_files
        .par_iter()
        .filter(|file_path| ctx.threshold_checker.should_process(file_path)) // Filter by extension
        .filter_map(|file_path| {
            // Early exit check for fail_fast mode
            if fail_fast && failure_detected.load(Ordering::Relaxed) {
                progress.inc();
                return None;
            }

            let result = process_file_for_check(
                file_path,
                &ctx.registry,
                &ctx.threshold_checker,
                cache,
                ctx.file_reader.as_ref(),
            );
            progress.inc();

            // Check if this result is a failure for fail_fast
            if fail_fast && result.is_failure() {
                failure_detected.store(true, Ordering::Relaxed);
            }

            Some(result)
        })
        .collect();
    progress.finish();

    // Separate successful results from errors
    let (mut results, file_stats, file_errors) = partition_file_results(file_results);

    // Report file processing errors (IO failures, lock errors) as warnings
    // These are critical path errors that could cause "missing files" in reports
    if !cli.quiet {
        for error in &file_errors {
            crate::output::print_warning(&format!("failed to process file: {error}"));
        }
    }

    // 4. Merge allowlist/denylist violations collected during scan
    //
    // These violations are produced by the structure-aware scanner based on:
    // - global allow/deny settings in [structure]
    // - per-rule allow/deny/naming settings in [[structure.rules]]
    //
    // They must be reported even when no directory count limits are configured.
    if !skip_structure_checks
        && let Some(ref scan_result) = scan_result
        && !scan_result.allowlist_violations.is_empty()
    {
        let allowlist_results: Vec<_> = scan_result
            .allowlist_violations
            .iter()
            .map(structure_violation_to_check_result)
            .collect();
        results.extend(allowlist_results);
    }

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

        // Check for missing sibling files (co-location enforcement)
        let sibling_violations = structure_checker.check_siblings(&scan_result.files);
        let sibling_results: Vec<_> = sibling_violations
            .iter()
            .map(structure_violation_to_check_result)
            .collect();
        results.extend(sibling_results);
    }

    // 6. Save cache if not disabled (errors are non-critical)
    if !args.no_sloc_cache
        && let Ok(cache_guard) = cache.lock()
    {
        let cache_path = state::cache_path(project_root);
        let _ = save_cache(&cache_path, &cache_guard);
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

    // 8. Write all outputs (stats JSON, main output, additional formats)
    let color_mode = color_choice_to_mode(cli.color);
    let output_params = OutputParams {
        results: &results,
        project_stats: project_stats.clone(),
        project_root,
        color_mode,
    };
    write_check_outputs(args, cli, &output_params)?;

    // 10. Determine exit code (CLI flags take precedence over config)
    let warnings_as_errors =
        args.warnings_as_errors || args.strict || config.check.warnings_as_errors;
    let exit_code =
        determine_exit_code(&results, args.warn_only, warnings_as_errors, ratchet_failed);

    // 11. Auto-snapshot on successful check if enabled
    if exit_code == EXIT_SUCCESS
        && auto_snapshot_enabled
        && let Some(ref stats) = project_stats
    {
        perform_auto_snapshot(stats, config, project_root, cli.quiet, cli.verbose);
    }

    Ok(exit_code)
}
