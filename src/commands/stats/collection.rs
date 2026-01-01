use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{Cache, compute_config_hash};
use crate::cli::{Cli, CommonStatsArgs};
use crate::config::LoadResult;
use crate::language::LanguageRegistry;
use crate::output::{FileStatistics, ProjectStatistics, ScanProgress};
use crate::scanner::scan_files;
use crate::state;

use crate::commands::context::{
    FileReader, RealFileReader, StatsContext, load_cache, load_config, print_preset_info,
    process_file_with_cache, resolve_scan_paths, save_cache,
};

/// Collect file statistics using common scanning arguments.
pub fn collect_stats(
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
///
/// Avoids duplicate config loading when the caller already has a `LoadResult`.
pub fn collect_stats_with_config(
    common: &CommonStatsArgs,
    cli: &Cli,
    load_result: LoadResult,
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

/// Save cache if caching is enabled (errors are non-critical).
pub fn save_cache_if_enabled(common: &CommonStatsArgs, cache: &Mutex<Cache>, project_root: &Path) {
    if !common.no_cache
        && let Ok(cache_guard) = cache.lock()
    {
        let cache_path = state::cache_path(project_root);
        let _ = save_cache(&cache_path, &cache_guard);
    }
}

/// Collect statistics for a single file.
pub fn collect_file_stats(
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
