use super::{DiffRange, parse_diff_range};

#[test]
fn parse_diff_range_single_ref() {
    let result = parse_diff_range("main").unwrap();
    assert_eq!(
        result,
        DiffRange {
            base: "main".to_string(),
            target: "HEAD".to_string(),
        }
    );
}

#[test]
fn parse_diff_range_explicit_range() {
    let result = parse_diff_range("main..feature").unwrap();
    assert_eq!(
        result,
        DiffRange {
            base: "main".to_string(),
            target: "feature".to_string(),
        }
    );
}

#[test]
fn parse_diff_range_origin_refs() {
    let result = parse_diff_range("origin/main..origin/feature").unwrap();
    assert_eq!(
        result,
        DiffRange {
            base: "origin/main".to_string(),
            target: "origin/feature".to_string(),
        }
    );
}

#[test]
fn parse_diff_range_tags() {
    let result = parse_diff_range("v1.0..v2.0").unwrap();
    assert_eq!(
        result,
        DiffRange {
            base: "v1.0".to_string(),
            target: "v2.0".to_string(),
        }
    );
}

#[test]
fn parse_diff_range_trailing_dots_defaults_to_head() {
    let result = parse_diff_range("main..").unwrap();
    assert_eq!(
        result,
        DiffRange {
            base: "main".to_string(),
            target: "HEAD".to_string(),
        }
    );
}

#[test]
fn parse_diff_range_no_base_returns_error() {
    let result = parse_diff_range("..feature");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("base reference"));
}

#[test]
fn parse_diff_range_empty_returns_error() {
    let result = parse_diff_range("");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("requires a git reference"));
}

#[test]
fn parse_diff_range_head_tilde() {
    let result = parse_diff_range("HEAD~3..HEAD").unwrap();
    assert_eq!(
        result,
        DiffRange {
            base: "HEAD~3".to_string(),
            target: "HEAD".to_string(),
        }
    );
}

#[test]
fn parse_diff_range_commit_hash() {
    let result = parse_diff_range("abc123..def456").unwrap();
    assert_eq!(
        result,
        DiffRange {
            base: "abc123".to_string(),
            target: "def456".to_string(),
        }
    );
}
