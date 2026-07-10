//! Config loading from filesystem and extends resolution.
//!
//! Provides the main API for loading sloc-guard configuration from files,
//! with support for config inheritance via `extends`.

use std::path::{Path, PathBuf};

use indexmap::IndexSet;

use crate::error::{ConfigSource, Result, SlocGuardError};
use crate::project::lexical_absolute;

use super::Config;
use super::extends::ExtendsResolver;
use super::filesystem::FileSystem;
use super::merge::{has_any_reset_markers, strip_reset_markers, validate_reset_positions};
use super::model::CONFIG_VERSION;
use super::remote::FetchPolicy;

// Re-export types that are part of the loader's public API
pub use super::extends::SourcedConfig;
pub use super::filesystem::RealFileSystem;

// Re-export for tests only
#[cfg(test)]
pub use super::extends::MAX_EXTENDS_DEPTH;

/// Describes where the effective configuration came from.
///
/// File-backed variants contain an absolute, lexically normalized path. Symlinks are intentionally
/// not resolved: relative `extends` entries and path rules are anchored at the selected config
/// location rather than the symlink target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigOrigin {
    /// A `.sloc-guard.toml` discovered in the current directory or an ancestor.
    ProjectFile(PathBuf),
    /// The platform-specific user configuration file.
    UserFile(PathBuf),
    /// A configuration file explicitly selected by the caller.
    ExplicitFile(PathBuf),
    /// No configuration file was found, so built-in defaults are active.
    BuiltInDefaults,
    /// Configuration loading was explicitly disabled by the caller.
    ///
    /// The filesystem loader never produces this variant; it is provided for
    /// command-layer options such as `--no-config`.
    Disabled,
}

impl ConfigOrigin {
    /// Return the backing file path, if this origin is file-backed.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::ProjectFile(path) | Self::UserFile(path) | Self::ExplicitFile(path) => Some(path),
            Self::BuiltInDefaults | Self::Disabled => None,
        }
    }
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

/// Command-layer load result with source metadata used for stable path resolution and notices.
///
/// This stays crate-private so adding provenance does not break the public [`LoadResult`] struct.
#[derive(Debug, Clone, PartialEq)]
pub struct LocatedLoadResult {
    pub config: Config,
    pub preset_used: Option<String>,
    pub origin: ConfigOrigin,
}

impl LocatedLoadResult {
    fn from_public(result: LoadResult, origin: ConfigOrigin) -> Self {
        Self {
            config: result.config,
            preset_used: result.preset_used,
            origin,
        }
    }

    fn into_public(self) -> LoadResult {
        LoadResult {
            config: self.config,
            preset_used: self.preset_used,
        }
    }
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
/// 1. The nearest `.sloc-guard.toml`, walking up from the current directory
///    (the walk stops at a `.git` file or directory that has no config)
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

    fn user_config_path(&self) -> Option<PathBuf> {
        self.fs.config_dir().map(|dir| dir.join(USER_CONFIG_NAME))
    }

    /// Convert a path to the stable lexical form exposed in load metadata.
    ///
    /// Do not canonicalize here. Resolving a config symlink would change the base directory for
    /// relative `extends` entries and make implicit and explicit loading of the same path disagree.
    fn stable_path(&self, path: &Path) -> PathBuf {
        self.fs
            .current_dir()
            .map_or_else(|_| path.to_path_buf(), |cwd| lexical_absolute(path, &cwd))
    }

