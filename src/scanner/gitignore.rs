use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use gix::bstr::{BStr, ByteSlice};
use gix::dir::walk::EmissionMode;

use super::{FileFilter, FileScanner};
use crate::error::{Result, SlocGuardError};

/// A file scanner that respects .gitignore patterns using gix.
///
/// This scanner discovers the git repository containing the scan root and uses
/// gix's dirwalk to iterate over files while automatically excluding paths
/// matched by .gitignore patterns.
///
/// If the scan path is not within a git repository, it returns an error.
pub struct GitAwareScanner<F: FileFilter> {
    filter: F,
}

impl<F: FileFilter> GitAwareScanner<F> {
    #[must_use]
    pub const fn new(filter: F) -> Self {
        Self { filter }
    }

    fn scan_with_gix(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let repo = gix::discover(root)
            .map_err(|e| SlocGuardError::Git(format!("Failed to discover git repository: {e}")))?;

        let workdir = repo
            .workdir()
            .ok_or_else(|| SlocGuardError::Git("Repository has no working directory".into()))?;

        // Compute relative path from workdir to root
        let root_abs = root.canonicalize().map_err(|e| {
            SlocGuardError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot canonicalize path {}: {e}", root.display()),
            ))
        })?;
        let workdir_abs = workdir.canonicalize().map_err(|e| {
            SlocGuardError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot canonicalize workdir {}: {e}", workdir.display()),
            ))
        })?;

        // Calculate the prefix for filtering entries within the scan root
        let prefix = if root_abs == workdir_abs {
            PathBuf::new()
        } else {
            root_abs
                .strip_prefix(&workdir_abs)
                .map_err(|_| {
                    SlocGuardError::Git(format!(
                        "Scan path {} is not within git workdir {}",
                        root_abs.display(),
                        workdir_abs.display()
                    ))
                })?
                .to_path_buf()
        };

        // Get index for tracked files
        let index = repo
            .index_or_empty()
            .map_err(|e| SlocGuardError::Git(format!("Failed to get git index: {e}")))?;

        // Walk the directory with gitignore support
        let should_interrupt = AtomicBool::new(false);
        let options = repo
            .dirwalk_options()
            .map_err(|e| SlocGuardError::Git(format!("Failed to create dirwalk options: {e}")))?
            .emit_tracked(true)
            .emit_untracked(EmissionMode::Matching);

        let mut delegate = Collector::new(&prefix, &self.filter);

        // Convert prefix to BStr for pattern matching
        let prefix_str = gix::bstr::BString::from(prefix.to_string_lossy().as_bytes());
        let patterns: &[&BStr] = if prefix_str.is_empty() {
            &[]
        } else {
            &[prefix_str.as_bstr()]
        };

        repo.dirwalk(&index, patterns, &should_interrupt, options, &mut delegate)
            .map_err(|e| SlocGuardError::Git(format!("Dirwalk failed: {e}")))?;

        Ok(delegate
            .files
            .into_iter()
            .map(|p| workdir_abs.join(p))
            .collect())
    }
}

impl<F: FileFilter> FileScanner for GitAwareScanner<F> {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        self.scan_with_gix(root)
    }
}

/// Delegate for collecting file paths during dirwalk.
struct Collector<'a, F: FileFilter> {
    prefix: &'a Path,
    filter: &'a F,
    files: Vec<PathBuf>,
}

impl<'a, F: FileFilter> Collector<'a, F> {
    const fn new(prefix: &'a Path, filter: &'a F) -> Self {
        Self {
            prefix,
            filter,
            files: Vec::new(),
        }
    }
}

impl<F: FileFilter> gix::dir::walk::Delegate for Collector<'_, F> {
    fn emit(
        &mut self,
        entry: gix::dir::EntryRef<'_>,
        _collapsed_directory_status: Option<gix::dir::entry::Status>,
    ) -> gix::dir::walk::Action {
        // Only process files (not directories)
        if entry.disk_kind == Some(gix::dir::entry::Kind::File) {
            let path = entry.rela_path.to_path_lossy();

            // Check if within our scan prefix
            let in_prefix = self.prefix.as_os_str().is_empty() || path.starts_with(self.prefix);

            if in_prefix && self.filter.should_include(path.as_ref()) {
                self.files.push(path.into_owned());
            }
        }
        gix::dir::walk::Action::Continue
    }
}

#[cfg(test)]
#[path = "gitignore_tests.rs"]
mod tests;
