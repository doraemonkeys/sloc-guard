mod context;
mod diff;

pub use context::GitContext;
pub use diff::{ChangedFiles, GitDiff};

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
