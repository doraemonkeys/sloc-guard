//! Snapshot command: record current statistics to trend history.
//!
//! Follows read/write separation principle:
//! - `stats trend/report` = read history (read-only)
//! - `snapshot` = write history (this command)

use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{Cache, compute_config_hash};
use crate::cli::{Cli, SnapshotArgs};
use crate::config::FetchPolicy;
use crate::git::GitContext;
use crate::output::{ProjectStatistics, ScanProgress};
use crate::scanner::scan_files;
use crate::state;
use crate::stats::TrendHistory;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::context::{
    RealFileReader, StatsContext, load_cache, load_config, print_preset_info, resolve_scan_paths,
    save_cache,
};
use super::stats::collect_file_stats;

/// Run the snapshot command.
///
/// Records current project statistics to trend history, respecting retention policies.
#[must_use]
pub fn run_snapshot(args: &SnapshotArgs, cli: &Cli) -> i32 {
    match run_snapshot_inner(args, cli) {
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

fn run_snapshot_inner(args: &SnapshotArgs, cli: &Cli) -> crate::Result<i32> {
    // 1. Load configuration
    let load_result = load_config(
        args.common.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        FetchPolicy::from_cli(cli.extends_policy),
    )?;
    let mut config = load_result.config;

    // 1a. Print preset info if a preset was used
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_info(preset_name);
    }

    // 1b. Discover project root
    let project_root = state::discover_project_root(Path::new("."));

    // 1c. Load cache if not disabled
    let cache_path = state::cache_path(&project_root);
    let config_hash = compute_config_hash(&config);
    let cache = if args.common.no_sloc_cache {
        None
    } else {
        load_cache(&cache_path, &config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // Apply CLI extensions override
    if let Some(ref cli_extensions) = args.common.ext {
        config.content.extensions.clone_from(cli_extensions);
    }

    // 2. Build stats context
    let ctx = StatsContext::from_config(&config);

    // 3. Prepare exclude patterns
    let mut exclude_patterns = config.scanner.exclude.clone();
    exclude_patterns.extend(args.common.exclude.clone());

    // 4. Determine paths to scan
    let paths_to_scan = resolve_scan_paths(&args.common.paths, &args.common.include);

    // 5. Scan directories
    let use_gitignore = config.scanner.gitignore && !args.common.no_gitignore;
    let all_files = scan_files(&paths_to_scan, &exclude_patterns, use_gitignore)?;

    // 6. Process files in parallel
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

    // 7. Save cache if enabled (errors are non-critical)
    if !args.common.no_sloc_cache
        && let Ok(cache_guard) = cache.lock()
    {
        let _ = save_cache(&cache_path, &cache_guard);
    }

    // 8. Build project statistics
    let project_stats = ProjectStatistics::new(file_stats);

    // 9. Get git context for the snapshot
    let git_context = GitContext::from_path(&project_root);

    // 10. Load history and add entry
    let default_path = state::history_path(&project_root);
    let history_path = args.history_file.as_ref().unwrap_or(&default_path);
    let mut history = TrendHistory::load_or_default(history_path);

    // Get trend config
    let trend_config = config.trend.clone();

    // Check if we should add (respects min_interval_secs unless --force)
    let current_time = state::current_unix_timestamp();

    let should_add = args.force || history.should_add(&trend_config, current_time);

    if args.dry_run {
        print_dry_run_output(&project_stats, git_context.as_ref(), should_add);
        return Ok(EXIT_SUCCESS);
    }

    if !should_add {
        if !cli.quiet {
            let min_interval = trend_config.min_interval_secs.unwrap_or(0);
            println!(
                "Snapshot skipped: min_interval_secs ({min_interval}s) not elapsed since last entry."
            );
            println!("Use --force to override.");
        }
        return Ok(EXIT_SUCCESS);
    }

    // Add entry with git context
    history.add_with_context(&project_stats, git_context.as_ref());

    // Save with retention policy applied
    history.save_with_retention(history_path, &trend_config)?;

    if !cli.quiet {
        print_snapshot_summary(&project_stats, git_context.as_ref(), history_path);
    }

    Ok(EXIT_SUCCESS)
}

fn print_dry_run_output(
    stats: &ProjectStatistics,
    git_context: Option<&GitContext>,
    would_add: bool,
) {
    println!("Dry-run: snapshot NOT saved\n");

    if would_add {
        println!("Would record:");
    } else {
        println!("Would skip (min_interval_secs not elapsed):");
    }

    println!("  Files:    {}", stats.total_files);
    println!("  Total:    {} lines", stats.total_lines);
    println!("  Code:     {} lines", stats.total_code);
    println!("  Comment:  {} lines", stats.total_comment);
    println!("  Blank:    {} lines", stats.total_blank);

    if let Some(ctx) = git_context {
        print!("  Git:      {}", ctx.commit);
        if let Some(ref branch) = ctx.branch {
            print!(" ({branch})");
        }
        println!();
    }
}

fn print_snapshot_summary(
    stats: &ProjectStatistics,
    git_context: Option<&GitContext>,
    history_path: &Path,
) {
    println!("Snapshot recorded to {}\n", history_path.display());

    println!("  Files:    {}", stats.total_files);
    println!("  Total:    {} lines", stats.total_lines);
    println!("  Code:     {} lines", stats.total_code);
    println!("  Comment:  {} lines", stats.total_comment);
    println!("  Blank:    {} lines", stats.total_blank);

    if let Some(ctx) = git_context {
        print!("  Git:      {}", ctx.commit);
        if let Some(ref branch) = ctx.branch {
            print!(" ({branch})");
        }
        println!();
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
