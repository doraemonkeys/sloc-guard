use std::path::{Path, PathBuf};

use indexmap::IndexSet;

use crate::error::{ConfigSource, Result, SlocGuardError};

use super::Config;
use super::model::CONFIG_VERSION;
use super::presets;
use super::remote::{FetchPolicy, fetch_remote_config, is_remote_url};

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

    /// Load config with extends chain, returning (value, `preset_used`).
    fn load_with_extends(
        &self,
        path: &Path,
        visited: &mut IndexSet<String>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;

        self.load_with_extends_from_content(path, &content, visited, depth)
    }

    /// Continue extends chain with pre-read content (avoids re-reading the file).
    fn load_with_extends_from_content(
        &self,
        path: &Path,
        content: &str,
        visited: &mut IndexSet<String>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let source = ConfigSource::file(path);
        let config_value = Self::parse_value_with_location(content, Some(source))?;
        self.load_with_extends_from_value(path, config_value, visited, depth)
    }

    /// Continue extends chain with pre-parsed value (avoids re-parsing).
    fn load_with_extends_from_value(
        &self,
        path: &Path,
        config_value: toml::Value,
        visited: &mut IndexSet<String>,
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
            // IndexSet preserves insertion order, so chain shows the actual traversal sequence
            let mut chain: Vec<String> = visited.iter().cloned().collect();
            chain.push(key);
            return Err(SlocGuardError::CircularExtends { chain });
        }

        self.process_config_value(config_value, Some(path), visited, depth)
    }

    /// Load remote config with extends chain, returning (value, `preset_used`).
    fn load_remote_with_extends(
        &self,
        url: &str,
        expected_hash: Option<&str>,
        visited: &mut IndexSet<String>,
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
            // IndexSet preserves insertion order, so chain shows the actual traversal sequence
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
        self.process_config_content(&content, Some(source), None, visited, depth)
    }

    /// Process config content and return (value, `preset_used`).
    ///
    /// # Arguments
    /// - `content`: Raw TOML content
    /// - `source`: Config source for error reporting (file/remote/preset)
    /// - `base_path`: Optional file path for resolving relative extends
    /// - `visited`: Set of visited configs for cycle detection
    /// - `depth`: Current depth in extends chain
    fn process_config_content(
        &self,
        content: &str,
        source: Option<ConfigSource>,
        base_path: Option<&Path>,
        visited: &mut IndexSet<String>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let config_value = Self::parse_value_with_location(content, source)?;
        self.process_config_value(config_value, base_path, visited, depth)
    }

    /// Process pre-parsed config value and return (value, `preset_used`).
    ///
    /// # Arguments
    /// - `config_value`: Pre-parsed TOML value
    /// - `base_path`: Optional file path for resolving relative extends
    /// - `visited`: Set of visited configs for cycle detection
    /// - `depth`: Current depth in extends chain
    fn process_config_value(
        &self,
        mut config_value: toml::Value,
        base_path: Option<&Path>,
        visited: &mut IndexSet<String>,
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

        let mut preset_used = None;

        if let Some(extends) = extends_value {
            let (base_value, child_preset) =
                if let Some(preset_name) = extends.strip_prefix("preset:") {
                    // Track which preset was used (caller decides whether/how to notify user)
                    // Presets are terminal - they don't count toward depth
                    preset_used = Some(preset_name.to_string());
                    (presets::load_preset(preset_name)?, None)
                } else if is_remote_url(&extends) {
                    self.load_remote_with_extends(
                        &extends,
                        extends_sha256.as_deref(),
                        visited,
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
                    self.load_with_extends(&resolved_path, visited, depth + 1)?
                };
            // Propagate preset from child if we didn't find one at this level
            if preset_used.is_none() {
                preset_used = child_preset;
            }
            config_value = merge_toml_values(base_value, config_value);
        }

        if let Some(table) = config_value.as_table_mut() {
            table.remove("extends");
            table.remove("extends_sha256");
        }

        // Validate $reset marker positions before stripping
        validate_reset_positions(&config_value, "")?;

        // Strip any remaining $reset markers (for configs without extends)
        strip_reset_markers(&mut config_value);

        Ok((config_value, preset_used))
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
}

#[cfg(test)]
#[path = "loader_tests/mod.rs"]
mod tests;
