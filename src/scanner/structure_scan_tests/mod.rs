use std::path::Path;

use super::*;
use crate::project::ProjectPaths;

mod allow_mode_tests;
mod basic_tests;
mod dot_prefix_scope_tests;
mod exclude_pruning_tests;

/// Test filter that accepts all files.
pub struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
}

/// Accept-all filter with an explicit logical root for path-sensitive scanner tests.
pub struct RootedAcceptAllFilter(ProjectPaths);

impl RootedAcceptAllFilter {
    pub fn new(root: &Path) -> Self {
        Self(ProjectPaths::rooted_with_cwd(
            root.to_path_buf(),
            root.to_path_buf(),
        ))
    }
}

impl FileFilter for RootedAcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }

    fn project_paths(&self) -> &ProjectPaths {
        &self.0
    }
}

/// Test filter that only accepts `.rs` files.
pub struct RustOnlyFilter;

impl FileFilter for RustOnlyFilter {
    fn should_include(&self, path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "rs")
    }
}
