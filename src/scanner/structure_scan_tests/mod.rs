use std::path::Path;

use super::*;

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

/// Test filter that only accepts `.rs` files.
pub struct RustOnlyFilter;

impl FileFilter for RustOnlyFilter {
    fn should_include(&self, path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "rs")
    }
}
