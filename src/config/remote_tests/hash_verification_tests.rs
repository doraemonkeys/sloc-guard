//! Tests for content hash computation and verification.

use super::super::{
    FetchPolicy, compute_content_hash, fetch_remote_config_with_client, write_to_cache,
};
use crate::error::SlocGuardError;

use super::{MockHttpClient, acquire_warning_lock, create_temp_project};

// ============================================================================
// Hash computation tests
// ============================================================================

#[test]
fn compute_content_hash_produces_consistent_hash() {
    let content = "version = \"2\"\n\n[content]\nmax_lines = 100\n";
    let hash1 = compute_content_hash(content);
    let hash2 = compute_content_hash(content);
    assert_eq!(hash1, hash2);
}

#[test]
fn compute_content_hash_produces_64_char_hex() {
    let content = "version = \"2\"\n\n[content]\nmax_lines = 100\n";
    let hash = compute_content_hash(content);
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn compute_content_hash_different_for_different_content() {
    let hash1 = compute_content_hash("version = \"2\"\n\n[content]\nmax_lines = 100\n");
    let hash2 = compute_content_hash("version = \"2\"\n\n[content]\nmax_lines = 200\n");
    assert_ne!(hash1, hash2);
}

// ============================================================================
// Hash verification tests
// ============================================================================

#[test]
fn hash_verification_success_on_match() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 900\n";
    let expected_hash = compute_content_hash(content);
    let client = MockHttpClient::success(content);

    let url = "https://mock-test-hash-success.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        Some(&expected_hash),
        FetchPolicy::Normal,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
}

#[test]
fn hash_verification_fails_on_mismatch() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 1000\n";
    let wrong_hash = "0".repeat(64); // Intentionally wrong hash
    let client = MockHttpClient::success(content);

    let url = "https://mock-test-hash-fail.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        Some(&wrong_hash),
        FetchPolicy::Normal,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("hash mismatch"));
    assert!(err_msg.contains(url));

    // Verify error contains expected and actual hash
    if let SlocGuardError::RemoteConfigHashMismatch {
        expected, actual, ..
    } = err
    {
        assert_eq!(expected, wrong_hash);
        assert_eq!(actual, compute_content_hash(content));
    } else {
        panic!("Expected RemoteConfigHashMismatch error");
    }
}

#[test]
fn hash_verification_with_cached_content_success() {
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 1100\n";
    let expected_hash = compute_content_hash(content);

    let url = "https://mock-test-hash-cache-success.example.com/config.toml";

    // Populate cache first
    let write_result = write_to_cache(url, content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    // Create a client that should NOT be called (cache hit)
    let client = MockHttpClient::error("Should not be called");

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        Some(&expected_hash),
        FetchPolicy::Normal,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
    assert_eq!(client.call_count(), 0); // Cache was used
}

#[test]
fn hash_verification_with_cached_content_fails_on_mismatch() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let cached_content = "version = \"2\"\n\n[content]\nmax_lines = 1200\n";
    let fresh_content = "version = \"2\"\n\n[content]\nmax_lines = 1201\n";
    let expected_hash = compute_content_hash(fresh_content);

    let url = "https://mock-test-hash-cache-fail.example.com/config.toml";

    // Populate cache with different content
    let write_result = write_to_cache(url, cached_content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    // Mock client returns fresh content that matches expected hash
    let client = MockHttpClient::success(fresh_content);

    // Should skip stale cache and fetch fresh content
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        Some(&expected_hash),
        FetchPolicy::Normal,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), fresh_content);
    assert_eq!(client.call_count(), 1); // Fetched fresh content
}

#[test]
fn hash_mismatch_on_both_cache_and_remote_fails() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let cached_content = "version = \"2\"\n\n[content]\nmax_lines = 1200\n";
    let remote_content = "version = \"2\"\n\n[content]\nmax_lines = 1201\n";
    let wrong_hash = "f".repeat(64); // Doesn't match either

    let url = "https://mock-test-hash-both-fail.example.com/config.toml";

    // Populate cache with content that doesn't match hash
    let write_result = write_to_cache(url, cached_content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    // Mock client returns content that also doesn't match hash
    let client = MockHttpClient::success(remote_content);

    // Should skip cache, fetch remote, and fail on hash verification
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        Some(&wrong_hash),
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("hash mismatch"));
    assert_eq!(client.call_count(), 1); // Tried to fetch
}

#[test]
fn hash_verification_offline_mode_success() {
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 1300\n";
    let expected_hash = compute_content_hash(content);
    let client = MockHttpClient::error("Should not be called in offline mode");

    let url = "https://mock-test-hash-offline-success.example.com/config.toml";

    // Populate cache first
    let write_result = write_to_cache(url, content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        Some(&expected_hash),
        FetchPolicy::Offline,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
    assert_eq!(client.call_count(), 0);
}

#[test]
fn hash_verification_offline_mode_fails_on_mismatch() {
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 1400\n";
    let wrong_hash = "a".repeat(64);
    let client = MockHttpClient::error("Should not be called in offline mode");

    let url = "https://mock-test-hash-offline-fail.example.com/config.toml";

    // Populate cache first
    let write_result = write_to_cache(url, content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        Some(&wrong_hash),
        FetchPolicy::Offline,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("hash mismatch"));
    assert_eq!(client.call_count(), 0);
}

#[test]
fn hash_not_required_when_none() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 1500\n";
    let client = MockHttpClient::success(content);

    let url = "https://mock-test-no-hash.example.com/config.toml";

    // No hash provided - should succeed
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
}
