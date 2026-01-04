use crate::error::SlocGuardError;

#[test]
fn git_display() {
    let err = SlocGuardError::Git("Failed to get git index".to_string());
    assert_eq!(err.to_string(), "Git error: Failed to get git index");
}

#[test]
fn git_repo_not_found_display() {
    let err = SlocGuardError::GitRepoNotFound("not a git repository".to_string());
    assert_eq!(
        err.to_string(),
        "Not a git repository: not a git repository"
    );
}

#[test]
fn git_error_type() {
    assert_eq!(SlocGuardError::Git("test".to_string()).error_type(), "Git");
}

#[test]
fn git_repo_not_found_error_type() {
    assert_eq!(
        SlocGuardError::GitRepoNotFound("test".to_string()).error_type(),
        "Git"
    );
}

#[test]
fn git_message() {
    let err = SlocGuardError::Git("git error".to_string());
    assert_eq!(err.message(), "git error");
}

#[test]
fn git_suggestion() {
    let err = SlocGuardError::Git("failed to read index".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("git is installed"));
}

#[test]
fn git_repo_not_found_suggestion() {
    let err = SlocGuardError::GitRepoNotFound("/some/path".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("git init"));
}

