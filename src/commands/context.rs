use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

use crate::cache::Cache;
use crate::checker::{StructureChecker, ThresholdChecker};
use crate::cli::ColorChoice;
use crate::config::{Config, ConfigLoader, FileConfigLoader, LoadResult};
use crate::counter::{CountResult, LineStats, SlocCounter};
use crate::language::LanguageRegistry;
use crate::output::ColorMode;
use crate::scanner::{AllowlistRuleBuilder, CompositeScanner, FileScanner, StructureScanConfig};

#[must_use]
pub(crate) const fn color_choice_to_mode(choice: ColorChoice) -> ColorMode {
    match choice {
        ColorChoice::Auto => ColorMode::Auto,
        ColorChoice::Always => ColorMode::Always,
        ColorChoice::Never => ColorMode::Never,
    }
}

/// Load configuration from the filesystem, returning both config and metadata.
///
/// The caller is responsible for handling side-effects like printing preset info.
pub(crate) fn load_config(
    config_path: Option<&Path>,
    no_config: bool,
    no_extends: bool,
    offline: bool,
) -> crate::Result<LoadResult> {
    if no_config {
        return Ok(LoadResult {
            config: Config::default(),
            preset_used: None,
        });
    }

    // Determine project root from config path or current directory
    let project_root = config_path
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok());

    let loader = FileConfigLoader::with_options(offline, project_root);
    if no_extends {
        config_path.map_or_else(
            || loader.load_without_extends(),
            |path| loader.load_from_path_without_extends(path),
        )
    } else {
        config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))
    }
}

/// Print preset usage info to stderr (once per session managed by caller).
pub(crate) fn print_preset_info(preset_name: &str) {
    crate::output::print_info_full(
        &format!("Using preset: {preset_name}"),
        None,
        Some("Run `sloc-guard config show` to see effective settings"),
    );
}

#[must_use]
pub fn load_cache(cache_path: &Path, config_hash: &str) -> Option<Cache> {
    if !cache_path.exists() {
        return None;
    }

    Cache::load(cache_path)
        .ok()
        .filter(|cache| cache.is_valid(config_hash))
}

pub fn save_cache(cache_path: &Path, cache: &Cache) {
    // Create parent directory if needed
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // Silently ignore errors when saving cache
    let _ = cache.save(cache_path);
}

