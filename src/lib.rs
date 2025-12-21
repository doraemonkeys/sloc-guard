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
pub mod path_utils;
pub mod scanner;
pub mod state;
pub mod stats;

pub use error::{Result, SlocGuardError};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_THRESHOLD_EXCEEDED: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
