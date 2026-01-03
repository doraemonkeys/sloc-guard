//! Tests for URL detection and hashing functions.

use super::super::{hash_url, is_remote_url};

#[test]
fn is_remote_url_detects_https() {
    assert!(is_remote_url("https://example.com/config.toml"));
    assert!(is_remote_url(
        "https://github.com/user/repo/raw/main/.sloc-guard.toml"
    ));
}

#[test]
fn is_remote_url_detects_http() {
    assert!(is_remote_url("http://example.com/config.toml"));
    assert!(is_remote_url("http://localhost:8080/config.toml"));
}

#[test]
fn is_remote_url_rejects_local_paths() {
    assert!(!is_remote_url("/etc/config.toml"));
    assert!(!is_remote_url("./config.toml"));
    assert!(!is_remote_url("../config.toml"));
    assert!(!is_remote_url("config.toml"));
    assert!(!is_remote_url("C:\\config.toml"));
}

#[test]
fn is_remote_url_rejects_other_schemes() {
    assert!(!is_remote_url("ftp://example.com/config.toml"));
    assert!(!is_remote_url("file:///etc/config.toml"));
    assert!(!is_remote_url("ssh://user@host/config.toml"));
}

#[test]
fn is_remote_url_rejects_empty_string() {
    assert!(!is_remote_url(""));
}

#[test]
fn is_remote_url_rejects_partial_schemes() {
    assert!(!is_remote_url("http:/example.com")); // Missing slash
    assert!(!is_remote_url("https:example.com")); // Missing slashes
    assert!(!is_remote_url("ttp://example.com")); // Typo
}

#[test]
fn hash_url_produces_consistent_hash() {
    let url = "https://example.com/config.toml";
    let hash1 = hash_url(url);
    let hash2 = hash_url(url);
    assert_eq!(hash1, hash2);
}

#[test]
fn hash_url_produces_different_hashes_for_different_urls() {
    let hash1 = hash_url("https://example.com/config1.toml");
    let hash2 = hash_url("https://example.com/config2.toml");
    assert_ne!(hash1, hash2);
}

#[test]
fn hash_url_produces_hex_string() {
    let hash = hash_url("https://example.com/config.toml");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
}

#[test]
fn hash_url_handles_special_characters() {
    let hash1 = hash_url("https://example.com/config.toml?foo=bar&baz=qux");
    let hash2 = hash_url("https://example.com/config.toml#section");
    assert_ne!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
    assert_eq!(hash2.len(), 64);
}
