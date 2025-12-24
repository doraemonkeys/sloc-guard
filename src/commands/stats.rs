use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{Cache, compute_config_hash};
use crate::cli::{Cli, GroupBy, StatsArgs};
use crate::language::LanguageRegistry;
use crate::output::{
    FileStatistics, OutputFormat, ProjectStatistics, ScanProgress, StatsFormatter,
    StatsJsonFormatter, StatsMarkdownFormatter, StatsTextFormatter,
};
use crate::scanner::scan_files;
use crate::state;
use crate::stats::TrendHistory;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::context::{
    FileReader, RealFileReader, StatsContext, load_cache, load_config, process_file_with_cache,
    resolve_scan_paths, save_cache, write_output,
};

#[must_use]
pub fn run_stats(args: &StatsArgs, cli: &Cli) -> i32 {
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
        GroupBy::Dir => project_stats.with_directory_breakdown(),
        GroupBy::None => project_stats,
    };
    let project_stats = if let Some(n) = args.top {
        project_stats.with_top_files(n)
    } else {
        project_stats
    };

    // 5.2 Trend tracking if enabled
    let project_stats = if args.trend {
        let default_path = state::history_path(project_root);
        let history_path = args.history_file.as_ref().unwrap_or(&default_path);
        let mut history = TrendHistory::load_or_default(history_path);
        let trend = history.compute_delta(&project_stats);
        history.add(&project_stats);
        // Ensure parent directory exists before saving
        if let Some(parent) = history_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Save history (silently ignore errors)
        let _ = history.save(history_path);
        if let Some(delta) = trend {
            project_stats.with_trend(delta)
        } else {
            project_stats
        }
    } else {
        project_stats
    };

    // 6. Format output
    let output = format_stats_output(args.format, &project_stats)?;

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
) -> crate::Result<String> {
    match format {
        OutputFormat::Text => StatsTextFormatter.format(stats),
        OutputFormat::Json => StatsJsonFormatter.format(stats),
        OutputFormat::Sarif => Err(crate::SlocGuardError::Config(
            "SARIF output format is not supported for stats command".to_string(),
        )),
        OutputFormat::Markdown => StatsMarkdownFormatter.format(stats),
        OutputFormat::Html => Err(crate::SlocGuardError::Config(
            "HTML output format is not yet supported for stats command".to_string(),
        )),
    }
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