pub(crate) fn resolve_scan_paths(paths: &[PathBuf], include: &[String]) -> Vec<PathBuf> {
    // CLI --include overrides paths
    if !include.is_empty() {
        return include.iter().map(PathBuf::from).collect();
    }

    // Use provided paths (or default ".")
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
    reader: &dyn FileReader,
) -> Option<(LineStats, String)> {
    let ext = file_path.extension()?.to_str()?;
    let language = registry.get_by_extension(ext)?;

    let path_key = file_path.to_string_lossy().replace('\\', "/");

    // Get file metadata for fast cache validation
    let (mtime, size) = reader.metadata(file_path).ok()?;

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
        let (file_hash, content) = read_file_with_hash(reader, file_path)?;
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

// =============================================================================
// IO Abstraction Traits for Testability
// =============================================================================

/// Trait for reading file contents and metadata (for testability).
///
/// This trait abstracts filesystem operations to enable pure unit testing
/// without real file system access.
pub trait FileReader: Send + Sync {
    /// Read file contents as bytes.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Get file metadata (mtime in seconds since epoch, size in bytes).
    ///
    /// # Errors
    /// Returns an error if metadata cannot be retrieved.
    fn metadata(&self, path: &Path) -> io::Result<(u64, u64)>;
}

/// Real filesystem implementation of `FileReader`.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFileReader;

impl FileReader for RealFileReader {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<(u64, u64)> {
        let metadata = fs::metadata(path)?;
        let mtime = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_secs();
        let size = metadata.len();
        Ok((mtime, size))
    }
}

/// Read file contents and compute SHA-256 hash.
#[must_use]
pub fn read_file_with_hash(reader: &dyn FileReader, path: &Path) -> Option<(String, Vec<u8>)> {
    let content = reader.read(path).ok()?;
    let hash = compute_hash_from_bytes(&content);
    Some((hash, content))
}

/// Compute SHA-256 hash from bytes.
fn compute_hash_from_bytes(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

// =============================================================================
// Context Structs for Dependency Injection
// =============================================================================

/// Context for check command containing injectable dependencies.
///
/// This struct enables dependency injection for testing by encapsulating
/// the core dependencies needed for the check command. Production code uses
/// `from_config()` factory method; tests can use `new()` for custom injection.
pub struct CheckContext {
    pub registry: LanguageRegistry,
    pub threshold_checker: ThresholdChecker,
    pub structure_checker: Option<StructureChecker>,
    /// Configuration for structure-aware scanning.
    pub structure_scan_config: Option<StructureScanConfig>,
    /// Injectable file scanner for directory traversal.
    pub scanner: Box<dyn FileScanner>,
    /// Injectable file reader for content access.
    pub file_reader: Box<dyn FileReader>,
}

impl CheckContext {
    /// Create context from config (production factory).
    ///
    /// # Errors
    /// Returns error if structure checker or structure scan config initialization fails.
    pub fn from_config(
        config: &Config,
        warn_threshold: f64,
        exclude_patterns: Vec<String>,
        use_gitignore: bool,
    ) -> crate::Result<Self> {
        let registry = LanguageRegistry::with_custom_languages(&config.languages);
        let threshold_checker =
            ThresholdChecker::new(config.clone()).with_warning_threshold(warn_threshold);
        let structure_checker = StructureChecker::new(&config.structure).ok();

        // Build structure scan config for unified traversal
        let structure_scan_config = Self::build_structure_scan_config(config, &exclude_patterns)?;

        let scanner = Box::new(CompositeScanner::new(exclude_patterns, use_gitignore));
        let file_reader = Box::new(RealFileReader);

        Ok(Self {
            registry,
            threshold_checker,
            structure_checker,
            structure_scan_config,
            scanner,
            file_reader,
        })
    }

    /// Build `StructureScanConfig` from config components.
    fn build_structure_scan_config(
        config: &Config,
        exclude_patterns: &[String],
    ) -> crate::Result<Option<StructureScanConfig>> {
        // Only build if structure checking is enabled
        let has_structure_config = config.structure.max_files.is_some()
            || config.structure.max_dirs.is_some()
            || config.structure.max_depth.is_some()
            || !config.structure.rules.is_empty()
            || !config.structure.deny_extensions.is_empty()
            || !config.structure.deny_patterns.is_empty()
            || !config.structure.deny_files.is_empty()
            || !config.structure.deny_dirs.is_empty();

        if !has_structure_config {
            return Ok(None);
        }

        // Build allowlist rules from structure.rules
        let mut allowlist_rules = Vec::new();
        for rule in &config.structure.rules {
            // Include rules that have allowlists, denylists, or naming patterns
            if !rule.allow_extensions.is_empty()
                || !rule.allow_patterns.is_empty()
                || !rule.allow_files.is_empty()
                || !rule.allow_dirs.is_empty()
                || !rule.deny_extensions.is_empty()
                || !rule.deny_patterns.is_empty()
                || !rule.deny_files.is_empty()
                || !rule.deny_dirs.is_empty()
                || rule.file_naming_pattern.is_some()
            {
                let allowlist_rule = AllowlistRuleBuilder::new(rule.scope.clone())
                    .with_extensions(rule.allow_extensions.clone())
                    .with_patterns(rule.allow_patterns.clone())
                    .with_allow_files(rule.allow_files.clone())
                    .with_allow_dirs(rule.allow_dirs.clone())
                    .with_deny_extensions(rule.deny_extensions.clone())
                    .with_deny_patterns(rule.deny_patterns.clone())
                    .with_deny_files(rule.deny_files.clone())
                    .with_deny_dirs(rule.deny_dirs.clone())
                    .with_naming_pattern(rule.file_naming_pattern.clone())
                    .build()?;
                allowlist_rules.push(allowlist_rule);
            }
        }

        let structure_scan_config = StructureScanConfig::builder()
            .count_exclude(config.structure.count_exclude.clone())
            .scanner_exclude(exclude_patterns.to_vec())
            .allowlist_rules(allowlist_rules)
            .global_allow_extensions(config.structure.allow_extensions.clone())
            .global_allow_files(config.structure.allow_files.clone())
            .global_allow_dirs(config.structure.allow_dirs.clone())
            .global_deny_extensions(config.structure.deny_extensions.clone())
            .global_deny_patterns(config.structure.deny_patterns.clone())
            .global_deny_files(config.structure.deny_files.clone())
            .global_deny_dirs(config.structure.deny_dirs.clone())
            .build()?;

        Ok(Some(structure_scan_config))
    }

    /// Create context with custom components (for testing).
    #[must_use]
    pub fn new(
        registry: LanguageRegistry,
        threshold_checker: ThresholdChecker,
        structure_checker: Option<StructureChecker>,
        structure_scan_config: Option<StructureScanConfig>,
        scanner: Box<dyn FileScanner>,
        file_reader: Box<dyn FileReader>,
    ) -> Self {
        Self {
            registry,
            threshold_checker,
            structure_checker,
            structure_scan_config,
            scanner,
            file_reader,
        }
    }
}

/// Context for stats command containing injectable dependencies.
///
/// This struct enables dependency injection for testing by encapsulating
/// the core dependencies needed for the stats command.
pub struct StatsContext {
    pub registry: LanguageRegistry,
    pub allowed_extensions: HashSet<String>,
}

impl StatsContext {
    /// Create context from config (production factory).
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let registry = LanguageRegistry::with_custom_languages(&config.languages);
        let allowed_extensions = config.content.extensions.iter().cloned().collect();

        Self {
            registry,
            allowed_extensions,
        }
    }

    /// Create context with custom components (for testing).
    #[must_use]
    pub const fn new(registry: LanguageRegistry, allowed_extensions: HashSet<String>) -> Self {
        Self {
            registry,
            allowed_extensions,
        }
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
