use std::path::Path;

use crate::error::Result;

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

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;
