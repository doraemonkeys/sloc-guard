use std::path::PathBuf;

use crate::error::ConfigSource;

#[test]
fn file_display() {
    let source = ConfigSource::file(PathBuf::from("config.toml"));
    assert_eq!(source.to_string(), "config.toml");
}

#[test]
fn remote_display() {
    let source = ConfigSource::remote("https://example.com/config.toml");
    assert_eq!(source.to_string(), "https://example.com/config.toml");
}

#[test]
fn preset_display() {
    let source = ConfigSource::preset("rust-strict");
    assert_eq!(source.to_string(), "preset:rust-strict");
}

#[test]
fn constructors() {
    let file = ConfigSource::file("/path/to/config.toml");
    let expected_path: &std::path::Path = std::path::Path::new("/path/to/config.toml");
    assert!(matches!(&file, ConfigSource::File { path } if path == expected_path));

    let remote = ConfigSource::remote("https://example.com");
    assert!(matches!(&remote, ConfigSource::Remote { url } if url == "https://example.com"));

    let preset = ConfigSource::preset("node-strict");
    assert!(matches!(&preset, ConfigSource::Preset { name } if name == "node-strict"));
}
