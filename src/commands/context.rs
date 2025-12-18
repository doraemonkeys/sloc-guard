use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::baseline::read_file_with_hash;
use crate::cache::Cache;
use crate::cli::ColorChoice;
use crate::config::{Config, ConfigLoader, FileConfigLoader};
use crate::counter::{CountResult, LineStats, SlocCounter};
use crate::language::LanguageRegistry;
use crate::output::ColorMode;

/// Default cache file path
pub const DEFAULT_CACHE_PATH: &str = ".sloc-guard-cache.json";

/// Default history file path for trend tracking
pub const DEFAULT_HISTORY_PATH: &str = ".sloc-guard-history.json";

/// Get file metadata (mtime, size) for cache validation.
#[must_use]
pub fn get_file_metadata(path: &Path) -> Option<(u64, u64)> {
    let metadata = fs::metadata(path).ok()?;
    let mtime = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let size = metadata.len();
    Some((mtime, size))
}

#[must_use]
pub(crate) const fn color_choice_to_mode(choice: ColorChoice) -> ColorMode {
    match choice {
        ColorChoice::Auto => ColorMode::Auto,
        ColorChoice::Always => ColorMode::Always,
        ColorChoice::Never => ColorMode::Never,
    }
}

pub(crate) fn load_config(
    config_path: Option<&Path>,
    no_config: bool,
    no_extends: bool,
) -> crate::Result<Config> {
    if no_config {
        return Ok(Config::default());
    }

    let loader = FileConfigLoader::new();
    if no_extends {
        config_path.map_or_else(
            || loader.load_without_extends(),
            |path| loader.load_from_path_without_extends(path),
        )
    } else {
        config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))
    }
}

#[must_use]
pub fn load_cache(config_hash: &str) -> Option<Cache> {
    let cache_path = Path::new(DEFAULT_CACHE_PATH);
    if !cache_path.exists() {
        return None;
    }

    Cache::load(cache_path)
        .ok()
        .filter(|cache| cache.is_valid(config_hash))
}

pub fn save_cache(cache: &Cache) {
    let cache_path = Path::new(DEFAULT_CACHE_PATH);
    // Silently ignore errors when saving cache
    let _ = cache.save(cache_path);
}

pub(crate) fn resolve_scan_paths(
    paths: &[PathBuf],
    include: &[String],
    config: &Config,
) -> Vec<PathBuf> {
    // CLI --include overrides config include_paths
    if !include.is_empty() {
        return include.iter().map(PathBuf::from).collect();
    }

    // If CLI paths provided (other than default "."), use them
    let default_path = PathBuf::from(".");
    if paths.len() != 1 || paths[0] != default_path {
        return paths.to_vec();
    }

    // Use config include_paths if available
    if !config.default.include_paths.is_empty() {
        return config
            .default
            .include_paths
            .iter()
            .map(PathBuf::from)
            .collect();
    }

    // Default to current directory
    paths.to_vec()
}

pub(crate) fn write_output(
    output_path: Option<&Path>,
    content: &str,
    quiet: bool,
) -> crate::Result<()> {
    if let Some(path) = output_path {
        fs::write(path, content)?;
    } else if !quiet {
        print!("{content}");
    }
    Ok(())
}

/// Count lines from pre-read file content.
#[must_use]
pub fn count_lines_from_content(content: &[u8], counter: &SlocCounter) -> Option<LineStats> {
    match counter.count_from_bytes(content) {
        CountResult::Stats(stats) => Some(stats),
        CountResult::IgnoredFile => None,
    }
}

/// Process file with cache support for stats collection.
pub fn process_file_with_cache(
    file_path: &Path,
    registry: &LanguageRegistry,
    cache: &Mutex<Cache>,
) -> Option<(LineStats, String)> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let path_key = file_path.to_string_lossy().replace('\\', "/");

    // Get file metadata for fast cache validation
    let (mtime, size) = get_file_metadata(file_path)?;

    // Try to get stats from cache using metadata (no file read needed)
    let cached_stats = {
        let cache_guard = cache.lock().ok()?;
        cache_guard
            .get_if_metadata_matches(&path_key, mtime, size)
            .map(|entry| LineStats::from(&entry.stats))
    };

    let stats = if let Some(stats) = cached_stats {
        stats
    } else {
        // Cache miss: read file, compute hash, and count lines
        let (file_hash, content) = read_file_with_hash(file_path).ok()?;
        let counter = SlocCounter::new(&language.comment_syntax);
        let result = count_lines_from_content(&content, &counter)?;

        // Update cache with metadata
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.set(&path_key, file_hash, &result, mtime, size);
        }

        result
    };

    Some((stats, language.name.clone()))
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
