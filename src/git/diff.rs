use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::{Result, SlocGuardError};

/// Returns the set of files changed compared to a git reference.
pub trait ChangedFiles {
    /// Get files changed between the given reference and HEAD.
    ///
    /// # Errors
    /// Returns an error if the reference cannot be parsed or the repository cannot be accessed.
    fn get_changed_files(&self, base_ref: &str) -> Result<HashSet<PathBuf>>;
}

/// Git diff implementation using gix.
pub struct GitDiff {
    repo_path: PathBuf,
    workdir: PathBuf,
}

impl GitDiff {
    /// Create a new `GitDiff` for the repository containing the given path.
    ///
    /// # Errors
    /// Returns an error if no git repository is found.
    pub fn discover(path: &Path) -> Result<Self> {
        let repo = gix::discover(path)
            .map_err(|e| SlocGuardError::Git(format!("Failed to discover git repository: {e}")))?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| SlocGuardError::Git("Repository has no working directory".into()))?
            .to_path_buf();
        Ok(Self {
            repo_path: repo.path().to_path_buf(),
            workdir,
        })
    }

    /// Get the working directory of the repository.
    #[must_use]
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    fn open_repo(&self) -> Result<gix::Repository> {
        gix::open(&self.repo_path)
            .map_err(|e| SlocGuardError::Git(format!("Failed to open git repository: {e}")))
    }

    fn collect_tree_paths(
        tree: &gix::Tree<'_>,
        prefix: &Path,
    ) -> Result<HashSet<(PathBuf, gix::ObjectId)>> {
        let mut paths = HashSet::new();
        Self::collect_tree_paths_recursive(tree, prefix, &mut paths)?;
        Ok(paths)
    }

    fn collect_tree_paths_recursive(
        tree: &gix::Tree<'_>,
        prefix: &Path,
        paths: &mut HashSet<(PathBuf, gix::ObjectId)>,
    ) -> Result<()> {
        for entry in tree.iter() {
            let entry = entry
                .map_err(|e| SlocGuardError::Git(format!("Failed to read tree entry: {e}")))?;
            let name = std::str::from_utf8(entry.filename())
                .map_err(|e| SlocGuardError::Git(format!("Invalid filename encoding: {e}")))?;
            let path = prefix.join(name);

            match entry.mode().kind() {
                gix::object::tree::EntryKind::Blob
                | gix::object::tree::EntryKind::BlobExecutable => {
                    paths.insert((path, entry.oid().into()));
                }
                gix::object::tree::EntryKind::Tree => {
                    let subtree = entry.object().map_err(|e| {
                        SlocGuardError::Git(format!("Failed to get subtree object: {e}"))
                    })?;
                    let subtree = subtree.into_tree();
                    Self::collect_tree_paths_recursive(&subtree, &path, paths)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl ChangedFiles for GitDiff {
    fn get_changed_files(&self, base_ref: &str) -> Result<HashSet<PathBuf>> {
        let repo = self.open_repo()?;

        // Parse the base reference
        let base_commit = repo
            .rev_parse_single(base_ref)
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to parse reference '{base_ref}': {e}"))
            })?
            .object()
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to get object for '{base_ref}': {e}"))
            })?
            .peel_to_commit()
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to peel to commit '{base_ref}': {e}"))
            })?;

        // Get HEAD commit
        let head_commit = repo
            .head_commit()
            .map_err(|e| SlocGuardError::Git(format!("Failed to get HEAD commit: {e}")))?;

        // Get trees
        let base_tree = base_commit
            .tree()
            .map_err(|e| SlocGuardError::Git(format!("Failed to get tree for base: {e}")))?;
        let head_tree = head_commit
            .tree()
            .map_err(|e| SlocGuardError::Git(format!("Failed to get tree for HEAD: {e}")))?;

        // Collect all file paths from both trees
        let base_paths = Self::collect_tree_paths(&base_tree, Path::new(""))?;
        let head_paths = Self::collect_tree_paths(&head_tree, Path::new(""))?;

        // Find changed files: files that exist in only one tree or have different OIDs
        let mut changed_files = HashSet::new();

        // Files in HEAD but not in base, or changed
        for (path, oid) in &head_paths {
            let is_changed = base_paths
                .iter()
                .find(|(p, _)| p == path)
                .is_none_or(|(_, base_oid)| base_oid != oid);
            if is_changed {
                changed_files.insert(self.workdir.join(path));
            }
        }

        // Files deleted (in base but not in HEAD) - include them as they might still exist locally
        for (path, _) in &base_paths {
            if !head_paths.iter().any(|(p, _)| p == path) {
                let full_path = self.workdir.join(path);
                if full_path.exists() {
                    changed_files.insert(full_path);
                }
            }
        }

        Ok(changed_files)
    }
}
