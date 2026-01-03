//! Tests for the expires module.

use super::*;
use crate::config::{Config, ContentRule, StructureRule};

#[test]
fn test_parse_valid_date() {
    let date = ParsedDate::parse("2025-12-31").unwrap();
    assert_eq!(date.year, 2025);
    assert_eq!(date.month, 12);
    assert_eq!(date.day, 31);
}

#[test]
fn test_parse_invalid_format() {
    assert!(ParsedDate::parse("2025/12/31").is_err());
    assert!(ParsedDate::parse("2025-12").is_err());
    assert!(ParsedDate::parse("not-a-date").is_err());
}

#[test]
fn test_parse_invalid_month() {
    assert!(ParsedDate::parse("2025-13-01").is_err());
    assert!(ParsedDate::parse("2025-00-01").is_err());
}

#[test]
fn test_parse_invalid_day() {
    assert!(ParsedDate::parse("2025-01-32").is_err());
    assert!(ParsedDate::parse("2025-01-00").is_err());
}

#[test]
fn test_date_comparison() {
    let earlier = ParsedDate::parse("2024-01-01").unwrap();
    let later = ParsedDate::parse("2025-12-31").unwrap();
    assert!(earlier < later);
}

#[test]
fn test_is_expired_at_past_date() {
    let today = ParsedDate::parse("2025-06-15").unwrap();
    assert!(is_expired_at("2025-01-01", today).unwrap());
}

#[test]
fn test_is_expired_at_future_date() {
    let today = ParsedDate::parse("2025-06-15").unwrap();
    assert!(!is_expired_at("2025-12-31", today).unwrap());
}

#[test]
fn test_is_expired_at_same_date() {
    let today = ParsedDate::parse("2025-06-15").unwrap();
    assert!(!is_expired_at("2025-06-15", today).unwrap());
}

#[test]
fn test_today_returns_valid_date() {
    let today = ParsedDate::today();
    assert!(today.year >= 2024);
    assert!((1..=12).contains(&today.month));
    assert!((1..=31).contains(&today.day));
}

