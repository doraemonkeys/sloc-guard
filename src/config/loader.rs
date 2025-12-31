use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Result, SlocGuardError};

use super::Config;
use super::model::CONFIG_VERSION;
use super::presets;
use super::remote::{fetch_remote_config, fetch_remote_config_offline, is_remote_url};

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
const USER_CONFIG_DIR: &str = "sloc-guard";
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

    /// Get the user's home directory.
    fn home_dir(&self) -> Option<PathBuf>;
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

    fn home_dir(&self) -> Option<PathBuf> {
        dirs_home()
    }
}

fn dirs_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

/// Loads configuration from the filesystem.
///
/// Search order:
/// 1. `.sloc-guard.toml` in current directory
/// 2. `$HOME/.config/sloc-guard/config.toml`
/// 3. Returns `Config::default()` if no config found
#[derive(Debug)]
pub struct FileConfigLoader<F: FileSystem = RealFileSystem> {
    fs: F,
    offline: bool,
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
            offline: false,
            project_root: None,
        }
    }

    /// Create a loader with offline mode and project root options.
    #[must_use]
    pub const fn with_options(offline: bool, project_root: Option<PathBuf>) -> Self {
        Self {
            fs: RealFileSystem,
            offline,
            project_root,
        }
    }
}

impl<F: FileSystem> FileConfigLoader<F> {
    #[must_use]
    pub const fn with_fs(fs: F) -> Self {
        Self {
            fs,
            offline: false,
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
        self.fs.home_dir().map(|home| {
            home.join(".config")
                .join(USER_CONFIG_DIR)
                .join(USER_CONFIG_NAME)
        })
    }

    fn parse_config(content: &str) -> Result<Config> {
        let config: Config = toml::from_str(content).map_err(SlocGuardError::from)?;
        validate_config_version(&config)?;
        Ok(config)
    }

    /// Load config with extends chain, returning (value, `preset_used`).
    fn load_with_extends(
        &self,
        path: &Path,
        visited: &mut HashSet<String>,
    ) -> Result<(toml::Value, Option<String>)> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let key = canonical.to_string_lossy().to_string();

        if !visited.insert(key) {
            return Err(SlocGuardError::Config(format!(
                "Circular extends detected: {}",
                canonical.display()
            )));
        }

        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;

        self.process_config_content(&content, Some(path), visited)
    }

    /// Load remote config with extends chain, returning (value, `preset_used`).
    fn load_remote_with_extends(
        &self,
        url: &str,
        expected_hash: Option<&str>,
        visited: &mut HashSet<String>,
    ) -> Result<(toml::Value, Option<String>)> {
        if !visited.insert(url.to_string()) {
            return Err(SlocGuardError::Config(format!(
                "Circular extends detected: {url}"
            )));
        }

        let content = if self.offline {
            fetch_remote_config_offline(url, self.project_root.as_deref(), expected_hash)?
        } else {
            fetch_remote_config(url, self.project_root.as_deref(), expected_hash)?
        };
        self.process_config_content(&content, None, visited)
    }

    /// Process config content and return (value, `preset_used`).
    fn process_config_content(
        &self,
        content: &str,
        base_path: Option<&Path>,
        visited: &mut HashSet<String>,
    ) -> Result<(toml::Value, Option<String>)> {
        let mut config_value: toml::Value =
            toml::from_str(content).map_err(SlocGuardError::from)?;

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
                    preset_used = Some(preset_name.to_string());
                    (presets::load_preset(preset_name)?, None)
                } else if is_remote_url(&extends) {
                    self.load_remote_with_extends(&extends, extends_sha256.as_deref(), visited)?
                } else {
                    let extends_path = Path::new(&extends);
                    let resolved_path = if extends_path.is_absolute() {
                        extends_path.to_path_buf()
                    } else if let Some(base) = base_path {
                        base.parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(extends_path)
                    } else {
                        return Err(SlocGuardError::Config(format!(
                            "Cannot resolve relative path '{extends}' from remote config"
                        )));
                    };
                    self.load_with_extends(&resolved_path, visited)?
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

        Ok((config_value, preset_used))
    }
}

/// Merge two TOML values. Child values take precedence.
/// Tables are merged recursively. Arrays are replaced, not appended.
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
        (_, child) => child,
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
        let mut visited = HashSet::new();
        let (merged_value, preset_used) = self.load_with_extends(path, &mut visited)?;
        let config_str =
            toml::to_string(&merged_value).map_err(|e| SlocGuardError::Config(e.to_string()))?;
        let config = Self::parse_config(&config_str)?;
        Ok(LoadResult {
            config,
            preset_used,
        })
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
        let config = Self::parse_config(&content)?;
        Ok(LoadResult {
            config,
            preset_used: None,
        })
    }
}

#[cfg(test)]
#[path = "loader_tests/mod.rs"]
mod tests;
