mod diff;

pub use diff::{ChangedFiles, GitDiff};

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
