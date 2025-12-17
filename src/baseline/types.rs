use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Result, SlocGuardError};

const BASELINE_VERSION: u32 = 1;

/// Entry for a single file in the baseline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineEntry {
    pub lines: usize,
    pub hash: String,
}

impl BaselineEntry {
    #[must_use]
    pub const fn new(lines: usize, hash: String) -> Self {
        Self { lines, hash }
    }
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
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).map_err(|e| SlocGuardError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        let reader = BufReader::new(file);
        let baseline: Self = serde_json::from_reader(reader)?;
        Ok(baseline)
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

    /// Add or update a file entry in the baseline.
    pub fn set(&mut self, path: &str, lines: usize, hash: String) {
        self.files
            .insert(path.to_string(), BaselineEntry::new(lines, hash));
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
