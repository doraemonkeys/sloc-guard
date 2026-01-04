use std::path::{Path, PathBuf};

use indexmap::IndexSet;

use crate::error::{ConfigSource, Result, SlocGuardError};

use super::Config;
use super::model::CONFIG_VERSION;
use super::presets;
use super::remote::{FetchPolicy, fetch_remote_config, is_remote_url};

/// A configuration value paired with its source.
///
/// Used during extends resolution to track which config contributed which values,
/// enabling the `explain --sources` feature to show field origins.
#[derive(Debug, Clone)]
pub struct SourcedConfig {
    /// The source of this configuration (file, remote URL, or preset).
    pub source: ConfigSource,
    /// The raw TOML value before merging.
    pub value: toml::Value,
}

/// Result of loading a configuration, containing both the config and metadata.
///
/// This allows the caller to decide how to handle loading side-effects (like printing
/// preset info) rather than coupling the loader to the output module.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadResult {
    /// The loaded configuration.
    pub config: Config,
    /// The preset name if a preset was used (e.g., "rust-strict").
    pub preset_used: Option<String>,
}

/// Result of loading a configuration with full source tracking.
///
/// Used by `explain --sources` to show which config contributed which settings.
#[derive(Debug, Clone)]
pub struct LoadResultWithSources {
    /// The loaded configuration.
    pub config: Config,
    /// The preset name if a preset was used (e.g., "rust-strict").
    pub preset_used: Option<String>,
    /// The inheritance chain from base to child (root → leaf).
    /// First element is the deepest base (e.g., preset), last is the local config.
    pub source_chain: Vec<SourcedConfig>,
}

/// Trait for loading configuration from various sources.
pub trait ConfigLoader {
    /// Load configuration from the default location.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    fn load(&self) -> Result<LoadResult>;

    /// Load configuration from a specific path.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    fn load_from_path(&self, path: &Path) -> Result<LoadResult>;

    /// Load configuration without resolving extends.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    fn load_without_extends(&self) -> Result<LoadResult>;

    /// Load configuration from a specific path without resolving extends.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    fn load_from_path_without_extends(&self, path: &Path) -> Result<LoadResult>;

    /// Load configuration with full source tracking.
    ///
    /// Returns the merged config along with the source chain showing which config
    /// contributed which values. Used by `explain --sources`.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    fn load_with_sources(&self) -> Result<LoadResultWithSources>;

    /// Load configuration from a specific path with full source tracking.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    fn load_from_path_with_sources(&self, path: &Path) -> Result<LoadResultWithSources>;

    /// Load configuration without resolving extends, with source tracking.
    ///
    /// Returns the config with a single-element source chain (the local file only).
    /// Used by `explain --sources --no-extends`.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    fn load_without_extends_with_sources(&self) -> Result<LoadResultWithSources>;

    /// Load configuration from a specific path without resolving extends, with source tracking.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    fn load_from_path_without_extends_with_sources(
        &self,
        path: &Path,
    ) -> Result<LoadResultWithSources>;
}

const LOCAL_CONFIG_NAME: &str = ".sloc-guard.toml";
const USER_CONFIG_NAME: &str = "config.toml";

/// Maximum depth of extends chain to prevent stack overflow from deeply nested configs.
///
/// Depth starts at 0 (the initial config) and increments with each `extends` resolution.
/// A value of 10 allows depths 0..=10 inclusive, meaning a chain of up to 11 config files.
pub const MAX_EXTENDS_DEPTH: usize = 10;

/// Validate config version. Returns an error if version is unsupported.
fn validate_config_version(config: &Config) -> Result<()> {
    match &config.version {
        None => Ok(()),                           // No version specified - use defaults
        Some(v) if v == CONFIG_VERSION => Ok(()), // V2 - valid
        Some(v) => Err(SlocGuardError::Config(format!(
            "Unsupported config version '{v}'. Only version '{CONFIG_VERSION}' is supported. \
             Please update your configuration to the V2 format."
        ))),
    }
}

/// Trait for filesystem operations (for testability).
pub trait FileSystem {
    /// Read file contents as a string.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read.
    fn read_to_string(&self, path: &Path) -> std::io::Result<String>;

