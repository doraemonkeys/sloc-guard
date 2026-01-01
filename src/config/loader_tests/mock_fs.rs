use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::config::FileSystem;

pub struct MockFileSystem {
    files: Mutex<HashMap<PathBuf, String>>,
    current_dir: PathBuf,
    config_dir: Option<PathBuf>,
}

impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
            current_dir: PathBuf::from("/project"),
            config_dir: Some(PathBuf::from("/home/user/.config/sloc-guard")),
        }
    }

    pub fn with_file(self, path: impl Into<PathBuf>, content: &str) -> Self {
        self.files
            .lock()
            .unwrap()
            .insert(path.into(), content.to_string());
        self
    }

    pub fn with_current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = path.into();
        self
    }

    pub fn with_config_dir(mut self, path: Option<PathBuf>) -> Self {
        self.config_dir = path;
        self
    }
}

impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let normalized = normalize_path(path);
        self.files
            .lock()
            .unwrap()
            .get(&normalized)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "file not found"))
    }

    fn exists(&self, path: &Path) -> bool {
        let normalized = normalize_path(path);
        self.files.lock().unwrap().contains_key(&normalized)
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        Ok(self.current_dir.clone())
    }

    fn config_dir(&self) -> Option<PathBuf> {
        self.config_dir.clone()
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        let normalized = normalize_path(path);
        // In mock, return error if file doesn't exist (like real canonicalize)
        if self.files.lock().unwrap().contains_key(&normalized) {
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
