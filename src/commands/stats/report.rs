use std::collections::HashSet;

use crate::EXIT_SUCCESS;
use crate::cli::{BreakdownBy, Cli, ReportArgs};
use crate::state;
use crate::stats::{TrendHistory, parse_duration};

use super::collection::{collect_stats_with_config, save_cache_if_enabled};
use super::formatting::format_report_output;
use crate::commands::context::{color_choice_to_mode, load_config, write_output};

const DEFAULT_TOP_COUNT: usize = 20;

/// Run the report subcommand: generate a comprehensive stats report.
pub fn run_report(args: &ReportArgs, cli: &Cli) -> crate::Result<i32> {
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
    let top_count = args
        .top
        .or(report_config.top_count)
        .unwrap_or(DEFAULT_TOP_COUNT);
    let breakdown_by = args
        .breakdown_by
        .or_else(|| parse_breakdown_by(report_config.breakdown_by.as_deref()))
        .unwrap_or(BreakdownBy::Lang);
    let breakdown_depth = args.depth.or(report_config.depth);
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
        // Warn if --depth is used with --breakdown-by lang (not applicable)
        if let Some(depth) = breakdown_depth
            && breakdown_by == BreakdownBy::Lang
        {
            crate::output::print_warning_full(
                "--depth is only applicable with --breakdown-by dir",
                Some(&format!("Ignoring depth: {depth}")),
                None,
            );
        }

        // Warn if depth = 0 (meaningless, behaves same as None)
        if breakdown_depth == Some(0) && breakdown_by == BreakdownBy::Dir {
            crate::output::print_warning_full(
                "depth = 0 has no effect",
                Some("Depth 0 behaves the same as no depth limit (shows full paths)"),
                Some(
                    "Use depth >= 1 for meaningful grouping (1 = top-level, 2 = two levels, etc.)",
                ),
            );
        }

        project_stats = match breakdown_by {
            BreakdownBy::Lang => project_stats.with_language_breakdown(),
            BreakdownBy::Dir => {
                project_stats.with_directory_breakdown_depth(Some(&project_root), breakdown_depth)
            }
        };
    }

    // Trend section
    if !exclude_sections.contains("trend") {
        let current_time = state::current_unix_timestamp();

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

    let color_mode = color_choice_to_mode(cli.color);
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
pub fn build_exclude_set(cli_excludes: &[String], config_excludes: &[String]) -> HashSet<String> {
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
pub fn parse_breakdown_by(value: Option<&str>) -> Option<BreakdownBy> {
    value.and_then(|v| match v.to_lowercase().as_str() {
        "lang" | "language" => Some(BreakdownBy::Lang),
        "dir" | "directory" => Some(BreakdownBy::Dir),
        _ => None,
    })
}