    /// Discover the effective configuration source for all no-argument load
    /// entry points.
    ///
    /// At each ancestor the project config is checked before the Git marker,
    /// so a config at the repository root is selected. A `.git` file (linked
    /// worktree/submodule) is intentionally the same boundary as a directory.
    fn discover_origin(&self) -> Result<ConfigOrigin> {
        let current_dir = self.fs.current_dir()?;
        let start = self.stable_path(&current_dir);
        for ancestor in start.ancestors() {
            let project_path = ancestor.join(LOCAL_CONFIG_NAME);
            if self.fs.exists(&project_path) {
                return Ok(ConfigOrigin::ProjectFile(self.stable_path(&project_path)));
            }

            if self.fs.exists(&ancestor.join(".git")) {
                break;
            }
        }

        if let Some(user_path) = self.user_config_path()
            && self.fs.exists(&user_path)
        {
            return Ok(ConfigOrigin::UserFile(self.stable_path(&user_path)));
        }

        Ok(ConfigOrigin::BuiltInDefaults)
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

    /// Parse config from content or value, depending on presence of reset markers.
    ///
    /// - With reset markers: validates positions, strips markers, parses via Value
    /// - Without reset markers: parses directly from content for precise line numbers
    fn parse_config_with_reset_handling(content: &str, value: toml::Value) -> Result<Config> {
        if has_any_reset_markers(&value) {
            Self::finalize_value_to_config(value)
        } else {
            Self::parse_config(content)
        }
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

    /// Load the implicitly selected configuration together with its filesystem origin.
    pub(crate) fn load_located(&self) -> Result<LocatedLoadResult> {
        let origin = self.discover_origin()?;
        if let Some(path) = origin.path() {
            let result = <Self as ConfigLoader>::load_from_path(self, path)?;
            return Ok(LocatedLoadResult::from_public(result, origin));
        }

        Ok(LocatedLoadResult {
            config: Config::default(),
            preset_used: None,
            origin,
        })
    }

    /// Load an explicit configuration together with its lexical filesystem origin.
    pub(crate) fn load_from_path_located(&self, path: &Path) -> Result<LocatedLoadResult> {
        let result = <Self as ConfigLoader>::load_from_path(self, path)?;
        Ok(LocatedLoadResult::from_public(
            result,
            ConfigOrigin::ExplicitFile(self.stable_path(path)),
        ))
    }

    /// Load the implicitly selected configuration without inheritance, retaining its origin.
    pub(crate) fn load_without_extends_located(&self) -> Result<LocatedLoadResult> {
        let origin = self.discover_origin()?;
        if let Some(path) = origin.path() {
            let result = <Self as ConfigLoader>::load_from_path_without_extends(self, path)?;
            return Ok(LocatedLoadResult::from_public(result, origin));
        }

        Ok(LocatedLoadResult {
            config: Config::default(),
            preset_used: None,
            origin,
        })
    }

    /// Load an explicit configuration without inheritance, retaining its lexical origin.
    pub(crate) fn load_from_path_without_extends_located(
        &self,
        path: &Path,
    ) -> Result<LocatedLoadResult> {
        let result = <Self as ConfigLoader>::load_from_path_without_extends(self, path)?;
        Ok(LocatedLoadResult::from_public(
            result,
            ConfigOrigin::ExplicitFile(self.stable_path(path)),
        ))
    }
}

impl<F: FileSystem> ConfigLoader for FileConfigLoader<F> {
    fn load(&self) -> Result<LoadResult> {
        self.load_located().map(LocatedLoadResult::into_public)
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
            // Single-file mode: use appropriate path based on reset marker presence
            let config = Self::parse_config_with_reset_handling(&content, value)?;
            Ok(LoadResult {
                config,
                preset_used: None,
            })
        }
    }

    fn load_without_extends(&self) -> Result<LoadResult> {
        self.load_without_extends_located()
            .map(LocatedLoadResult::into_public)
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

        // Single-file mode: use appropriate path based on reset marker presence
        let config = Self::parse_config_with_reset_handling(&content, value)?;
        Ok(LoadResult {
            config,
            preset_used: None,
        })
    }

    fn load_with_sources(&self) -> Result<LoadResultWithSources> {
        let origin = self.discover_origin()?;
        if let Some(path) = origin.path() {
            return self.load_from_path_with_sources(path);
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
            // Single-file mode: use appropriate path based on reset marker presence
            let config = Self::parse_config_with_reset_handling(&content, value.clone())?;
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
        let origin = self.discover_origin()?;
        if let Some(path) = origin.path() {
            return self.load_from_path_without_extends_with_sources(path);
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

        // Single-file mode: use appropriate path based on reset marker presence
        let config = Self::parse_config_with_reset_handling(&content, value.clone())?;

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
