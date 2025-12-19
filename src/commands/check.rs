use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::analyzer::generate_split_suggestions;
use crate::baseline::Baseline;
use crate::cache::{Cache, compute_config_hash};
use crate::checker::{
    CheckResult, Checker, StructureChecker, StructureViolation, ThresholdChecker, ViolationType,
};
use crate::cli::{CheckArgs, Cli};
use crate::counter::LineStats;
use crate::git::{ChangedFiles, GitDiff};
use crate::language::LanguageRegistry;
use crate::output::{
    HtmlFormatter, JsonFormatter, MarkdownFormatter, OutputFormat, OutputFormatter, SarifFormatter,
    ScanProgress, TextFormatter,
};
use crate::scanner::scan_files;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

use super::context::{
    color_choice_to_mode, load_cache, load_config, process_file_with_cache, resolve_scan_paths,
    save_cache, write_output,
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
    // 1. Load configuration
    let mut config = load_config(args.config.as_deref(), cli.no_config, cli.no_extends)?;

    // 2. Apply CLI argument overrides
    apply_cli_overrides(&mut config, args);

    // 3. Load baseline if specified
    let baseline = load_baseline(args.baseline.as_deref())?;

    // 3.1 Load cache if not disabled
    let config_hash = compute_config_hash(&config);
    let cache = if args.no_cache {
        None
    } else {
        load_cache(&config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // 4. Prepare exclude patterns from scanner config
    let mut exclude_patterns = config.scanner.exclude.clone();
    exclude_patterns.extend(args.exclude.clone());

    // 5. Determine paths to scan
    let paths_to_scan = resolve_scan_paths(&args.paths, &args.include, &config);

    // 6. Scan directories (respecting .gitignore if enabled)
    // Scanner now returns ALL files, extension filtering is done by ThresholdChecker
    let use_gitignore = config.scanner.gitignore && !args.no_gitignore;
    let all_files = scan_files(&paths_to_scan, &exclude_patterns, use_gitignore)?;

    // 6.1 Filter by git diff if --diff is specified
    let all_files = filter_by_git_diff(all_files, args.diff.as_deref())?;

    // 7. Process each file (parallel with rayon)
    let registry = LanguageRegistry::with_custom_languages(&config.languages);

    // Apply CLI extensions override to config.content if provided
    if let Some(ref cli_extensions) = args.ext {
        config.content.extensions.clone_from(cli_extensions);
    }

    let warn_threshold = args.warn_threshold.unwrap_or(config.content.warn_threshold);
    let checker = ThresholdChecker::new(config.clone()).with_warning_threshold(warn_threshold);

    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let mut results: Vec<_> = all_files
        .par_iter()
        .filter(|file_path| checker.should_process(file_path)) // Filter by extension here
        .filter_map(|file_path| {
            let result = process_file_for_check(file_path, &registry, &checker, &cache);
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    // 7.2 Run structure checks if enabled
    let structure_checker = StructureChecker::new(&config.structure)?;
    if structure_checker.is_enabled() {
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

    // 7.3 Save cache if not disabled
    if !args.no_cache
        && let Ok(cache_guard) = cache.lock()
    {
        save_cache(&cache_guard);
    }

    // 8. Apply baseline comparison: mark failures as grandfathered if in baseline
    if let Some(ref baseline) = baseline {
        apply_baseline_comparison(&mut results, baseline);
    }

    // 8.1 Generate split suggestions for failed files if --fix is enabled
    if args.fix {
        generate_split_suggestions(&mut results, &registry);
    }

    // 9. Format output
    let color_mode = color_choice_to_mode(cli.color);
    let output = format_output(args.format, &results, color_mode, cli.verbose, args.fix)?;

    // 10. Write output
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    // 11. Determine exit code
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

pub(crate) const fn apply_cli_overrides(config: &mut crate::config::Config, args: &CheckArgs) {
    if let Some(max_lines) = args.max_lines {
        config.content.max_lines = max_lines;
    }

    if args.no_skip_comments {
        config.content.skip_comments = false;
    }

    if args.no_skip_blank {
        config.content.skip_blank = false;
    }

    if let Some(warn_threshold) = args.warn_threshold {
        config.content.warn_threshold = warn_threshold;
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
) -> Option<CheckResult> {
    let (stats, _language) = process_file_with_cache(file_path, registry, cache)?;
    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(file_path);
    let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);
    Some(checker.check(file_path, &effective_stats))
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
