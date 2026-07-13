//! Loader-level coverage that remote extends sources pass through per-source
//! schema enforcement with `ConfigSource::Remote` origin, complementing the
//! file and preset cases in `loader_tests::strict_schema_tests`.

use std::fs;

use crate::config::{ConfigLoader, FileConfigLoader};
use crate::error::{ConfigSource, SlocGuardError};

use super::super::{FetchPolicy, write_to_cache};

use super::create_temp_project;

#[test]
fn relative_extends_in_remote_source_names_the_url() {
    let temp_dir = create_temp_project();
    let root = temp_dir.path();
    let url = "https://mock-test-remote-relative-extends.example.com/base.toml";

    // A remote config has no directory to anchor a relative extends path.
    let remote_content = "extends = \"nested/base.toml\"\n";
    write_to_cache(url, remote_content, Some(root)).expect("cache priming should succeed");

    let child_path = root.join("child.toml");
    fs::write(&child_path, format!("extends = \"{url}\"\n")).unwrap();

    let loader = FileConfigLoader::with_options(FetchPolicy::Offline, Some(root.to_path_buf()));
    let err = loader.load_from_path(&child_path).unwrap_err();

    match err {
        SlocGuardError::ExtendsResolution { path, base } => {
            assert_eq!(path, "nested/base.toml");
            assert_eq!(base, url, "the unresolvable base should be the remote URL");
        }
        other => panic!("Expected ExtendsResolution error, got: {other:?}"),
    }
}

#[test]
fn unknown_field_in_remote_extends_source_reports_remote_origin() {
    let temp_dir = create_temp_project();
    let root = temp_dir.path();
    let url = "https://mock-test-remote-strict-schema.example.com/base.toml";

    // Prime the cache and load offline: no network involved, yet the loader
    // still routes the remote content through the extends resolver's fetch.
    let remote_content = "version = \"2\"\n\n[content]\nmax_linez = 100\n";
    write_to_cache(url, remote_content, Some(root)).expect("cache priming should succeed");

    let child_path = root.join("child.toml");
    fs::write(
        &child_path,
        format!("extends = \"{url}\"\n\n[content]\nmax_lines = 200\n"),
    )
    .unwrap();

    let loader = FileConfigLoader::with_options(FetchPolicy::Offline, Some(root.to_path_buf()));
    let err = loader.load_from_path(&child_path).unwrap_err();

    match err {
        SlocGuardError::Syntax {
            origin,
            line,
            message,
            ..
        } => {
            assert_eq!(origin, Some(ConfigSource::remote(url)));
            assert_eq!(line, 4, "expected the line within the remote source");
            assert!(message.contains("max_linez"), "got: {message}");
        }
        other => panic!("Expected Syntax error, got: {other:?}"),
    }
}