    /// Check if a path exists.
    fn exists(&self, path: &Path) -> bool;

    /// Get the current working directory.
    ///
    /// # Errors
    /// Returns an error if the current directory cannot be determined.
    fn current_dir(&self) -> std::io::Result<PathBuf>;

    /// Get the platform-specific configuration directory for sloc-guard.
    ///
    /// Returns the appropriate config directory based on platform conventions:
    /// - Windows: `%APPDATA%\sloc-guard` (e.g., `C:\Users\xxx\AppData\Roaming\sloc-guard`)
    /// - macOS: `~/Library/Application Support/sloc-guard`
    /// - Linux: `~/.config/sloc-guard` (XDG)
    fn config_dir(&self) -> Option<PathBuf>;

    /// Canonicalize a path to its absolute, normalized form.
    ///
    /// # Errors
    /// Returns an error if the path cannot be canonicalized (e.g., file doesn't exist).
    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf>;
}

/// Real filesystem implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        std::env::current_dir()
    }

    fn config_dir(&self) -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "sloc-guard")
            .map(|dirs| dirs.config_dir().to_path_buf())
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        path.canonicalize()
    }
}

/// Loads configuration from the filesystem.
///
/// Search order:
/// 1. `.sloc-guard.toml` in current directory
/// 2. Platform-specific user config directory:
///    - Windows: `%APPDATA%\sloc-guard\config.toml`
///    - macOS: `~/Library/Application Support/sloc-guard/config.toml`
///    - Linux: `~/.config/sloc-guard/config.toml` (XDG)
/// 3. Returns `Config::default()` if no config found
#[derive(Debug)]
pub struct FileConfigLoader<F: FileSystem = RealFileSystem> {
    fs: F,
    fetch_policy: FetchPolicy,
    project_root: Option<PathBuf>,
}

impl Default for FileConfigLoader<RealFileSystem> {
    fn default() -> Self {
        Self::new()
    }
}

impl FileConfigLoader<RealFileSystem> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fs: RealFileSystem,
            fetch_policy: FetchPolicy::Normal,
            project_root: None,
        }
    }

    /// Create a loader with fetch policy and project root options.
    #[must_use]
    pub const fn with_options(fetch_policy: FetchPolicy, project_root: Option<PathBuf>) -> Self {
        Self {
            fs: RealFileSystem,
            fetch_policy,
            project_root,
        }
    }
}

impl<F: FileSystem> FileConfigLoader<F> {
    #[must_use]
    pub const fn with_fs(fs: F) -> Self {
        Self {
            fs,
            fetch_policy: FetchPolicy::Normal,
            project_root: None,
        }
    }

    fn local_config_path(&self) -> Option<PathBuf> {
        self.fs
            .current_dir()
            .ok()
            .map(|dir| dir.join(LOCAL_CONFIG_NAME))
    }

    fn user_config_path(&self) -> Option<PathBuf> {
        self.fs.config_dir().map(|dir| dir.join(USER_CONFIG_NAME))
    }

    fn parse_config(content: &str) -> Result<Config> {
        let config: Config = toml::from_str(content).map_err(SlocGuardError::from)?;
        validate_config_version(&config)?;
        Ok(config)
    }

    /// Finalize a parsed TOML value into a Config.
    ///
    /// Validates `$reset` marker positions, strips markers, and parses to Config.
    fn finalize_value_to_config(mut value: toml::Value) -> Result<Config> {
        validate_reset_positions(&value, "")?;
        strip_reset_markers(&mut value);
        let config_str =
            toml::to_string(&value).map_err(|e| SlocGuardError::Config(e.to_string()))?;
        Self::parse_config(&config_str)
    }

    /// Parse content to `toml::Value` with precise syntax error reporting.
    fn parse_value_with_location(
        content: &str,
        source: Option<ConfigSource>,
    ) -> Result<toml::Value> {
        toml::from_str(content).map_err(|e| SlocGuardError::syntax_from_toml(&e, content, source))
    }

    // =========================================================================
    // Unified internal methods with optional source tracking
    // =========================================================================

