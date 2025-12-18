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
use crate::stats::TrendHistory;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS};

use super::context::{
    DEFAULT_HISTORY_PATH, load_cache, load_config, process_file_with_cache, resolve_scan_paths,
    save_cache, write_output,
};

#[must_use]
pub fn run_stats(args: &StatsArgs, cli: &Cli) -> i32 {
    match run_stats_impl(args, cli) {
        Ok(exit_code) => exit_code,
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

pub(crate) fn run_stats_impl(args: &StatsArgs, cli: &Cli) -> crate::Result<i32> {
    // 1. Load configuration (for exclude patterns)
    let config = load_config(args.config.as_deref(), cli.no_config, cli.no_extends)?;

    // 1.1 Load cache if not disabled
    let config_hash = compute_config_hash(&config);
    let cache = if args.no_cache {
        None
    } else {
        load_cache(&config_hash)
    };
    let cache = Mutex::new(cache.unwrap_or_else(|| Cache::new(config_hash)));

    // 2. Prepare extensions and exclude patterns
    let extensions = args
        .ext
        .clone()
        .unwrap_or_else(|| config.default.extensions.clone());
    let mut exclude_patterns = config.exclude.patterns.clone();
    exclude_patterns.extend(args.exclude.clone());

    // 3. Determine paths to scan
    let paths_to_scan = resolve_scan_paths(&args.paths, &args.include, &config);

    // 4. Scan directories (respecting .gitignore if enabled)
    let use_gitignore = config.default.gitignore && !args.no_gitignore;
    let all_files = scan_files(
        &paths_to_scan,
        &extensions,
        &exclude_patterns,
        use_gitignore,
    )?;

    // 5. Process each file and collect statistics (parallel with rayon)
    let registry = LanguageRegistry::with_custom_languages(&config.languages);

    let progress = ScanProgress::new(all_files.len() as u64, cli.quiet);
    let file_stats: Vec<_> = all_files
        .par_iter()
        .filter_map(|file_path| {
            let result = collect_file_stats(file_path, &registry, &cache);
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    // 5.1 Save cache if not disabled
    if !args.no_cache
        && let Ok(cache_guard) = cache.lock()
    {
        save_cache(&cache_guard);
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
        let history_path = Path::new(DEFAULT_HISTORY_PATH);
        let mut history = TrendHistory::load_or_default(history_path);
        let trend = history.compute_delta(&project_stats);
        history.add(&project_stats);
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
) -> Option<FileStatistics> {
    let (stats, language) = process_file_with_cache(file_path, registry, cache)?;
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
