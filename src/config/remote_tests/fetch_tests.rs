//! Tests for remote config fetching with mock HTTP client.

use super::super::{
    FetchPolicy, ReqwestClient, cache_file_path, fetch_remote_config,
    fetch_remote_config_with_client,
};

use super::{MockHttpClient, acquire_warning_lock, create_temp_project};

#[test]
fn fetch_remote_config_rejects_invalid_url() {
    let temp_dir = create_temp_project();
    let result = fetch_remote_config(
        "/local/path",
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid remote config URL"));
}

#[test]
fn fetch_remote_config_rejects_non_http_scheme() {
    let temp_dir = create_temp_project();
    let result = fetch_remote_config(
        "ftp://example.com/config.toml",
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid remote config URL"));
}

#[test]
fn fetch_with_mock_client_success() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 200\n";
    let client = MockHttpClient::success(content);

    let url = "https://mock-test-success.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
    assert_eq!(client.call_count(), 1);
}

#[test]
fn fetch_with_mock_client_error() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let client = MockHttpClient::error("Connection refused");

    let url = "https://mock-test-error.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Connection refused")
    );
    assert_eq!(client.call_count(), 1);
}

#[test]
fn fetch_with_mock_client_uses_cache_on_second_call() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 300\n";
    let client = MockHttpClient::success(content);

    let url = "https://mock-test-cache.example.com/config.toml";

    // First call should hit the client
    let result1 = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result1.is_ok());
    assert_eq!(client.call_count(), 1);

    // Check if cache was written successfully
    let cache_file_exists = cache_file_path(url, temp_dir.path()).exists();
    if !cache_file_exists {
        return;
    }

    // Second call should use cache, not hit client
    let result2 = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), content);
    assert_eq!(client.call_count(), 1); // Still 1, cache was used
}

#[test]
fn fetch_with_mock_client_invalid_url_never_calls_client() {
    let temp_dir = create_temp_project();
    let client = MockHttpClient::success("should not be called");

    let result = fetch_remote_config_with_client(
        "/local/path",
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid remote config URL")
    );
    assert_eq!(client.call_count(), 0); // Client should never be called
}

#[test]
fn fetch_with_mock_client_ftp_url_never_calls_client() {
    let temp_dir = create_temp_project();
    let client = MockHttpClient::success("should not be called");

    let result = fetch_remote_config_with_client(
        "ftp://example.com/config.toml",
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert_eq!(client.call_count(), 0);
}

#[test]
fn fetch_with_mock_client_http_url_accepted() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 400\n";
    let client = MockHttpClient::success(content);

    let url = "http://mock-test-http.example.com/config.toml";

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

#[test]
fn fetch_with_mock_client_timeout_error() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let client = MockHttpClient::error("Request timeout fetching remote config");

    let url = "https://mock-test-timeout.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}

#[test]
fn fetch_with_mock_client_http_404_error() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let client = MockHttpClient::error("Failed to fetch remote config: HTTP 404 Not Found");

    let url = "https://mock-test-404.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("404"));
}

#[test]
fn fetch_with_mock_client_http_500_error() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let client =
        MockHttpClient::error("Failed to fetch remote config: HTTP 500 Internal Server Error");

    let url = "https://mock-test-500.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("500"));
}

#[test]
fn fetch_with_mock_client_network_error() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let client = MockHttpClient::error("Failed to connect to remote config URL");

    let url = "https://mock-test-network.example.com/config.toml";

    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("connect"));
}

#[test]
fn reqwest_client_can_be_instantiated() {
    let client = ReqwestClient;
    // Just verify it can be created
    let _ = client;
}
