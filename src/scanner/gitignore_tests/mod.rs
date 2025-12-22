//! Tests for `GitAwareScanner`.
//!
//! Organized into the following submodules:
//! - `scan_tests`: Basic `scan()` functionality tests
//! - `structure_tests`: `scan_with_structure()` functionality tests
//! - `deny_file_tests`: File denial pattern tests
//! - `deny_dir_tests`: Directory denial pattern tests

mod fixtures;

mod deny_dir_tests;
mod deny_file_tests;
mod scan_tests;
mod structure_tests;
