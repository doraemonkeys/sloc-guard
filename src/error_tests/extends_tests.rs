use crate::error::SlocGuardError;

// =============================================================================
// CircularExtends Tests
// =============================================================================

#[test]
fn circular_extends_display() {
    let err = SlocGuardError::CircularExtends {
        chain: vec![
            "a.toml".to_string(),
            "b.toml".to_string(),
            "a.toml".to_string(),
        ],
    };
    let msg = err.to_string();
    assert!(msg.contains("Circular extends"));
    assert!(msg.contains("a.toml"));
    assert!(msg.contains("b.toml"));
}

#[test]
fn circular_extends_error_type() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string()],
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn circular_extends_message() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string(), "b.toml".to_string()],
    };
    let msg = err.message();
    assert!(msg.contains("circular extends"));
    assert!(msg.contains("a.toml → b.toml"));
}

#[test]
fn circular_extends_detail() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string(), "b.toml".to_string()],
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("chain:"));
}

#[test]
fn circular_extends_suggestion() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string()],
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("circular reference"));
}

// =============================================================================
// ExtendsTooDeep Tests
// =============================================================================

#[test]
fn extends_too_deep_display() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec!["config_0.toml".to_string(), "config_1.toml".to_string()],
    };
    let msg = err.to_string();
    assert!(msg.contains("too deep"));
    assert!(msg.contains("11"));
    assert!(msg.contains("10"));
}

#[test]
fn extends_too_deep_error_type() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec![],
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn extends_too_deep_message() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 15,
        max: 10,
        chain: vec![],
    };
    let msg = err.message();
    assert!(msg.contains("15"));
    assert!(msg.contains("10"));
}

#[test]
fn extends_too_deep_detail() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec!["a.toml".to_string(), "b.toml".to_string()],
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("chain:"));
    assert!(detail.contains("a.toml"));
}

#[test]
fn extends_too_deep_suggestion() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec![],
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("flatten") || suggestion.contains("presets"));
}

// =============================================================================
// ExtendsResolution Tests
// =============================================================================

#[test]
fn extends_resolution_display() {
    let err = SlocGuardError::ExtendsResolution {
        path: "../base.toml".to_string(),
        base: "remote config".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("../base.toml"));
    assert!(msg.contains("remote config"));
}

#[test]
fn extends_resolution_error_type() {
    let err = SlocGuardError::ExtendsResolution {
        path: "relative.toml".to_string(),
        base: "https://example.com".to_string(),
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn extends_resolution_message() {
    let err = SlocGuardError::ExtendsResolution {
        path: "../base.toml".to_string(),
        base: "remote config".to_string(),
    };
    let msg = err.message();
    assert!(msg.contains("../base.toml"));
    assert!(msg.contains("remote config"));
}

#[test]
fn extends_resolution_suggestion() {
    let err = SlocGuardError::ExtendsResolution {
        path: "../base.toml".to_string(),
        base: "remote config".to_string(),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("absolute path") || suggestion.contains("relative path"));
}

// =============================================================================
// RemoteConfigHashMismatch Tests
// =============================================================================

#[test]
fn remote_config_hash_mismatch_detail() {
    let err = SlocGuardError::RemoteConfigHashMismatch {
        url: "https://example.com".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("abc123"));
    assert!(detail.contains("def456"));
}

#[test]
fn remote_config_hash_mismatch_suggestion() {
    let err = SlocGuardError::RemoteConfigHashMismatch {
        url: "https://example.com/config.toml".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("extends_sha256"));
}
