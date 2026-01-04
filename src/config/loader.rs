//! Config loading from filesystem and extends resolution.
//!
//! Provides the main API for loading sloc-guard configuration from files,
//! with support for config inheritance via `extends`.

use std::path::{Path, PathBuf};

use indexmap::IndexSet;

use crate::error::{ConfigSource, Result, SlocGuardError};

use super::Config;
use super::extends::ExtendsResolver;
use super::filesystem::FileSystem;
use super::merge::{strip_reset_markers, validate_reset_positions};
use super::model::CONFIG_VERSION;
use super::remote::FetchPolicy;

// Re-export types that are part of the loader's public API
pub use super::extends::SourcedConfig;
pub use super::filesystem::RealFileSystem;

// Re-export for tests only
#[cfg(test)]
pub use super::extends::MAX_EXTENDS_DEPTH;

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

    /// Create an extends resolver for this loader.
    fn resolver(&self) -> ExtendsResolver<'_, F> {
        ExtendsResolver::new(&self.fs, self.fetch_policy, self.project_root.as_deref())
    }

    /// Continue extends chain with pre-parsed value (non-tracking variant).
    fn load_with_extends_from_value(
        &self,
        path: &Path,
        config_value: toml::Value,
        visited: &mut IndexSet<String>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        self.resolver()
            .load_with_extends_from_value(path, config_value, visited, None, depth)
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
        self.resolver().load_with_extends_from_value(
            path,
            config_value,
            visited,
            Some(sources),
            depth,
        )
    }
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
        let value = ExtendsResolver::<F>::parse_value_with_location(&content, Some(source))?;
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
        let value = ExtendsResolver::<F>::parse_value_with_location(&content, Some(source))?;
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
        let value = ExtendsResolver::<F>::parse_value_with_location(&content, Some(source))?;
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
        let value = ExtendsResolver::<F>::parse_value_with_location(&content, Some(source))?;
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
