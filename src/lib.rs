pub mod analyzer;
pub mod baseline;
pub mod cache;
pub mod checker;
pub mod cli;
pub mod commands;
pub mod config;
pub mod counter;
pub mod error;
pub mod git;
pub mod language;
pub mod output;
pub mod project;
pub mod scanner;
pub mod state;
pub mod stats;

pub use error::{Result, SlocGuardError};

pub const REPO_URL: &str = "https://github.com/doraemonkeys/sloc-guard";

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_THRESHOLD_EXCEEDED: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;

#[cfg(test)]
static TEST_CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn lock_test_cwd() -> std::sync::MutexGuard<'static, ()> {
    TEST_CWD_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
