use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::config::FileSystem;

pub struct MockFileSystem {
    files: Mutex<HashMap<PathBuf, String>>,
    directories: Mutex<HashSet<PathBuf>>,
    canonical_paths: Mutex<HashMap<PathBuf, PathBuf>>,
    current_dir: PathBuf,
    current_dir_error: bool,
    config_dir: Option<PathBuf>,
}

impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
            directories: Mutex::new(HashSet::new()),
            canonical_paths: Mutex::new(HashMap::new()),
            current_dir: PathBuf::from("/project"),
            current_dir_error: false,
            config_dir: Some(PathBuf::from("/home/user/.config/sloc-guard")),
        }
    }

    pub fn with_file(self, path: impl Into<PathBuf>, content: &str) -> Self {
        let path = self.resolve_path(&path.into());
        self.files.lock().unwrap().insert(path, content.to_string());
        self
    }

    pub fn with_dir(self, path: impl Into<PathBuf>) -> Self {
        let path = self.resolve_path(&path.into());
        self.directories.lock().unwrap().insert(path);
        self
    }

    /// Configure a path to canonicalize to a different location, modelling a symlink while keeping
    /// reads anchored at the link path.
    pub fn with_canonical_path(
        self,
        path: impl Into<PathBuf>,
        canonical: impl Into<PathBuf>,
    ) -> Self {
        let path = self.resolve_path(&path.into());
        let canonical = self.resolve_path(&canonical.into());
        self.canonical_paths.lock().unwrap().insert(path, canonical);
        self
    }

    pub fn with_current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = path.into();
        self
    }

    pub fn with_current_dir_error(mut self) -> Self {
        self.current_dir_error = true;
        self
    }

    pub fn with_config_dir(mut self, path: Option<PathBuf>) -> Self {
        self.config_dir = path;
        self
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            normalize_path(path)
        } else {
            normalize_path(&self.current_dir.join(path))
        }
    }
}

impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let normalized = self.resolve_path(path);
        self.files
            .lock()
            .unwrap()
            .get(&normalized)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "file not found"))
    }

    fn exists(&self, path: &Path) -> bool {
        let normalized = self.resolve_path(path);
        self.files.lock().unwrap().contains_key(&normalized)
            || self.directories.lock().unwrap().contains(&normalized)
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        if self.current_dir_error {
            Err(Error::new(
                ErrorKind::NotFound,
                "current directory is unavailable",
            ))
        } else {
            Ok(self.current_dir.clone())
        }
    }

    fn config_dir(&self) -> Option<PathBuf> {
        self.config_dir.clone()
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        let normalized = self.resolve_path(path);
        let canonical = self
            .canonical_paths
            .lock()
            .unwrap()
            .get(&normalized)
            .cloned();
        if let Some(canonical) = canonical {
            return Ok(canonical);
        }
        // In mock, return error if file doesn't exist (like real canonicalize)
        if self.files.lock().unwrap().contains_key(&normalized)
            || self.directories.lock().unwrap().contains(&normalized)
        {
            Ok(normalized)
        } else {
            Err(Error::new(ErrorKind::NotFound, "file not found"))
        }
    }
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy().replace('\\', "/");
    let mut components = Vec::new();
    for part in path_str.split('/') {
        match part {
            ".." => {
                components.pop();
            }
            "." | "" => {}
            _ => components.push(part),
        }
    }
    let normalized = if path_str.starts_with('/') {
        format!("/{}", components.join("/"))
    } else {
        components.join("/")
    };
    PathBuf::from(normalized)
}
