use crate::error::SlocGuardError;

#[test]
fn display() {
    let glob_err = globset::Glob::new("[invalid").unwrap_err();
    let err = SlocGuardError::InvalidPattern {
        pattern: "[invalid".to_string(),
        source: glob_err,
    };
    assert!(err.to_string().contains("[invalid"));
}

#[test]
fn message() {
    let glob_err = globset::Glob::new("[invalid").unwrap_err();
    let err = SlocGuardError::InvalidPattern {
        pattern: "[invalid".to_string(),
        source: glob_err,
    };
    let message = err.message();
    assert!(message.contains("[invalid"));
    // Glob error message should be included
    assert!(message.len() > "[invalid".len());
}

#[test]
fn detail() {
    let glob_err = globset::Glob::new("[invalid").unwrap_err();
    let err = SlocGuardError::InvalidPattern {
        pattern: "[invalid".to_string(),
        source: glob_err,
    };
    let detail = err.detail().unwrap();
    // Should contain the glob error
    assert!(!detail.is_empty());
}

#[test]
fn suggestion() {
    let glob_err = globset::Glob::new("[invalid").unwrap_err();
    let err = SlocGuardError::InvalidPattern {
        pattern: "[invalid".to_string(),
        source: glob_err,
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("glob pattern syntax"));
}

