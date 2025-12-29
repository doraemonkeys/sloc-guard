use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{Cache, compute_config_hash};
use crate::cli::{
    BreakdownArgs, BreakdownBy, Cli, CommonStatsArgs, FileSortOrder as CliFileSortOrder, FilesArgs,
    HistoryArgs, HistoryOutputFormat, ReportArgs, ReportOutputFormat, StatsAction, StatsArgs,
    StatsOutputFormat, SummaryArgs, TrendArgs,
};
use crate::language::LanguageRegistry;
use crate::output::{
    ColorMode, FileSortOrder, FileStatistics, ProjectStatistics, ScanProgress, StatsFormatter,
    StatsHtmlFormatter, StatsJsonFormatter, StatsMarkdownFormatter, StatsTextFormatter,
};
use crate::scanner::scan_files;
use crate::state;
use crate::stats::{TrendEntry, TrendHistory, parse_duration};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::context::{
    FileReader, RealFileReader, StatsContext, load_cache, load_config, print_preset_info,
    process_file_with_cache, resolve_scan_paths, save_cache, write_output,
};

#[must_use]
pub fn run_stats(args: &StatsArgs, cli: &Cli) -> i32 {
    let result = match &args.action {
        StatsAction::Summary(summary_args) => run_summary(summary_args, cli),
        StatsAction::Files(files_args) => run_files(files_args, cli),
        StatsAction::Breakdown(breakdown_args) => run_breakdown(breakdown_args, cli),
        StatsAction::Trend(trend_args) => run_trend(trend_args, cli),
        StatsAction::History(history_args) => run_history(history_args, cli),
        StatsAction::Report(report_args) => run_report(report_args, cli),
    };

    match result {
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

// ============================================================================
// Summary Subcommand
// ============================================================================

fn run_summary(args: &SummaryArgs, cli: &Cli) -> crate::Result<i32> {
    let (project_stats, project_root, cache) = collect_stats(&args.common, cli)?;
    save_cache_if_enabled(&args.common, &cache, &project_root);

    // Summary only: no file list, no breakdown
    let project_stats = project_stats.with_summary_only();

    let color_mode = super::context::color_choice_to_mode(cli.color);
    let output =
        format_stats_subcommand_output(args.format, &project_stats, color_mode, &project_root)?;
    println!("{output}");
    Ok(EXIT_SUCCESS)
}

/// Unified stats output formatter for Text/Json/Markdown formats.
/// Used by summary, files, breakdown, and trend subcommands.
fn format_stats_subcommand_output(
    format: StatsOutputFormat,
    stats: &ProjectStatistics,
    color_mode: ColorMode,
    project_root: &Path,
) -> crate::Result<String> {
    match format {
        StatsOutputFormat::Text => StatsTextFormatter::new(color_mode)
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        StatsOutputFormat::Json => StatsJsonFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        StatsOutputFormat::Markdown => StatsMarkdownFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
    }
}

// ============================================================================
// Files Subcommand
// ============================================================================

fn run_files(args: &FilesArgs, cli: &Cli) -> crate::Result<i32> {
    let (project_stats, project_root, cache) = collect_stats(&args.common, cli)?;
    save_cache_if_enabled(&args.common, &cache, &project_root);

    // Convert CLI sort order to output sort order and apply sorting
    let sort_order = cli_sort_to_output_sort(args.sort);
    let project_stats = project_stats.with_sorted_files(sort_order, args.top);

    let color_mode = super::context::color_choice_to_mode(cli.color);
    let output =
        format_stats_subcommand_output(args.format, &project_stats, color_mode, &project_root)?;
    println!("{output}");
    Ok(EXIT_SUCCESS)
}

/// Convert CLI's `FileSortOrder` to output module's `FileSortOrder`.
const fn cli_sort_to_output_sort(cli_sort: CliFileSortOrder) -> FileSortOrder {
    match cli_sort {
        CliFileSortOrder::Code => FileSortOrder::Code,
        CliFileSortOrder::Total => FileSortOrder::Total,
        CliFileSortOrder::Comment => FileSortOrder::Comment,
        CliFileSortOrder::Blank => FileSortOrder::Blank,
        CliFileSortOrder::Name => FileSortOrder::Name,
    }
}

// ============================================================================
// Breakdown Subcommand
// ============================================================================

fn run_breakdown(args: &BreakdownArgs, cli: &Cli) -> crate::Result<i32> {
    let (project_stats, project_root, cache) = collect_stats(&args.common, cli)?;
    save_cache_if_enabled(&args.common, &cache, &project_root);

    // Warn if --depth is used with --by lang (not applicable)
    if args.depth.is_some() && args.by == BreakdownBy::Lang {
        crate::output::print_warning_full(
            "--depth is only applicable with --by dir",
            Some(&format!("Ignoring depth: {}", args.depth.unwrap())),
            None,
        );
    }

    // Apply grouping
    let project_stats = match args.by {
        BreakdownBy::Lang => project_stats.with_language_breakdown(),
        BreakdownBy::Dir => {
            project_stats.with_directory_breakdown_depth(Some(&project_root), args.depth)
        }
    };

    let color_mode = super::context::color_choice_to_mode(cli.color);
    let output =
        format_stats_subcommand_output(args.format, &project_stats, color_mode, &project_root)?;
    println!("{output}");
    Ok(EXIT_SUCCESS)
}

// ============================================================================
// Trend Subcommand
// ============================================================================

fn run_trend(args: &TrendArgs, cli: &Cli) -> crate::Result<i32> {
    let (project_stats, project_root, cache) = collect_stats(&args.common, cli)?;
    save_cache_if_enabled(&args.common, &cache, &project_root);

    // Load history and compute trend delta
    let default_path = state::history_path(&project_root);
    let history_path = args.history_file.as_ref().unwrap_or(&default_path);
    let history = TrendHistory::load_or_default(history_path);

    // Get current time for delta computation
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_secs();

    // Compute trend: either from --since duration or from latest entry
    let trend = args.since.as_ref().map_or_else(
        || history.compute_delta(&project_stats),
        |since_str| match parse_duration(since_str) {
            Ok(duration_secs) => {
                history.compute_delta_since(duration_secs, &project_stats, current_time)
            }
            Err(e) => {
                crate::output::print_warning_full(
                    "Invalid --since duration",
                    Some(&e.message()),
                    Some("Falling back to latest entry comparison"),
                );
                history.compute_delta(&project_stats)
            }
        },
    );

    // Apply trend to stats for display
    let project_stats = if let Some(delta) = trend {
        project_stats.with_trend(delta)
    } else {
        project_stats
    };

    let color_mode = super::context::color_choice_to_mode(cli.color);
    let output =
        format_stats_subcommand_output(args.format, &project_stats, color_mode, &project_root)?;
    println!("{output}");
    Ok(EXIT_SUCCESS)
}

// ============================================================================
// Report Subcommand
// ============================================================================

fn run_report(args: &ReportArgs, cli: &Cli) -> crate::Result<i32> {
    // Load config once and reuse for both report settings and stats collection
    let load_result = load_config(
        args.common.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        cli.offline,
    )?;
    let report_config = &load_result.config.stats.report;

    // Merge CLI flags with config (CLI takes precedence)
    let exclude_sections = build_exclude_set(&args.exclude_sections, &report_config.exclude);
    let top_count = args.top.or(report_config.top_count).unwrap_or(10);
    let breakdown_by = args
        .breakdown_by
        .or_else(|| parse_breakdown_by(report_config.breakdown_by.as_deref()))
        .unwrap_or(BreakdownBy::Lang);
    let trend_since = args
        .since
        .as_ref()
        .or(report_config.trend_since.as_ref())
        .cloned();

    // Pass the already-loaded config to avoid duplicate loading
    let (project_stats, project_root, cache) =
        collect_stats_with_config(&args.common, cli, load_result)?;
    save_cache_if_enabled(&args.common, &cache, &project_root);

    // Load history for trend
    let default_path = state::history_path(&project_root);
    let history_path = args.history_file.as_ref().unwrap_or(&default_path);
    let history = TrendHistory::load_or_default(history_path);

    // Warn if user tries to exclude summary (summary is always present in reports)
    if exclude_sections.contains("summary") {
        crate::output::print_warning_full(
            "Cannot exclude 'summary' from report",
            Some("Summary is always included in comprehensive reports"),
            Some("Use 'stats files' or 'stats breakdown' for specific sections only"),
        );
    }

    // Build stats with sections based on exclusion list
    let mut project_stats = project_stats;

    // Files section
    if !exclude_sections.contains("files") {
        project_stats = project_stats.with_top_files(top_count);
    }

    // Breakdown section
    if !exclude_sections.contains("breakdown") {
        project_stats = match breakdown_by {
            BreakdownBy::Lang => project_stats.with_language_breakdown(),
            BreakdownBy::Dir => {
                project_stats.with_directory_breakdown_depth(Some(&project_root), None)
            }
        };
    }

    // Trend section
    if !exclude_sections.contains("trend") {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_secs();

        let trend = trend_since.as_ref().map_or_else(
            || history.compute_delta(&project_stats),
            |since_str| match parse_duration(since_str) {
                Ok(duration_secs) => {
                    history.compute_delta_since(duration_secs, &project_stats, current_time)
                }
                Err(e) => {
                    crate::output::print_warning_full(
                        "Invalid trend_since duration",
                        Some(&e.message()),
                        Some("Falling back to latest entry comparison"),
                    );
                    history.compute_delta(&project_stats)
                }
            },
        );

        if let Some(delta) = trend {
            project_stats = project_stats.with_trend(delta);
        }
    }

    let color_mode = super::context::color_choice_to_mode(cli.color);
    let output = format_report_output(
        args.format,
        &project_stats,
        color_mode,
        &project_root,
        &history,
    )?;

    // Write to file or stdout
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    Ok(EXIT_SUCCESS)
}

/// Build the set of sections to exclude, merging CLI args with config.
fn build_exclude_set(cli_excludes: &[String], config_excludes: &[String]) -> HashSet<String> {
    let mut excludes = HashSet::new();
    for section in config_excludes {
        excludes.insert(section.to_lowercase());
    }
    for section in cli_excludes {
        excludes.insert(section.to_lowercase());
    }
    excludes
}

/// Parse `breakdown_by` string from config to enum.
fn parse_breakdown_by(value: Option<&str>) -> Option<BreakdownBy> {
    value.and_then(|v| match v.to_lowercase().as_str() {
        "lang" | "language" => Some(BreakdownBy::Lang),
        "dir" | "directory" => Some(BreakdownBy::Dir),
        _ => None,
    })
}

fn format_report_output(
    format: ReportOutputFormat,
    stats: &ProjectStatistics,
    color_mode: ColorMode,
    project_root: &Path,
    trend_history: &TrendHistory,
) -> crate::Result<String> {
    match format {
        ReportOutputFormat::Text => StatsTextFormatter::new(color_mode)
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        ReportOutputFormat::Json => StatsJsonFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        ReportOutputFormat::Markdown => StatsMarkdownFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .format(stats),
        ReportOutputFormat::Html => StatsHtmlFormatter::new()
            .with_project_root(Some(project_root.to_path_buf()))
            .with_trend_history(trend_history.clone())
            .format(stats),
    }
}

// ============================================================================
// Shared Helpers
// ============================================================================

/// Collect file statistics using common scanning arguments.
fn collect_stats(
    common: &CommonStatsArgs,
    cli: &Cli,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    let load_result = load_config(
        common.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        cli.offline,
    )?;
    collect_stats_with_config(common, cli, load_result)
}

/// Collect file statistics using pre-loaded configuration.
/// This avoids duplicate config loading when the caller already has a `LoadResult`.
fn collect_stats_with_config(
    common: &CommonStatsArgs,
    cli: &Cli,
    load_result: crate::config::LoadResult,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    let mut config = load_result.config;

    // Print preset info if a preset was used
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_info(preset_name);
    }

    // Discover project root
    let project_root = state::discover_project_root(Path::new("."));

    // Load cache if not disabled
    let cache_path = state::cache_path(&project_root);
    let config_hash = compute_config_hash(&config);
    let cache = if common.no_cache {
        None
    } else {
        load_cache(&cache_path, &config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // Apply CLI extensions override
    if let Some(ref cli_extensions) = common.ext {
        config.content.extensions.clone_from(cli_extensions);
    }

    // Build stats context
    let ctx = StatsContext::from_config(&config);

    // Prepare exclude patterns
    let mut exclude_patterns = config.scanner.exclude.clone();
    exclude_patterns.extend(common.exclude.clone());

    // Determine paths to scan
    let paths_to_scan = resolve_scan_paths(&common.paths, &common.include);

    // Scan directories
    let use_gitignore = config.scanner.gitignore && !common.no_gitignore;
    let all_files = scan_files(&paths_to_scan, &exclude_patterns, use_gitignore)?;

    // Process files in parallel
    let reader = RealFileReader;
    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let file_stats: Vec<_> = all_files
        .par_iter()
        .filter(|file_path| {
            if ctx.allowed_extensions.is_empty() {
                return true;
            }
            file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ctx.allowed_extensions.contains(ext))
        })
        .filter_map(|file_path| {
            let result = collect_file_stats(file_path, &ctx.registry, &cache, &reader);
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    Ok((ProjectStatistics::new(file_stats), project_root, cache))
}

/// Save cache if caching is enabled.
fn save_cache_if_enabled(common: &CommonStatsArgs, cache: &Mutex<Cache>, project_root: &Path) {
    if !common.no_cache
        && let Ok(cache_guard) = cache.lock()
    {
        let cache_path = state::cache_path(project_root);
        save_cache(&cache_path, &cache_guard);
    }
}

fn collect_file_stats(
    file_path: &Path,
    registry: &LanguageRegistry,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> Option<FileStatistics> {
    let (stats, language) = process_file_with_cache(file_path, registry, cache, reader)?;
    Some(FileStatistics {
        path: file_path.to_path_buf(),
        stats,
        language,
    })
}

/// Kept for backward compatibility with existing tests.
#[cfg(test)]
pub(crate) fn format_stats_output(
    format: crate::output::OutputFormat,
    stats: &ProjectStatistics,
    color_mode: ColorMode,
    project_root: Option<&Path>,
    trend_history: Option<&TrendHistory>,
) -> crate::Result<String> {
    use crate::output::OutputFormat;
    match format {
        OutputFormat::Text => StatsTextFormatter::new(color_mode)
            .with_project_root(project_root.map(Path::to_path_buf))
            .format(stats),
        OutputFormat::Json => StatsJsonFormatter::new()
            .with_project_root(project_root.map(Path::to_path_buf))
            .format(stats),
        OutputFormat::Sarif => Err(crate::SlocGuardError::Config(
            "SARIF output format is not supported for stats command".to_string(),
        )),
        OutputFormat::Markdown => StatsMarkdownFormatter::new()
            .with_project_root(project_root.map(Path::to_path_buf))
            .format(stats),
        OutputFormat::Html => {
            let mut formatter =
                StatsHtmlFormatter::new().with_project_root(project_root.map(Path::to_path_buf));
            if let Some(history) = trend_history {
                formatter = formatter.with_trend_history(history.clone());
            }
            formatter.format(stats)
        }
    }
}

// ============================================================================
// History Subcommand
// ============================================================================

// Note: `_cli` is reserved for future options like `--verbose`, `--quiet`, or `--color` support
fn run_history(args: &HistoryArgs, _cli: &Cli) -> crate::Result<i32> {
    // Discover project root for history file resolution
    let project_root = state::discover_project_root(Path::new("."));

    // Determine history file path
    let default_path = state::history_path(&project_root);
    let history_path = args.history_file.as_ref().unwrap_or(&default_path);

    // Load history
    let history = TrendHistory::load_or_default(history_path);

    // Get entries (most recent first, limited by --limit)
    let entries = history.entries();
    let total_entries = entries.len();
    let display_entries: Vec<_> = entries.iter().rev().take(args.limit).collect();

    // Format output
    let output = match args.format {
        HistoryOutputFormat::Text => format_history_text(&display_entries, total_entries),
        HistoryOutputFormat::Json => format_history_json(&display_entries)?,
    };

    println!("{output}");

    Ok(EXIT_SUCCESS)
}

/// Format history entries as human-readable text.
fn format_history_text(entries: &[&TrendEntry], total_entries: usize) -> String {
    use std::fmt::Write;

    if entries.is_empty() {
        return "No history entries found.\n\nRecord a snapshot with: sloc-guard snapshot"
            .to_string();
    }

    let mut output = String::new();
    let _ = writeln!(
        output,
        "History ({} of {} entries)\n",
        entries.len(),
        total_entries
    );

    for (i, entry) in entries.iter().enumerate() {
        // Format timestamp as ISO 8601 datetime
        let datetime = format_timestamp(entry.timestamp);

        // Format git context
        let git_info = match (&entry.git_ref, &entry.git_branch) {
            (Some(commit), Some(branch)) => format!(" - {commit} ({branch})"),
            (Some(commit), None) => format!(" - {commit}"),
            (None, Some(branch)) => format!(" ({branch})"),
            (None, None) => String::new(),
        };

        let _ = writeln!(output, "{}. {datetime}{git_info}", i + 1);
        let _ = writeln!(
            output,
            "   Files: {}  Total: {}  Code: {}  Comment: {}  Blank: {}",
            entry.total_files, entry.total_lines, entry.code, entry.comment, entry.blank
        );

        // Add empty line between entries (except for the last one)
        if i < entries.len() - 1 {
            output.push('\n');
        }
    }

    output
}

/// Format history entries as JSON.
fn format_history_json(entries: &[&TrendEntry]) -> crate::Result<String> {
    // Create a struct for JSON serialization
    #[derive(serde::Serialize)]
    struct HistoryOutput<'a> {
        count: usize,
        entries: &'a [&'a TrendEntry],
    }

    let output = HistoryOutput {
        count: entries.len(),
        entries,
    };

    serde_json::to_string_pretty(&output).map_err(crate::SlocGuardError::from)
}

/// Format Unix timestamp as ISO 8601 datetime string.
///
/// Uses manual UTC calculation to avoid adding a datetime dependency (chrono/time).
/// This is acceptable for simple UTC formatting; complex timezone handling would warrant a crate.
fn format_timestamp(timestamp: u64) -> String {
    // Convert to date components (simplified UTC implementation)
    let days_since_epoch = timestamp / 86400;
    let secs_in_day = timestamp % 86400;

    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;

    // Calculate year/month/day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days_since_epoch);

    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02}")
}

/// Convert days since Unix epoch to (year, month, day).
#[allow(clippy::cast_possible_wrap)]
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Simplified algorithm for UTC date calculation
    // Safe cast: days since 1970 won't exceed i64::MAX for foreseeable dates
    let mut remaining_days = days as i64;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let days_in_month = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &days_in_m in &days_in_month {
        if remaining_days < days_in_m {
            break;
        }
        remaining_days -= days_in_m;
        month += 1;
    }

    let day = remaining_days + 1;

    // Safe cast: year >= 1970 and day >= 1 are guaranteed by the algorithm above
    #[allow(clippy::cast_sign_loss)]
    (year as u64, month, day as u64)
}

/// Check if a year is a leap year.
const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
