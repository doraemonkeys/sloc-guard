use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use crate::cache::{Cache, compute_config_hash};
use crate::cli::{Cli, CommonStatsArgs};
use crate::config::{Config, FetchPolicy, LoadResult, LocatedLoadResult};
use crate::language::LanguageRegistry;
use crate::output::{FileStatistics, ProjectStatistics, ScanProgress};
use crate::project::ProjectPaths;
use crate::scanner::scan_files_with_project_paths;
use crate::state;

use crate::commands::config_notice::{print_config_notice, print_preset_notice};
use crate::commands::context::{
    FileProcessResult, FileReader, RealFileReader, StatsContext, color_choice_to_mode, load_cache,
    load_config, resolve_config_root, resolve_exclude_patterns, resolve_scan_paths, save_cache,
};

/// Collect file statistics using common scanning arguments.
#[allow(dead_code)] // Compatibility entry point for callers that do not own an output format.
pub fn collect_stats(
    common: &CommonStatsArgs,
    cli: &Cli,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    collect_stats_with_notice(common, cli, false)
}

/// Collect statistics and report configuration provenance according to the output mode.
pub fn collect_stats_with_notice(
    common: &CommonStatsArgs,
    cli: &Cli,
    human_text: bool,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    let load_result = load_config(
        common.config.as_deref(),
        cli.no_config,
        cli.no_extends,
        FetchPolicy::from_cli(cli.extends_policy),
    )?;
    let color_mode = color_choice_to_mode(cli.color);
    print_config_notice(
        &load_result.origin,
        cli.quiet,
        cli.verbose,
        human_text,
        color_mode,
    );
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_notice(preset_name, cli.quiet, cli.verbose, human_text, color_mode);
    }
    collect_stats_with_config(common, cli, load_result)
}

/// Collect file statistics using pre-loaded configuration.
///
/// Avoids duplicate config loading when the caller already has a `LoadResult`.
pub fn collect_stats_with_config(
    common: &CommonStatsArgs,
    cli: &Cli,
    load_result: LocatedLoadResult,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    collect_stats_with_located_config_and_reader(common, cli, load_result, &RealFileReader)
}

/// Collect file statistics with injectable file reader for testability.
///
/// This variant accepts a `FileReader` implementation, enabling unit tests
/// to inject mock readers without filesystem access.
///
/// # Errors
///
/// Returns an error if directory scanning fails or file I/O errors occur.
pub fn collect_stats_with_config_and_reader(
    common: &CommonStatsArgs,
    cli: &Cli,
    load_result: LoadResult,
    reader: &dyn FileReader,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    let project_root = state::discover_project_root(Path::new("."));
    collect_stats_with_paths(
        common,
        cli,
        load_result.config,
        project_root,
        &ProjectPaths::unrooted(),
        reader,
    )
}

fn collect_stats_with_located_config_and_reader(
    common: &CommonStatsArgs,
    cli: &Cli,
    load_result: LocatedLoadResult,
    reader: &dyn FileReader,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    // Resolve state and rule roots before moving the loaded configuration.
    let project_root = state::discover_project_root(Path::new("."));
    let config_root = resolve_config_root(&load_result, &project_root);
    let project_paths = ProjectPaths::rooted(config_root);
    collect_stats_with_paths(
        common,
        cli,
        load_result.config,
        project_root,
        &project_paths,
        reader,
    )
}

fn collect_stats_with_paths(
    common: &CommonStatsArgs,
    cli: &Cli,
    mut config: Config,
    project_root: std::path::PathBuf,
    project_paths: &ProjectPaths,
    reader: &dyn FileReader,
) -> crate::Result<(ProjectStatistics, std::path::PathBuf, Mutex<Cache>)> {
    // Load cache if not disabled
    let cache_path = state::cache_path(&project_root);
    let config_hash = compute_config_hash(&config);
    let cache = if common.no_sloc_cache {
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
    let exclude_patterns =
        resolve_exclude_patterns(&config.scanner.exclude, &common.exclude, project_paths);

    // Determine paths to scan
    let paths_to_scan = resolve_scan_paths(&common.paths, &common.include);

    // Scan directories
    let use_gitignore = config.scanner.gitignore && !common.no_gitignore;
    let all_files = scan_files_with_project_paths(
        &paths_to_scan,
        &exclude_patterns,
        use_gitignore,
        (*project_paths).clone(),
    )?;

    // Process files in parallel
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
            let logical_path = project_paths.logical(file_path);
            let result = collect_file_stats_with_logical_path(
                file_path,
                &logical_path,
                &ctx.registry,
                &cache,
                reader,
            );
            progress.inc();
            result
        })
        .collect();
    progress.finish();

    Ok((ProjectStatistics::new(file_stats), project_root, cache))
}

/// Save cache if caching is enabled (errors are non-critical).
pub fn save_cache_if_enabled(common: &CommonStatsArgs, cache: &Mutex<Cache>, project_root: &Path) {
    if !common.no_sloc_cache
        && let Ok(cache_guard) = cache.lock()
    {
        let cache_path = state::cache_path(project_root);
        let _ = save_cache(&cache_path, &cache_guard);
    }
}

/// Collect statistics for a single file.
///
/// Returns `Some` only for successfully processed files. Skipped files (unknown
/// extension, no extension, ignored by directive) and errors are silently filtered.
/// For stats collection, this silent skip behavior is acceptable since we're just
/// aggregating metrics, not enforcing compliance.
pub fn collect_file_stats(
    file_path: &Path,
    registry: &LanguageRegistry,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> Option<FileStatistics> {
    collect_file_stats_with_logical_path(file_path, file_path, registry, cache, reader)
}

/// Collect statistics from a physical file under a stable logical identity.
pub fn collect_file_stats_with_logical_path(
    file_path: &Path,
    logical_path: &Path,
    registry: &LanguageRegistry,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> Option<FileStatistics> {
    match crate::commands::context::process_file_with_cache_key(
        file_path,
        logical_path,
        registry,
        cache,
        reader,
    ) {
        FileProcessResult::Success { stats, language } => Some(FileStatistics {
            path: logical_path.to_path_buf(),
            stats,
            language,
        }),
        // Skipped files (unknown extension, no extension, ignored by directive)
        // and errors are silently filtered for stats collection
        FileProcessResult::Skipped(_) | FileProcessResult::Error(_) => None,
    }
}
