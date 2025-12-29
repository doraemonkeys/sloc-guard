use crate::cli::{
    BreakdownArgs, BreakdownBy, Cli, FileSortOrder as CliFileSortOrder, FilesArgs, StatsAction,
    StatsArgs, SummaryArgs, TrendArgs,
};
use crate::output::FileSortOrder;
use crate::state;
use crate::stats::{TrendHistory, parse_duration};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::collection::{collect_stats, save_cache_if_enabled};
use super::formatting::format_stats_subcommand_output;
use super::history::run_history;
use super::report::run_report;
use crate::commands::context::color_choice_to_mode;

/// Main entry point for the stats command.
///
/// Dispatches to the appropriate subcommand handler.
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

    let color_mode = color_choice_to_mode(cli.color);
    let output =
        format_stats_subcommand_output(args.format, &project_stats, color_mode, &project_root)?;
    println!("{output}");
    Ok(EXIT_SUCCESS)
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

    let color_mode = color_choice_to_mode(cli.color);
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

    let color_mode = color_choice_to_mode(cli.color);
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

    let color_mode = color_choice_to_mode(cli.color);
    let output =
        format_stats_subcommand_output(args.format, &project_stats, color_mode, &project_root)?;
    println!("{output}");
    Ok(EXIT_SUCCESS)
}
