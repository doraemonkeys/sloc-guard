use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Result, SlocGuardError};

use super::Config;

/// Trait for loading configuration from various sources.
pub trait ConfigLoader {
    /// Load configuration from the default location.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    fn load(&self) -> Result<Config>;

    /// Load configuration from a specific path.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    fn load_from_path(&self, path: &Path) -> Result<Config>;
}

const LOCAL_CONFIG_NAME: &str = ".sloc-guard.toml";
const USER_CONFIG_DIR: &str = "sloc-guard";
const USER_CONFIG_NAME: &str = "config.toml";

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
        }
    }
}

impl<F: FileSystem> FileConfigLoader<F> {
    #[must_use]
    pub const fn with_fs(fs: F) -> Self {
        Self { fs }
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
        toml::from_str(content).map_err(SlocGuardError::from)
    }

    fn load_with_extends(
        &self,
        path: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<toml::Value> {
        let canonical = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());

        if !visited.insert(canonical.clone()) {
            return Err(SlocGuardError::Config(format!(
                "Circular extends detected: {}",
                canonical.display()
            )));
        }

        let content = self
            .fs
            .read_to_string(path)
            .map_err(|source| SlocGuardError::FileRead {
                path: path.to_path_buf(),
                source,
            })?;

        let mut config_value: toml::Value =
            toml::from_str(&content).map_err(SlocGuardError::from)?;

        let extends_path = config_value
            .get("extends")
            .and_then(toml::Value::as_str)
            .map(|s| {
                let extends_path = Path::new(s);
                if extends_path.is_absolute() {
                    extends_path.to_path_buf()
                } else {
                    path.parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(extends_path)
                }
            });

        if let Some(extends) = extends_path {
            let base_value = self.load_with_extends(&extends, visited)?;
            config_value = merge_toml_values(base_value, config_value);
        }

        if let Some(table) = config_value.as_table_mut() {
            table.remove("extends");
        }

        Ok(config_value)
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
    fn load(&self) -> Result<Config> {
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

        Ok(Config::default())
    }

    fn load_from_path(&self, path: &Path) -> Result<Config> {
        let mut visited = HashSet::new();
        let merged_value = self.load_with_extends(path, &mut visited)?;
        let config_str =
            toml::to_string(&merged_value).map_err(|e| SlocGuardError::Config(e.to_string()))?;
        Self::parse_config(&config_str)
    }
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;
