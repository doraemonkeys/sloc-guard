//! Tests for structure config and rules semantic validation.

use crate::config::{Config, StructureConfig, StructureRule};

use super::super::*;

// ============================================================================
// Global Structure Config Tests
// ============================================================================

#[test]
fn warn_threshold_too_high() {
    let config = Config {
        structure: StructureConfig {
            warn_threshold: Some(1.5),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_threshold"));
    assert!(err_msg.contains("0.0 and 1.0"));
}

#[test]
fn warn_threshold_negative() {
    let config = Config {
        structure: StructureConfig {
            warn_threshold: Some(-0.1),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_threshold"));
}

#[test]
fn warn_threshold_valid_boundaries() {
    let mut config = Config::default();

    config.structure.warn_threshold = Some(0.0);
    assert!(validate_config_semantics(&config).is_ok());

    config.structure.warn_threshold = Some(1.0);
    assert!(validate_config_semantics(&config).is_ok());

    config.structure.warn_threshold = Some(0.9);
    assert!(validate_config_semantics(&config).is_ok());
}

#[test]
fn warn_files_threshold_out_of_range() {
    let config = Config {
        structure: StructureConfig {
            warn_files_threshold: Some(2.0),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_files_threshold"));
}

#[test]
fn warn_dirs_threshold_out_of_range() {
    let config = Config {
        structure: StructureConfig {
            warn_dirs_threshold: Some(-0.5),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_dirs_threshold"));
}

#[test]
fn warn_files_at_negative() {
    let config = Config {
        structure: StructureConfig {
            warn_files_at: Some(-10),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_files_at"));
    assert!(err_msg.contains("non-negative"));
}

#[test]
fn warn_dirs_at_negative() {
    let config = Config {
        structure: StructureConfig {
            warn_dirs_at: Some(-5),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_dirs_at"));
    assert!(err_msg.contains("non-negative"));
}

#[test]
fn warn_files_at_greater_than_max_files() {
    let config = Config {
        structure: StructureConfig {
            max_files: Some(50),
            warn_files_at: Some(60),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_files_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn warn_files_at_equal_to_max_files() {
    let config = Config {
        structure: StructureConfig {
            max_files: Some(50),
            warn_files_at: Some(50),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_files_at"));
}

#[test]
fn warn_dirs_at_greater_than_max_dirs() {
    let config = Config {
        structure: StructureConfig {
            max_dirs: Some(10),
            warn_dirs_at: Some(15),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.warn_dirs_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn warn_at_valid_with_unlimited_max() {
    // When max_files is -1 (unlimited), warn_files_at can be any non-negative value
    let config = Config {
        structure: StructureConfig {
            max_files: Some(-1),
            warn_files_at: Some(100),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn warn_at_valid_less_than_max() {
    let config = Config {
        structure: StructureConfig {
            max_files: Some(50),
            warn_files_at: Some(45),
            max_dirs: Some(10),
            warn_dirs_at: Some(8),
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

// ============================================================================
// Structure Rules Tests
// ============================================================================

#[test]
fn rule_warn_threshold_out_of_range() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                warn_threshold: Some(1.5),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[0].warn_threshold"));
}

#[test]
fn rule_warn_files_threshold_out_of_range() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                warn_files_threshold: Some(2.0),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[0].warn_files_threshold"));
}

#[test]
fn rule_warn_dirs_threshold_out_of_range() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                warn_dirs_threshold: Some(-0.1),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[0].warn_dirs_threshold"));
}

#[test]
fn rule_warn_files_at_negative() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                warn_files_at: Some(-5),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[0].warn_files_at"));
    assert!(err_msg.contains("non-negative"));
}

#[test]
fn rule_warn_dirs_at_negative() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                warn_dirs_at: Some(-1),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[0].warn_dirs_at"));
}

#[test]
fn rule_warn_files_at_greater_than_max_files() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(30),
                warn_files_at: Some(40),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[0].warn_files_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn rule_warn_dirs_at_greater_than_max_dirs() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                max_dirs: Some(5),
                warn_dirs_at: Some(10),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[0].warn_dirs_at"));
}

#[test]
fn rule_valid_config() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![StructureRule {
                scope: "src/**".to_string(),
                max_files: Some(50),
                warn_files_at: Some(45),
                max_dirs: Some(10),
                warn_dirs_at: Some(8),
                warn_threshold: Some(0.9),
                warn_files_threshold: Some(0.85),
                warn_dirs_threshold: Some(0.8),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn second_rule_invalid() {
    let config = Config {
        structure: StructureConfig {
            rules: vec![
                StructureRule {
                    scope: "src/**".to_string(),
                    warn_threshold: Some(0.9),
                    ..Default::default()
                },
                StructureRule {
                    scope: "tests/**".to_string(),
                    warn_threshold: Some(1.5), // Invalid
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("structure.rules[1].warn_threshold"));
}