#[test]
fn test_collect_expired_rules_with_date() {
    let mut config = Config::default();
    config.content.rules = vec![
        ContentRule {
            pattern: "src/old/**".to_string(),
            max_lines: 500,
            expires: Some("2025-01-01".to_string()),
            reason: Some("Legacy code".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/new/**".to_string(),
            max_lines: 500,
            expires: Some("2025-12-31".to_string()),
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
    ];
    config.structure.rules = vec![StructureRule {
        scope: "tests/".to_string(),
        expires: Some("2024-06-01".to_string()),
        ..Default::default()
    }];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    assert_eq!(expired.len(), 2);
    assert_eq!(expired[0].rule_type, ExpiredRuleType::Content);
    assert_eq!(expired[0].pattern, "src/old/**");
    assert_eq!(expired[0].reason, Some("Legacy code".to_string()));
    assert_eq!(expired[1].rule_type, ExpiredRuleType::Structure);
    assert_eq!(expired[1].pattern, "tests/");
}

// =========================================================================
// Expires Combination Tests
// =========================================================================
// These tests verify complex scenarios involving multiple rules with
// different expiration states, ensuring the expiration logic correctly
// identifies all expired rules regardless of rule ordering or mixing.

#[test]
fn test_multiple_content_rules_mixed_expiration() {
    let mut config = Config::default();
    config.content.rules = vec![
        ContentRule {
            pattern: "src/legacy/**".to_string(),
            max_lines: 1000,
            expires: Some("2024-01-01".to_string()), // Expired
            reason: Some("Legacy exemption".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/generated/**".to_string(),
            max_lines: 2000,
            expires: Some("2026-12-31".to_string()), // Not expired
            reason: Some("Generated code".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/vendor/**".to_string(),
            max_lines: 1500,
            expires: Some("2025-03-01".to_string()), // Expired
            reason: Some("Vendor code".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "tests/**".to_string(),
            max_lines: 800,
            expires: None, // No expiration
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
    ];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    // Should find 2 expired rules: legacy (index 0) and vendor (index 2)
    assert_eq!(expired.len(), 2);

    assert_eq!(expired[0].index, 0);
    assert_eq!(expired[0].pattern, "src/legacy/**");
    assert_eq!(expired[0].expires, "2024-01-01");

    assert_eq!(expired[1].index, 2);
    assert_eq!(expired[1].pattern, "src/vendor/**");
    assert_eq!(expired[1].expires, "2025-03-01");
}

#[test]
fn test_multiple_structure_rules_mixed_expiration() {
    let mut config = Config::default();
    config.structure.rules = vec![
        StructureRule {
            scope: "src/modules/**".to_string(),
            max_files: Some(100),
            expires: Some("2024-12-01".to_string()), // Expired
            reason: Some("Module growth exemption".to_string()),
            ..Default::default()
        },
        StructureRule {
            scope: "docs/**".to_string(),
            max_files: Some(50),
            expires: Some("2026-01-01".to_string()), // Not expired
            reason: None,
            ..Default::default()
        },
        StructureRule {
            scope: "scripts/**".to_string(),
            max_files: Some(30),
            expires: None, // No expiration
            reason: None,
            ..Default::default()
        },
    ];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].rule_type, ExpiredRuleType::Structure);
    assert_eq!(expired[0].index, 0);
    assert_eq!(expired[0].pattern, "src/modules/**");
}

#[test]
fn test_mixed_content_and_structure_expired_rules() {
    let mut config = Config::default();
    config.content.rules = vec![
        ContentRule {
            pattern: "src/legacy/**".to_string(),
            max_lines: 1000,
            expires: Some("2025-01-01".to_string()), // Expired
            reason: Some("Legacy content".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/new/**".to_string(),
            max_lines: 500,
            expires: Some("2026-01-01".to_string()), // Not expired
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
    ];
    config.structure.rules = vec![
        StructureRule {
            scope: "tests/old/**".to_string(),
            max_files: Some(200),
            expires: Some("2024-06-01".to_string()), // Expired
            reason: Some("Test reorganization".to_string()),
            ..Default::default()
        },
        StructureRule {
            scope: "tests/new/**".to_string(),
            max_files: Some(50),
            expires: Some("2027-01-01".to_string()), // Not expired
            reason: None,
            ..Default::default()
        },
    ];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    // Should find 2 expired: 1 content, 1 structure
    assert_eq!(expired.len(), 2);

    // First should be content rule (content rules are checked first)
    assert_eq!(expired[0].rule_type, ExpiredRuleType::Content);
    assert_eq!(expired[0].pattern, "src/legacy/**");
    assert_eq!(expired[0].reason, Some("Legacy content".to_string()));

    // Second should be structure rule
    assert_eq!(expired[1].rule_type, ExpiredRuleType::Structure);
    assert_eq!(expired[1].pattern, "tests/old/**");
    assert_eq!(expired[1].reason, Some("Test reorganization".to_string()));
}

#[test]
fn test_same_pattern_different_expires() {
    // Tests the edge case where multiple rules target the same pattern
    // but have different expiration dates (e.g., staggered exemptions)
    let mut config = Config::default();
    config.content.rules = vec![
        ContentRule {
            pattern: "src/special/**".to_string(),
            max_lines: 800,
            expires: Some("2024-01-01".to_string()), // Expired (first exemption)
            reason: Some("Phase 1 exemption".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/special/**".to_string(),
            max_lines: 1000,
            expires: Some("2025-01-01".to_string()), // Expired (second exemption)
            reason: Some("Phase 2 exemption".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/special/**".to_string(),
            max_lines: 1200,
            expires: Some("2026-01-01".to_string()), // Not expired (current exemption)
            reason: Some("Phase 3 exemption".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
    ];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    // First two rules are expired
    assert_eq!(expired.len(), 2);
    assert_eq!(expired[0].index, 0);
    assert_eq!(expired[0].reason, Some("Phase 1 exemption".to_string()));
    assert_eq!(expired[1].index, 1);
    assert_eq!(expired[1].reason, Some("Phase 2 exemption".to_string()));
}

#[test]
fn test_all_rules_expired() {
    let mut config = Config::default();
    config.content.rules = vec![
        ContentRule {
            pattern: "src/**".to_string(),
            max_lines: 1000,
            expires: Some("2024-01-01".to_string()),
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "tests/**".to_string(),
            max_lines: 800,
            expires: Some("2024-06-01".to_string()),
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
    ];
    config.structure.rules = vec![StructureRule {
        scope: "docs/**".to_string(),
        max_files: Some(50),
        expires: Some("2025-01-01".to_string()),
        ..Default::default()
    }];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    // All 3 rules should be expired
    assert_eq!(expired.len(), 3);
}

#[test]
fn test_no_rules_expired() {
    let mut config = Config::default();
    config.content.rules = vec![ContentRule {
        pattern: "src/**".to_string(),
        max_lines: 1000,
        expires: Some("2026-01-01".to_string()), // Future
        reason: None,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
    }];
    config.structure.rules = vec![StructureRule {
        scope: "tests/**".to_string(),
        max_files: Some(50),
        expires: Some("2030-12-31".to_string()), // Far future
        ..Default::default()
    }];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    assert!(expired.is_empty());
}

#[test]
fn test_expires_on_boundary_date() {
    // Rule expires exactly on today's date should NOT be considered expired
    // (expires means "valid until this date", so it's still valid on the date)
    let mut config = Config::default();
    config.content.rules = vec![
        ContentRule {
            pattern: "src/today/**".to_string(),
            max_lines: 1000,
            expires: Some("2025-06-15".to_string()), // Same as today
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/yesterday/**".to_string(),
            max_lines: 1000,
            expires: Some("2025-06-14".to_string()), // Yesterday - expired
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "src/tomorrow/**".to_string(),
            max_lines: 1000,
            expires: Some("2025-06-16".to_string()), // Tomorrow - not expired
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
    ];

    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    // Only "yesterday" rule should be expired
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].pattern, "src/yesterday/**");
}

#[test]
fn test_empty_rules_no_expired() {
    let config = Config::default();
    let today = ParsedDate::parse("2025-06-15").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    assert!(expired.is_empty());
}

#[test]
fn test_rules_without_expires_never_expire() {
    let mut config = Config::default();
    config.content.rules = vec![
        ContentRule {
            pattern: "src/**".to_string(),
            max_lines: 1000,
            expires: None, // No expiration
            reason: Some("Permanent exemption".to_string()),
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
        ContentRule {
            pattern: "vendor/**".to_string(),
            max_lines: 5000,
            expires: None, // No expiration
            reason: None,
            warn_threshold: None,
            warn_at: None,
            skip_comments: None,
            skip_blank: None,
        },
    ];
    config.structure.rules = vec![StructureRule {
        scope: "generated/**".to_string(),
        max_files: Some(1000),
        expires: None,
        ..Default::default()
    }];

    // Even with a far future date, rules without expires should not appear
    let today = ParsedDate::parse("2099-12-31").unwrap();
    let expired = collect_expired_rules_with_date(&config, today);

    assert!(expired.is_empty());
}
