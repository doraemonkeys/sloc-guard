use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::error::{Result, SlocGuardError};

pub trait FileFilter {
    fn should_include(&self, path: &Path) -> bool;
}

pub struct GlobFilter {
    extensions: Vec<String>,
    exclude_patterns: GlobSet,
}

impl GlobFilter {
    /// Create a new filter with the given extensions and exclude patterns.
    ///
    /// # Errors
    /// Returns an error if any exclude pattern is invalid.
    pub fn new(extensions: Vec<String>, exclude_patterns: &[String]) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        for pattern in exclude_patterns {
            let glob = Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: pattern.clone(),
                source: e,
            })?;
            builder.add(glob);
        }
        let exclude_patterns = builder
            .build()
            .map_err(|e| SlocGuardError::InvalidPattern {
                pattern: "combined patterns".to_string(),
                source: e,
            })?;

        Ok(Self {
            extensions,
            exclude_patterns,
        })
    }

    fn has_valid_extension(&self, path: &Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }

        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| self.extensions.iter().any(|e| e == ext))
    }

    fn is_excluded(&self, path: &Path) -> bool {
        self.exclude_patterns.is_match(path)
    }
}

impl FileFilter for GlobFilter {
    fn should_include(&self, path: &Path) -> bool {
        self.has_valid_extension(path) && !self.is_excluded(path)
    }
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;
