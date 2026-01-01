use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

use crate::cache::Cache;
use crate::checker::{StructureChecker, ThresholdChecker};
use crate::cli::ColorChoice;
use crate::config::{Config, ConfigLoader, FetchPolicy, FileConfigLoader, LoadResult};
use crate::counter::{CountResult, LineStats, SlocCounter};
use crate::language::LanguageRegistry;
use crate::output::ColorMode;
use crate::scanner::{AllowlistRuleBuilder, CompositeScanner, FileScanner, StructureScanConfig};

// =============================================================================
// File Processing Error Types
// =============================================================================

/// Reason a file was skipped during processing (not an error, just no stats produced).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSkipReason {
    /// File has no extension (cannot determine language).
    NoExtension,
    /// File extension is not recognized by the language registry.
    UnrecognizedExtension(String),
    /// File was explicitly ignored via inline directive (sloc-guard: ignore-file).
    IgnoredByDirective,
}

impl fmt::Display for FileSkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoExtension => write!(f, "file has no extension"),
            Self::UnrecognizedExtension(ext) => write!(f, "unrecognized extension: .{ext}"),
            Self::IgnoredByDirective => write!(f, "ignored by sloc-guard directive"),
        }
    }
}

/// Error that occurred while trying to process a file.
#[derive(Debug)]
pub enum FileProcessError {
    /// Failed to read file metadata (mtime/size).
    MetadataError { path: PathBuf, source: io::Error },
    /// Failed to acquire cache lock.
    CacheLockError { path: PathBuf },
    /// Failed to read file contents.
    ReadError { path: PathBuf, source: io::Error },
}

impl fmt::Display for FileProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetadataError { path, source } => {
                write!(
                    f,
                    "failed to read metadata for '{}': {source}",
                    path.display()
                )
            }
            Self::CacheLockError { path } => {
                write!(f, "failed to acquire cache lock for '{}'", path.display())
            }
            Self::ReadError { path, source } => {
                write!(f, "failed to read '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for FileProcessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MetadataError { source, .. } | Self::ReadError { source, .. } => Some(source),
            Self::CacheLockError { .. } => None,
        }
    }
}

impl FileProcessError {
    /// Returns the path that caused the error.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::MetadataError { path, .. }
            | Self::CacheLockError { path }
            | Self::ReadError { path, .. } => path,
        }
    }
}

/// Result of attempting to process a single file.
#[derive(Debug)]
pub enum FileProcessResult {
    /// File was successfully processed and stats were computed.
    Success { stats: LineStats, language: String },
    /// File was legitimately skipped (not an error).
    Skipped(FileSkipReason),
    /// An error occurred while processing the file.
    Error(FileProcessError),
}

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
///
/// # Errors
/// Returns an error if:
/// - The current directory cannot be determined (when `config_path` is `None` or has no parent)
/// - The configuration file cannot be loaded
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
    let project_root = resolve_project_root(config_path)?;

    let loader = FileConfigLoader::with_options(FetchPolicy::from_offline(offline), project_root);
    if no_extends {
        config_path.map_or_else(
            || loader.load_without_extends(),
            |path| loader.load_from_path_without_extends(path),
        )
    } else {
        config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))
    }
}

