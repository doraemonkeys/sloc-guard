use std::path::PathBuf;

use super::*;

#[test]
fn error_display_config() {
    let err = SlocGuardError::Config("invalid threshold".to_string());
    assert_eq!(err.to_string(), "Configuration error: invalid threshold");
}

#[test]
fn error_display_file_read() {
    let err = SlocGuardError::FileRead {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    assert!(err.to_string().contains("test.rs"));
}
