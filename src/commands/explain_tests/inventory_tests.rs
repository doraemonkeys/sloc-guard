//! Tests for the config-driven scan inventory used by structure explain.

use std::ffi::OsString;
use std::path::Path;

use tempfile::TempDir;

use crate::commands::context::CheckContext;
use crate::config::{Config, ScannerConfig, StructureConfig};
use crate::project::ProjectPaths;

use super::super::scan_dir_inventory;

fn config_with(exclude: &[&str], gitignore: bool) -> Config {
    Config {
        scanner: ScannerConfig {
            gitignore,
            exclude: exclude.iter().map(ToString::to_string).collect(),
        },
        structure: StructureConfig {
            max_files: Some(10),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn context_rooted_at(root: &Path, config: &Config) -> CheckContext {
    CheckContext::from_config_with_project_paths(
        config,
        config.content.warn_threshold,
        config.scanner.exclude.clone(),
        config.scanner.gitignore,
        ProjectPaths::rooted_with_cwd(root.to_path_buf(), root.to_path_buf()),
    )
    .unwrap()
}

#[test]
fn scan_inventory_drops_scanner_excluded_children() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("pkg")).unwrap();
    std::fs::write(dir.path().join("pkg").join("kept.rs"), "").unwrap();
    std::fs::write(dir.path().join("pkg").join("dropped.gen"), "").unwrap();
    let config = config_with(&["**/*.gen"], false);
    let ctx = context_rooted_at(dir.path(), &config);

    let stats = scan_dir_inventory(&ctx, &dir.path().join("pkg"), Path::new("pkg"))
        .unwrap()
        .expect("pkg survives the scan");

    assert_eq!(stats.files, vec![OsString::from("kept.rs")]);
    assert_eq!(stats.depth, 1);
}

#[test]
fn scan_inventory_drops_gitignored_children() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
    std::fs::create_dir(dir.path().join("pkg")).unwrap();
    std::fs::write(dir.path().join("pkg").join("kept.rs"), "").unwrap();
    std::fs::write(dir.path().join("pkg").join("ignored.log"), "").unwrap();
    let config = config_with(&[], true);
    let ctx = context_rooted_at(dir.path(), &config);

    let stats = scan_dir_inventory(&ctx, &dir.path().join("pkg"), Path::new("pkg"))
        .unwrap()
        .expect("pkg survives the scan");

    assert_eq!(stats.files, vec![OsString::from("kept.rs")]);
}

#[test]
fn scan_inventory_reports_pruned_directory_as_absent() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("vendor")).unwrap();
    std::fs::write(dir.path().join("vendor").join("lib.rs"), "").unwrap();
    let config = config_with(&["vendor/**"], false);
    let ctx = context_rooted_at(dir.path(), &config);

    let stats = scan_dir_inventory(&ctx, &dir.path().join("vendor"), Path::new("vendor")).unwrap();

    assert_eq!(stats, None);
}

#[test]
fn scan_inventory_outside_config_root_scans_directory_as_own_root() {
    let workspace = TempDir::new().unwrap();
    let project = workspace.path().join("project");
    let outside = workspace.path().join("outside");
    std::fs::create_dir(&project).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("file.rs"), "").unwrap();
    let config = config_with(&[], false);
    let ctx = context_rooted_at(&project, &config);
    let logical = ctx.project_paths().logical(&outside);

    let stats = scan_dir_inventory(&ctx, &outside, &logical)
        .unwrap()
        .expect("outside directory is scanned as its own root");

    assert_eq!(stats.files, vec![OsString::from("file.rs")]);
}
