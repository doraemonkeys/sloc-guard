//! Tests for the on-disk directory inventory used by structure explain.

use std::ffi::OsString;

use super::super::read_dir_inventory;

#[test]
fn read_dir_inventory_splits_and_sorts_children() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub_b")).unwrap();
    std::fs::create_dir(dir.path().join("sub_a")).unwrap();
    std::fs::write(dir.path().join("b.rs"), "").unwrap();
    std::fs::write(dir.path().join("a.rs"), "").unwrap();

    let stats = read_dir_inventory(dir.path(), 3).unwrap();

    assert_eq!(
        stats.files,
        vec![OsString::from("a.rs"), OsString::from("b.rs")]
    );
    assert_eq!(
        stats.dirs,
        vec![OsString::from("sub_a"), OsString::from("sub_b")]
    );
    assert_eq!(stats.depth, 3);
}

#[test]
fn read_dir_inventory_missing_directory_errors() {
    let dir = tempfile::tempdir().unwrap();

    let result = read_dir_inventory(&dir.path().join("missing"), 0);

    assert!(result.is_err());
}
