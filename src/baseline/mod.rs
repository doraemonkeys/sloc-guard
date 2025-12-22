use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Result, SlocGuardError};

const BASELINE_VERSION: u32 = 2;

/// Type of structure violation stored in baseline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructureViolationType {
    Files,
    Dirs,
}

/// Entry for a single path in the baseline (V2 format).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BaselineEntry {
    /// Content (SLOC) violation entry
    Content { lines: usize, hash: String },
    /// Structure (directory limit) violation entry
    Structure {
        violation_type: StructureViolationType,
        count: usize,
    },
}

impl BaselineEntry {
    #[must_use]
    pub const fn content(lines: usize, hash: String) -> Self {
        Self::Content { lines, hash }
    }

    #[must_use]
    pub const fn structure(violation_type: StructureViolationType, count: usize) -> Self {
        Self::Structure {
            violation_type,
            count,
        }
    }

    /// Returns true if this is a content entry.
    #[must_use]
    pub const fn is_content(&self) -> bool {
        matches!(self, Self::Content { .. })
    }

    /// Returns true if this is a structure entry.
    #[must_use]
    pub const fn is_structure(&self) -> bool {
        matches!(self, Self::Structure { .. })
    }
}

/// Legacy entry format (V1) for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LegacyBaselineEntry {
    lines: usize,
    hash: String,
}

/// Legacy baseline format (V1) for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyBaseline {
    version: u32,
    files: HashMap<String, LegacyBaselineEntry>,
}

/// Baseline file structure for tracking grandfathered violations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Baseline {
    version: u32,
    files: HashMap<String, BaselineEntry>,
}

impl Default for Baseline {
    fn default() -> Self {
        Self::new()
    }
}

impl Baseline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: BASELINE_VERSION,
            files: HashMap::new(),
        }
    }

    /// Load baseline from a JSON file.
    /// Supports both V1 (legacy) and V2 formats with automatic migration.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| SlocGuardError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Try V2 format first
        if let Ok(baseline) = serde_json::from_str::<Self>(&content) {
            return Ok(baseline);
        }

        // Fall back to V1 migration
        let legacy: LegacyBaseline = serde_json::from_str(&content)?;
        Ok(Self::migrate_from_v1(legacy))
    }

    /// Migrate from V1 baseline format to V2.
    fn migrate_from_v1(legacy: LegacyBaseline) -> Self {
        let files = legacy
            .files
            .into_iter()
            .map(|(path, entry)| (path, BaselineEntry::content(entry.lines, entry.hash)))
            .collect();

        Self {
            version: BASELINE_VERSION,
            files,
        }
    }

    /// Save baseline to a JSON file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path).map_err(|e| SlocGuardError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Add or update a content entry in the baseline.
    pub fn set_content(&mut self, path: &str, lines: usize, hash: String) {
        self.files
            .insert(path.to_string(), BaselineEntry::content(lines, hash));
    }

    /// Add or update a structure entry in the baseline.
    pub fn set_structure(
        &mut self,
        path: &str,
        violation_type: StructureViolationType,
        count: usize,
    ) {
        self.files.insert(
            path.to_string(),
            BaselineEntry::structure(violation_type, count),
        );
    }

    /// Add or update an entry in the baseline.
    pub fn set(&mut self, path: &str, entry: BaselineEntry) {
        self.files.insert(path.to_string(), entry);
    }

    /// Get a file entry from the baseline.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&BaselineEntry> {
        self.files.get(path)
    }

    /// Remove a file entry from the baseline.
    pub fn remove(&mut self, path: &str) -> Option<BaselineEntry> {
        self.files.remove(path)
    }

    /// Check if a file exists in the baseline.
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Get all file entries in the baseline.
    #[must_use]
    pub const fn files(&self) -> &HashMap<String, BaselineEntry> {
        &self.files
    }

    /// Get the number of files in the baseline.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the baseline is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get the version of the baseline format.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }
}

/// Compute SHA-256 hash of file content.
///
/// # Errors
/// Returns an error if the file cannot be read.
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| SlocGuardError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Read file content and compute SHA-256 hash in a single pass.
/// Returns (hash, content) to avoid reading the file twice.
///
/// # Errors
/// Returns an error if the file cannot be read.
pub fn read_file_with_hash(path: &Path) -> Result<(String, Vec<u8>)> {
    let content = fs::read(path).map_err(|e| SlocGuardError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;
    let hash = compute_hash_from_bytes(&content);
    Ok((hash, content))
}

/// Compute SHA-256 hash from bytes.
#[must_use]
pub fn compute_hash_from_bytes(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

/// Compute SHA-256 hash of a string.
#[must_use]
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "baseline_tests.rs"]
mod tests;