/// Resolves the project root directory.
///
/// Uses the config path's parent directory if available, otherwise falls back
/// to the current working directory.
///
/// # Errors
/// Returns `SlocGuardError::Io` if `current_dir()` fails and no config path parent is available.
pub(crate) fn resolve_project_root(config_path: Option<&Path>) -> crate::Result<Option<PathBuf>> {
    // If config path has a parent, use that
    if let Some(parent) = config_path.and_then(|p| p.parent()) {
        return Ok(Some(parent.to_path_buf()));
    }

    // Fall back to current directory, propagating errors
    let cwd = std::env::current_dir().map_err(|e| crate::SlocGuardError::Io {
        source: e,
        path: None,
        operation: Some("get current directory"),
    })?;
    Ok(Some(cwd))
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

/// Save cache to disk.
///
/// Callers typically ignore errors with `let _ =` since cache is non-critical.
///
/// # Errors
/// Returns an error if the parent directory cannot be created or the cache cannot be written.
pub fn save_cache(cache_path: &Path, cache: &Cache) -> std::io::Result<()> {
    // Create parent directory if needed
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    cache
        .save(cache_path)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}

pub(crate) fn resolve_scan_paths(paths: &[PathBuf], include: &[String]) -> Vec<PathBuf> {
    // CLI --include overrides paths
    if !include.is_empty() {
        return include.iter().map(PathBuf::from).collect();
    }

    // Use provided paths (or default ".")
    paths.to_vec()
}

/// Write output to a file or stdout.
///
/// When `output_path` is `Some`, the content is written to the file (creating parent
/// directories if needed). The `quiet` flag only affects stdout output—file writes
/// always proceed regardless of this flag.
pub(crate) fn write_output(
    output_path: Option<&Path>,
    content: &str,
    quiet: bool,
) -> crate::Result<()> {
    if let Some(path) = output_path {
        // Create parent directories if needed (consistent with save_cache behavior)
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
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
///
/// Returns a `FileProcessResult` that distinguishes between:
/// - `Success`: file was processed and stats computed
/// - `Skipped`: file was legitimately skipped (no extension, unrecognized, or ignored by directive)
/// - `Error`: an IO or lock error occurred
///
/// This explicit result type allows callers to report errors rather than silently ignoring them.
pub fn process_file_with_cache(
    file_path: &Path,
    registry: &LanguageRegistry,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> FileProcessResult {
    // Check for extension
    let Some(ext_os) = file_path.extension() else {
        return FileProcessResult::Skipped(FileSkipReason::NoExtension);
    };
    let Some(ext) = ext_os.to_str() else {
        return FileProcessResult::Skipped(FileSkipReason::NoExtension);
    };

    // Check for recognized language
    let Some(language) = registry.get_by_extension(ext) else {
        return FileProcessResult::Skipped(FileSkipReason::UnrecognizedExtension(ext.to_string()));
    };

    let path_key = file_path.to_string_lossy().replace('\\', "/");

    // Get file metadata for fast cache validation
    let (mtime, size) = match reader.metadata(file_path) {
        Ok(meta) => meta,
        Err(source) => {
            return FileProcessResult::Error(FileProcessError::MetadataError {
                path: file_path.to_path_buf(),
                source,
            });
        }
    };

    // Try to get stats from cache using metadata (no file read needed)
    let cached_stats = {
        let Ok(cache_guard) = cache.lock() else {
            return FileProcessResult::Error(FileProcessError::CacheLockError {
                path: file_path.to_path_buf(),
            });
        };
        cache_guard
            .get_if_metadata_matches(&path_key, mtime, size)
            .map(|entry| LineStats::from(&entry.stats))
    };

    let stats = if let Some(stats) = cached_stats {
        stats
    } else {
        // Cache miss: read file, compute hash, and count lines
        let (file_hash, content) = match read_file_with_hash_result(reader, file_path) {
            Ok(result) => result,
            Err(source) => {
                return FileProcessResult::Error(FileProcessError::ReadError {
                    path: file_path.to_path_buf(),
                    source,
                });
            }
        };

        let counter = SlocCounter::new(&language.comment_syntax);
        let Some(result) = count_lines_from_content(&content, &counter) else {
            // File was ignored by directive
            return FileProcessResult::Skipped(FileSkipReason::IgnoredByDirective);
        };

        // Update cache with metadata (lock errors here are non-critical, just skip update)
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.set(&path_key, file_hash, &result, mtime, size);
        }

        result
    };

    FileProcessResult::Success {
        stats,
        language: language.name.clone(),
    }
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
///
/// Returns `None` on read error. For explicit error handling, use `read_file_with_hash_result`.
#[must_use]
pub fn read_file_with_hash(reader: &dyn FileReader, path: &Path) -> Option<(String, Vec<u8>)> {
    read_file_with_hash_result(reader, path).ok()
}

/// Read file contents and compute SHA-256 hash, returning explicit errors.
///
/// # Errors
/// Returns an error if the file cannot be read.
pub fn read_file_with_hash_result(
    reader: &dyn FileReader,
    path: &Path,
) -> io::Result<(String, Vec<u8>)> {
    let content = reader.read(path)?;
    let hash = compute_hash_from_bytes(&content);
    Ok((hash, content))
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
            ThresholdChecker::new(config.clone())?.with_warning_threshold(warn_threshold);
        let structure_checker = Some(StructureChecker::new(&config.structure)?);

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
        if !config.structure.is_enabled() {
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