    /// Load config with extends chain, optionally tracking sources.
    ///
    /// When `sources` is `Some`, records each config source in the chain for
    /// `explain --sources` output. When `None`, skips source tracking overhead.
    fn load_with_extends_impl(
        &self,
        path: &Path,
        visited: &mut IndexSet<String>,
        sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;

        self.load_with_extends_from_content_impl(path, &content, visited, sources, depth)
    }

    /// Continue extends chain with pre-read content, optionally tracking sources.
    fn load_with_extends_from_content_impl(
        &self,
        path: &Path,
        content: &str,
        visited: &mut IndexSet<String>,
        sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let source = ConfigSource::file(path);
        let config_value = Self::parse_value_with_location(content, Some(source))?;
        self.load_with_extends_from_value_impl(path, config_value, visited, sources, depth)
    }

    /// Continue extends chain with pre-parsed value, optionally tracking sources.
    fn load_with_extends_from_value_impl(
        &self,
        path: &Path,
        config_value: toml::Value,
        visited: &mut IndexSet<String>,
        sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        if depth > MAX_EXTENDS_DEPTH {
            return Err(SlocGuardError::ExtendsTooDeep {
                depth,
                max: MAX_EXTENDS_DEPTH,
                chain: visited.iter().cloned().collect(),
            });
        }

        let canonical =
            self.fs
                .canonicalize(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;
        let key = canonical.to_string_lossy().to_string();

        if !visited.insert(key.clone()) {
            let mut chain: Vec<String> = visited.iter().cloned().collect();
            chain.push(key);
            return Err(SlocGuardError::CircularExtends { chain });
        }

        self.process_config_value_impl(config_value, Some(path), visited, sources, depth)
    }

    /// Load remote config with extends chain, optionally tracking sources.
    fn load_remote_with_extends_impl(
        &self,
        url: &str,
        expected_hash: Option<&str>,
        visited: &mut IndexSet<String>,
        mut sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        if depth > MAX_EXTENDS_DEPTH {
            return Err(SlocGuardError::ExtendsTooDeep {
                depth,
                max: MAX_EXTENDS_DEPTH,
                chain: visited.iter().cloned().collect(),
            });
        }

        let url_key = url.to_string();
        if !visited.insert(url_key.clone()) {
            let mut chain: Vec<String> = visited.iter().cloned().collect();
            chain.push(url_key);
            return Err(SlocGuardError::CircularExtends { chain });
        }

        let content = fetch_remote_config(
            url,
            self.project_root.as_deref(),
            expected_hash,
            self.fetch_policy,
        )?;
        let source = ConfigSource::remote(url);
        let config_value = Self::parse_value_with_location(&content, Some(source))?;

        // Process extends chain (base sources come first via recursion)
        // Clone only when tracking sources; move otherwise for efficiency
        if sources.is_some() {
            let (merged_value, preset_used) = self.process_config_value_impl(
                config_value.clone(),
                None, // No base_path for remote configs
                visited,
                sources.as_deref_mut(),
                depth,
            )?;

            // Record remote source (after recursion, so base sources come first)
            // INVARIANT: sources was Some, unwrap is safe
            sources.as_mut().unwrap().push(SourcedConfig {
                source: ConfigSource::remote(url),
                value: config_value,
            });

            Ok((merged_value, preset_used))
        } else {
            self.process_config_value_impl(
                config_value, // Move, no clone needed
                None,
                visited,
                None,
                depth,
            )
        }
    }

    /// Process pre-parsed config value, optionally tracking sources.
    ///
    /// This is the core method that handles extends resolution and optional source collection.
    /// When `sources` is `Some`, records each config in the inheritance chain.
    //
    // Note: We use `as_mut().map(|v| &mut **v)` instead of `as_deref_mut()` because we need
    // `Option<&mut Vec<T>>` (for `push()`), not `Option<&mut [T]>` (slice) which `as_deref_mut` returns.
    // The `useless_let_if_seq` lint is suppressed because the conditional mutation pattern
    // is clearer with explicit `let mut` for this complex extends resolution logic.
    #[allow(clippy::option_as_ref_deref, clippy::useless_let_if_seq)]
    fn process_config_value_impl(
        &self,
        config_value: toml::Value,
        base_path: Option<&Path>,
        visited: &mut IndexSet<String>,
        mut sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let extends_value = config_value
            .get("extends")
            .and_then(toml::Value::as_str)
            .map(String::from);

        let extends_sha256 = config_value
            .get("extends_sha256")
            .and_then(toml::Value::as_str)
            .map(String::from);

        let (mut merged_value, preset_used) = if let Some(extends) = extends_value {
            let (base_value, preset_used) =
                if let Some(preset_name) = extends.strip_prefix("preset:") {
                    let preset_value = presets::load_preset(preset_name)?;
                    // Record preset source if tracking
                    if let Some(src_vec) = sources.as_mut() {
                        src_vec.push(SourcedConfig {
                            source: ConfigSource::preset(preset_name),
                            value: preset_value.clone(),
                        });
                    }
                    (preset_value, Some(preset_name.to_string()))
                } else if is_remote_url(&extends) {
                    self.load_remote_with_extends_impl(
                        &extends,
                        extends_sha256.as_deref(),
                        visited,
                        sources.as_deref_mut(),
                        depth + 1,
                    )?
                } else {
                    let extends_path = Path::new(&extends);
                    let resolved_path = if extends_path.is_absolute() {
                        extends_path.to_path_buf()
                    } else if let Some(base) = base_path {
                        base.parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(extends_path)
                    } else {
                        return Err(SlocGuardError::ExtendsResolution {
                            path: extends,
                            base: "remote config".to_string(),
                        });
                    };
                    self.load_with_extends_impl(
                        &resolved_path,
                        visited,
                        sources.as_deref_mut(),
                        depth + 1,
                    )?
                };
            // Record source before merge (clone only when tracking is needed)
            if let Some(src_vec) = sources.as_mut()
                && let Some(path) = base_path
            {
                src_vec.push(SourcedConfig {
                    source: ConfigSource::file(path),
                    value: config_value.clone(),
                });
            }
            (merge_toml_values(base_value, config_value), preset_used)
        } else {
            // Record source before return (clone only when tracking is needed)
            if let Some(src_vec) = sources.as_mut()
                && let Some(path) = base_path
            {
                src_vec.push(SourcedConfig {
                    source: ConfigSource::file(path),
                    value: config_value.clone(),
                });
            }
            (config_value, None)
        };

        if let Some(table) = merged_value.as_table_mut() {
            table.remove("extends");
            table.remove("extends_sha256");
        }

        validate_reset_positions(&merged_value, "")?;
        strip_reset_markers(&mut merged_value);

        Ok((merged_value, preset_used))
    }

    // =========================================================================
    // Convenience wrappers delegating to unified implementation
    // =========================================================================

    /// Continue extends chain with pre-parsed value (non-tracking variant).
    fn load_with_extends_from_value(
        &self,
        path: &Path,
        config_value: toml::Value,
        visited: &mut IndexSet<String>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        self.load_with_extends_from_value_impl(path, config_value, visited, None, depth)
    }

    /// Continue extends chain with pre-parsed value, tracking sources.
    fn load_with_extends_from_value_tracking(
        &self,
        path: &Path,
        config_value: toml::Value,
        visited: &mut IndexSet<String>,
        sources: &mut Vec<SourcedConfig>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        self.load_with_extends_from_value_impl(path, config_value, visited, Some(sources), depth)
    }
}

/// The reset marker used to clear parent arrays during merge.
pub const RESET_MARKER: &str = "$reset";

/// Merge two TOML values. Child values take precedence.
/// Tables are merged recursively. Arrays are appended (parent + child),
/// unless child array starts with `$reset` marker which clears the parent.
fn merge_toml_values(base: toml::Value, child: toml::Value) -> toml::Value {
    match (base, child) {
        (toml::Value::Table(mut base_table), toml::Value::Table(child_table)) => {
            for (key, child_val) in child_table {
                match base_table.remove(&key) {
                    Some(base_val) => {
                        base_table.insert(key, merge_toml_values(base_val, child_val));
                    }
                    None => {
                        base_table.insert(key, child_val);
                    }
                }
            }
            toml::Value::Table(base_table)
        }
        (toml::Value::Array(base_arr), toml::Value::Array(child_arr)) => {
            merge_arrays(base_arr, child_arr)
        }
        (_, child) => child,
    }
}

/// Merge two arrays. Appends by default, but if child starts with `$reset`,
/// clears the parent and uses remaining child elements.
///
/// Note: The reset marker is intentionally handled in two places:
/// 1. Here during parent-child merge (skips marker, discards parent)
/// 2. In `strip_reset_markers()` for standalone configs without extends
pub fn merge_arrays(base: Vec<toml::Value>, mut child: Vec<toml::Value>) -> toml::Value {
    if has_reset_marker(&child) {
        // Reset: remove the marker, discard parent, use remaining child elements
        child.remove(0);
        toml::Value::Array(child)
    } else {
        // Append: parent + child
        let mut merged = base;
        merged.extend(child);
        toml::Value::Array(merged)
    }
}

/// Check if an array starts with a reset marker.
/// - For string arrays: first element is "$reset"
/// - For table arrays (rules): first element has `pattern = "$reset"` or `scope = "$reset"`
pub fn has_reset_marker(arr: &[toml::Value]) -> bool {
    arr.first().is_some_and(is_reset_element)
}

/// Check if a value is a reset marker element.
pub fn is_reset_element(value: &toml::Value) -> bool {
    match value {
        toml::Value::String(s) => s == RESET_MARKER,
        toml::Value::Table(table) => {
            // For content.rules: check pattern field
            // For structure.rules: check scope field
            table
                .get("pattern")
                .or_else(|| table.get("scope"))
                .and_then(toml::Value::as_str)
                .is_some_and(|s| s == RESET_MARKER)
        }
        _ => false,
    }
}

/// Strip remaining reset markers from the merged config value.
///
/// This handles the case where a config has `$reset` but no parent extends.
/// Note: `merge_arrays()` also removes the marker during parent-child merge;
/// this function catches markers in standalone configs without extends chain.
fn strip_reset_markers(value: &mut toml::Value) {
    match value {
        toml::Value::Table(table) => {
            for (_, val) in table.iter_mut() {
                strip_reset_markers(val);
            }
        }
        toml::Value::Array(arr) => {
            // Remove reset marker if it's the first element
            if arr.first().is_some_and(is_reset_element) {
                arr.remove(0);
            }
            // Recursively strip from nested values
            for val in arr {
                strip_reset_markers(val);
            }
        }
        _ => {}
    }
}

/// Validate that `$reset` markers are only in first position of arrays.
/// Returns an error if `$reset` is found in any position other than first.
fn validate_reset_positions(value: &toml::Value, path: &str) -> Result<()> {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                validate_reset_positions(val, &child_path)?;
            }
        }
        toml::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                if i > 0 && is_reset_element(val) {
                    return Err(SlocGuardError::Config(format!(
                        "'{RESET_MARKER}' must be the first element in array '{path}', found at position {i}"
                    )));
                }
                // Recursively validate nested values
                validate_reset_positions(val, path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl<F: FileSystem> ConfigLoader for FileConfigLoader<F> {
    fn load(&self) -> Result<LoadResult> {
        if let Some(local_path) = self.local_config_path()
            && self.fs.exists(&local_path)
        {
            return self.load_from_path(&local_path);
        }

        if let Some(user_path) = self.user_config_path()
            && self.fs.exists(&user_path)
        {
            return self.load_from_path(&user_path);
        }

        Ok(LoadResult {
            config: Config::default(),
            preset_used: None,
        })
    }

    fn load_from_path(&self, path: &Path) -> Result<LoadResult> {
        // Dual-path loading: use precise line numbers for single-file,
        // source chain tracking for inheritance mode
        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;

        let source = ConfigSource::file(path);

        // Parse once and check for extends
        let value = Self::parse_value_with_location(&content, Some(source))?;
        let has_extends = value.get("extends").is_some();

        if has_extends {
            // Inheritance mode: pass pre-parsed value to avoid re-parsing
            // Line numbers after merge are meaningless, use source chain tracking
            let mut visited = IndexSet::new();
            let (merged_value, preset_used) =
                self.load_with_extends_from_value(path, value, &mut visited, 0)?;
            let config = Self::finalize_value_to_config(merged_value)?;
            Ok(LoadResult {
                config,
                preset_used,
            })
        } else {
            // Single-file mode: finalize and parse
            let config = Self::finalize_value_to_config(value)?;
            Ok(LoadResult {
                config,
                preset_used: None,
            })
        }
    }

    fn load_without_extends(&self) -> Result<LoadResult> {
        if let Some(local_path) = self.local_config_path()
            && self.fs.exists(&local_path)
        {
            return self.load_from_path_without_extends(&local_path);
        }

        if let Some(user_path) = self.user_config_path()
            && self.fs.exists(&user_path)
        {
            return self.load_from_path_without_extends(&user_path);
        }

        Ok(LoadResult {
            config: Config::default(),
            preset_used: None,
        })
    }

    fn load_from_path_without_extends(&self, path: &Path) -> Result<LoadResult> {
        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;
        // Single-file mode: use precise syntax error reporting
        let source = ConfigSource::file(path);
        let value = Self::parse_value_with_location(&content, Some(source))?;
        let config = Self::finalize_value_to_config(value)?;
        Ok(LoadResult {
            config,
            preset_used: None,
        })
    }

    fn load_with_sources(&self) -> Result<LoadResultWithSources> {
        if let Some(local_path) = self.local_config_path()
            && self.fs.exists(&local_path)
        {
            return self.load_from_path_with_sources(&local_path);
        }

        if let Some(user_path) = self.user_config_path()
            && self.fs.exists(&user_path)
        {
            return self.load_from_path_with_sources(&user_path);
        }

        // No config file found - return default with empty source chain
        Ok(LoadResultWithSources {
            config: Config::default(),
            preset_used: None,
            source_chain: vec![],
        })
    }

    fn load_from_path_with_sources(&self, path: &Path) -> Result<LoadResultWithSources> {
        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;

        let source = ConfigSource::file(path);
        let value = Self::parse_value_with_location(&content, Some(source))?;
        let has_extends = value.get("extends").is_some();

        if has_extends {
            // Inheritance mode with source tracking
            let mut visited = IndexSet::new();
            let mut sources = Vec::new();
            let (merged_value, preset_used) = self.load_with_extends_from_value_tracking(
                path,
                value,
                &mut visited,
                &mut sources,
                0,
            )?;
            let config = Self::finalize_value_to_config(merged_value)?;
            Ok(LoadResultWithSources {
                config,
                preset_used,
                source_chain: sources,
            })
        } else {
            // Single-file mode - just this file as source
            let config = Self::finalize_value_to_config(value.clone())?;
            Ok(LoadResultWithSources {
                config,
                preset_used: None,
                source_chain: vec![SourcedConfig {
                    source: ConfigSource::file(path),
                    value,
                }],
            })
        }
    }

    fn load_without_extends_with_sources(&self) -> Result<LoadResultWithSources> {
        if let Some(local_path) = self.local_config_path()
            && self.fs.exists(&local_path)
        {
            return self.load_from_path_without_extends_with_sources(&local_path);
        }

        if let Some(user_path) = self.user_config_path()
            && self.fs.exists(&user_path)
        {
            return self.load_from_path_without_extends_with_sources(&user_path);
        }

        // No config file found - return default with empty source chain
        Ok(LoadResultWithSources {
            config: Config::default(),
            preset_used: None,
            source_chain: vec![],
        })
    }

    fn load_from_path_without_extends_with_sources(
        &self,
        path: &Path,
    ) -> Result<LoadResultWithSources> {
        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;

        let source = ConfigSource::file(path);
        let value = Self::parse_value_with_location(&content, Some(source))?;
        // Single-file mode: ignore extends, return only this file as source
        let config = Self::finalize_value_to_config(value.clone())?;
        Ok(LoadResultWithSources {
            config,
            preset_used: None,
            source_chain: vec![SourcedConfig {
                source: ConfigSource::file(path),
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "loader_tests/mod.rs"]
mod tests;
