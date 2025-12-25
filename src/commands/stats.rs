use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{Cache, compute_config_hash};
use crate::cli::{Cli, GroupBy, HistoryArgs, HistoryOutputFormat, StatsAction, StatsArgs};
use crate::git::GitContext;
use crate::language::LanguageRegistry;
use crate::output::{
    ColorMode, FileStatistics, OutputFormat, ProjectStatistics, ScanProgress, StatsFormatter,
    StatsHtmlFormatter, StatsJsonFormatter, StatsMarkdownFormatter, StatsTextFormatter,
};
use crate::scanner::scan_files;
use crate::state;
use crate::stats::{TrendEntry, TrendHistory, parse_duration};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::context::{
    FileReader, RealFileReader, StatsContext, load_cache, load_config, process_file_with_cache,
    resolve_scan_paths, save_cache, write_output,
};

#[must_use]
pub fn run_stats(args: &StatsArgs, cli: &Cli) -> i32 {
    // Handle subcommand if present
    if let Some(StatsAction::History(history_args)) = &args.action {
        return run_history(history_args, cli);
    }

    match run_stats_impl(args, cli) {
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

pub(crate) fn run_stats_impl(args: &StatsArgs, cli: &Cli) -> crate::Result<i32> {
    // 1. Load configuration (for exclude patterns)
    let mut config = load_config(
        args.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        cli.offline,
    )?;

    // 1.0.1 Discover project root for consistent state file resolution
    let project_root = state::discover_project_root(Path::new("."));

    // 1.1 Load cache if not disabled
    let cache_path = state::cache_path(&project_root);
    let config_hash = compute_config_hash(&config);
    let cache = if args.no_cache {
        None
    } else {
        load_cache(&cache_path, &config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // Apply CLI extensions override if provided
    if let Some(ref cli_extensions) = args.ext {
        config.content.extensions.clone_from(cli_extensions);
    }

    // 2. Build stats context with dependencies
    let ctx = StatsContext::from_config(&config);

    // 3. Run stats with context
    run_stats_with_context(args, cli, &config, &ctx, &cache, &project_root)
}

/// Internal implementation accepting injectable context (for testing).
///
/// This function contains the core stats logic and accepts pre-built dependencies,
/// enabling unit testing with custom/mock components.
pub(crate) fn run_stats_with_context(
    args: &StatsArgs,
    cli: &Cli,
    config: &crate::config::Config,
    ctx: &StatsContext,
    cache: &Mutex<Cache>,
    project_root: &Path,
) -> crate::Result<i32> {
    // 1. Prepare exclude patterns from scanner config
    let mut exclude_patterns = config.scanner.exclude.clone();
    exclude_patterns.extend(args.exclude.clone());

    // 2. Determine paths to scan
    let paths_to_scan = resolve_scan_paths(&args.paths, &args.include, config);

    // 3. Scan directories (respecting .gitignore if enabled)
    // Scanner returns ALL files, extension filtering is done below
    let use_gitignore = config.scanner.gitignore && !args.no_gitignore;
    let all_files = scan_files(&paths_to_scan, &exclude_patterns, use_gitignore)?;

    // 4. Process each file and collect statistics (parallel with rayon) using injected context
    let reader = RealFileReader;
    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let file_stats: Vec<_> = all_files
        .par_iter()
        .filter(|file_path| {
            // Filter by extension using context's allowed_extensions
            if ctx.allowed_extensions.is_empty() {
                return true;
            }
            file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ctx.allowed_extensions.contains(ext))
        })
        .filter_map(|file_path| {
            let result = collect_file_stats(file_path, &ctx.registry, cache, &reader);
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    // 5. Save cache if not disabled
    if !args.no_cache
        && let Ok(cache_guard) = cache.lock()
    {
        let cache_path = state::cache_path(project_root);
        save_cache(&cache_path, &cache_guard);
    }

    let project_stats = ProjectStatistics::new(file_stats);
    let project_stats = match args.group_by {
        GroupBy::Lang => project_stats.with_language_breakdown(),
        GroupBy::Dir => project_stats.with_directory_breakdown_relative(Some(project_root)),
        GroupBy::None => project_stats,
    };
    let project_stats = if let Some(n) = args.top {
        project_stats.with_top_files(n)
    } else {
        project_stats
    };

    // 5.2 Trend tracking if enabled (--trend or --since implies trend)
    let trend_enabled = args.trend || args.since.is_some();
    let (project_stats, trend_history) = if trend_enabled {
        let default_path = state::history_path(project_root);
        let history_path = args.history_file.as_ref().unwrap_or(&default_path);
        let mut history = TrendHistory::load_or_default(history_path);

        // Capture git context for trend entry (commit hash and branch name)
        let git_context = GitContext::from_path(project_root);

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

        // Add entry only if retention policy allows (respects min_interval_secs)
        // Git context is captured to record commit hash and branch for later delta display
        history.add_if_allowed_with_context(&project_stats, &config.trend, git_context.as_ref());

        // Ensure parent directory exists before saving
        if let Some(parent) = history_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Save with retention policy applied (cleanup old entries)
        let _ = history.save_with_retention(history_path, &config.trend);

        // Only show trend if delta is significant (reduces noise from trivial changes)
        let project_stats = if let Some(delta) = trend
            && delta.is_significant(&config.trend)
        {
            project_stats.with_trend(delta)
        } else {
            project_stats
        };

        (project_stats, Some(history))
    } else {
        (project_stats, None)
    };

    // 6. Format output
    let color_mode = super::context::color_choice_to_mode(cli.color);
    let output = format_stats_output(
        args.format,
        &project_stats,
        color_mode,
        Some(project_root),
        trend_history.as_ref(),
    )?;

    // 7. Write output
    write_output(args.output.as_deref(), &output, cli.quiet)?;

    Ok(EXIT_SUCCESS)
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

pub(crate) fn format_stats_output(
    format: OutputFormat,
    stats: &ProjectStatistics,
    color_mode: ColorMode,
    project_root: Option<&Path>,
    trend_history: Option<&TrendHistory>,
) -> crate::Result<String> {
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

/// Run the `stats history` subcommand.
#[must_use]
pub fn run_history(args: &HistoryArgs, cli: &Cli) -> i32 {
    match run_history_impl(args, cli) {
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

// Note: `_cli` is reserved for future options like `--verbose`, `--quiet`, or `--color` support
fn run_history_impl(args: &HistoryArgs, _cli: &Cli) -> crate::Result<i32> {
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
        return "No history entries found.\n\nRun `sloc-guard stats --trend` to start tracking."
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
