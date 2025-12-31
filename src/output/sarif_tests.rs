use std::path::PathBuf;

use super::*;
use crate::counter::LineStats;

fn make_passed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Passed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        violation_category: None,
    }
}

fn make_warning_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Warning {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

fn make_failed_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Failed {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }
}

fn make_grandfathered_result(path: &str, code: usize, limit: usize) -> CheckResult {
    CheckResult::Grandfathered {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit,
        override_reason: None,
        violation_category: None,
    }
}

#[test]
fn sarif_output_is_valid_json() {
    let formatter = SarifFormatter::new();
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(parsed.get("$schema").is_some());
    assert_eq!(parsed.get("version").unwrap(), "2.1.0");
    assert!(parsed.get("runs").is_some());
}

#[test]
fn sarif_has_correct_schema() {
    let formatter = SarifFormatter::new();
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(
        parsed.get("$schema").unwrap(),
        "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json"
    );
}

#[test]
fn sarif_tool_info() {
    let formatter = SarifFormatter::new();
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let driver = &parsed["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "sloc-guard");
    assert!(driver.get("version").is_some());
    assert!(driver.get("rules").is_some());
}

#[test]
fn sarif_rules_defined() {
    let formatter = SarifFormatter::new();
    let results: Vec<CheckResult> = vec![];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    // 2 content rules + 8 structure rules = 10 total
    assert_eq!(rules.len(), 10);
    // Content rules at indices 0-1
    assert_eq!(rules[0]["id"], "sloc-guard/line-limit-exceeded");
    assert_eq!(rules[1]["id"], "sloc-guard/line-limit-warning");
    // Structure rules at indices 2-9
    assert_eq!(rules[2]["id"], "sloc-guard/structure-file-count");
    assert_eq!(rules[3]["id"], "sloc-guard/structure-dir-count");
}

#[test]
fn sarif_failed_result() {
    let formatter = SarifFormatter::new();
    let results = vec![make_failed_result("src/big_file.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let result = &parsed["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "sloc-guard/line-limit-exceeded");
    assert_eq!(result["level"], "error");
    assert_eq!(result["ruleIndex"], 0);

    let location = &result["locations"][0]["physicalLocation"]["artifactLocation"];
    assert_eq!(location["uri"], "src/big_file.rs");
}

#[test]
fn sarif_warning_result() {
    let formatter = SarifFormatter::new();
    let results = vec![make_warning_result("src/medium_file.rs", 460, 500)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let result = &parsed["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "sloc-guard/line-limit-warning");
    assert_eq!(result["level"], "warning");
    assert_eq!(result["ruleIndex"], 1);
}

#[test]
fn sarif_grandfathered_result_has_suppression() {
    let formatter = SarifFormatter::new();
    let results = vec![make_grandfathered_result("src/legacy.rs", 700, 500)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let result = &parsed["runs"][0]["results"][0];
    assert_eq!(result["level"], "note");
    assert!(result.get("suppressions").is_some());

    let suppression = &result["suppressions"][0];
    assert_eq!(suppression["kind"], "external");
}

#[test]
fn sarif_passed_results_excluded() {
    let formatter = SarifFormatter::new();
    let results = vec![
        make_passed_result("pass.rs", 100, 500),
        make_failed_result("fail.rs", 600, 500),
    ];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let sarif_results = parsed["runs"][0]["results"].as_array().unwrap();
    assert_eq!(sarif_results.len(), 1);
    assert_eq!(
        sarif_results[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "fail.rs"
    );
}

#[test]
fn sarif_empty_results() {
    let formatter = SarifFormatter::new();
    let results: Vec<CheckResult> = vec![];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let sarif_results = parsed["runs"][0]["results"].as_array().unwrap();
    assert!(sarif_results.is_empty());
}

#[test]
fn sarif_result_properties() {
    let formatter = SarifFormatter::new();
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let props = &parsed["runs"][0]["results"][0]["properties"];
    assert_eq!(props["sloc"], 600);
    assert_eq!(props["limit"], 500);
    assert!(props.get("usagePercent").is_some());
    assert_eq!(props["stats"]["code"], 600);
}

#[test]
fn sarif_windows_path_converted() {
    let formatter = SarifFormatter::new();
    let results = vec![CheckResult::Failed {
        path: PathBuf::from("src\\subdir\\file.rs"),
        stats: LineStats {
            total: 610,
            code: 600,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let uri = &parsed["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
        ["uri"];
    assert_eq!(uri, "src/subdir/file.rs");
}

#[test]
fn sarif_with_suggestions() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let result = make_failed_result("big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = SarifFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let props = &parsed["runs"][0]["results"][0]["properties"];
    assert!(props.get("suggestions").is_some());
}

#[test]
fn sarif_without_suggestions_flag_excludes_suggestions() {
    use crate::analyzer::{SplitChunk, SplitSuggestion};

    let result = make_failed_result("big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = SarifFormatter::new().with_suggestions(false);
    let output = formatter.format(&[result]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let props = &parsed["runs"][0]["results"][0]["properties"];
    assert!(props.get("suggestions").is_none());
}

#[test]
fn sarif_default_formatter() {
    let formatter = SarifFormatter::default();
    let results = vec![make_failed_result("test.rs", 600, 500)];

    let output = formatter.format(&results).unwrap();
    assert!(output.contains("$schema"));
}

#[test]
fn sarif_override_reason_included() {
    let results = vec![CheckResult::Warning {
        path: PathBuf::from("legacy.rs"),
        stats: LineStats {
            total: 460,
            code: 450,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        raw_stats: None,
        limit: 500,
        override_reason: Some("Legacy migration code".to_string()),
        suggestions: None,
        violation_category: None,
    }];

    let formatter = SarifFormatter::new();
    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let props = &parsed["runs"][0]["results"][0]["properties"];
    assert_eq!(props["overrideReason"], "Legacy migration code");
}

// ============================================================================
// Structure violation tests
// ============================================================================

mod structure_violations {
    use super::*;
    use crate::checker::{ViolationCategory, ViolationType};

    fn make_structure_failed(
        path: &str,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
    ) -> CheckResult {
        CheckResult::Failed {
            path: PathBuf::from(path),
            stats: LineStats {
                total: 0,
                code: actual,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            raw_stats: None,
            limit,
            override_reason: None,
            suggestions: None,
            violation_category: Some(ViolationCategory::Structure {
                violation_type,
                triggering_rule: None,
            }),
        }
    }

    fn make_structure_warning(
        path: &str,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
    ) -> CheckResult {
        CheckResult::Warning {
            path: PathBuf::from(path),
            stats: LineStats {
                total: 0,
                code: actual,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            raw_stats: None,
            limit,
            override_reason: None,
            suggestions: None,
            violation_category: Some(ViolationCategory::Structure {
                violation_type,
                triggering_rule: None,
            }),
        }
    }

    fn make_structure_grandfathered(
        path: &str,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
    ) -> CheckResult {
        CheckResult::Grandfathered {
            path: PathBuf::from(path),
            stats: LineStats {
                total: 0,
                code: actual,
                comment: 0,
                blank: 0,
                ignored: 0,
            },
            raw_stats: None,
            limit,
            override_reason: None,
            violation_category: Some(ViolationCategory::Structure {
                violation_type,
                triggering_rule: None,
            }),
        }
    }

    #[test]
    fn sarif_rules_include_structure_rules() {
        let formatter = SarifFormatter::new();
        let results: Vec<CheckResult> = vec![];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        // 2 content rules + 8 structure rules = 10 total
        assert_eq!(rules.len(), 10);

        // Verify structure rule IDs exist
        let rule_ids: Vec<&str> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert!(rule_ids.contains(&"sloc-guard/structure-file-count"));
        assert!(rule_ids.contains(&"sloc-guard/structure-dir-count"));
        assert!(rule_ids.contains(&"sloc-guard/structure-max-depth"));
        assert!(rule_ids.contains(&"sloc-guard/structure-disallowed-file"));
        assert!(rule_ids.contains(&"sloc-guard/structure-disallowed-dir"));
        assert!(rule_ids.contains(&"sloc-guard/structure-denied"));
        assert!(rule_ids.contains(&"sloc-guard/structure-naming"));
        assert!(rule_ids.contains(&"sloc-guard/structure-sibling"));
    }

    #[test]
    fn sarif_file_count_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/components",
            ViolationType::FileCount,
            25,
            20,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-file-count");
        assert_eq!(result["level"], "error");
        assert_eq!(result["ruleIndex"], 2);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("25 files"));
        assert!(message.contains("limit of 20"));
    }

    #[test]
    fn sarif_dir_count_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/modules",
            ViolationType::DirCount,
            15,
            10,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-dir-count");
        assert_eq!(result["ruleIndex"], 3);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("15 subdirectories"));
        assert!(message.contains("limit of 10"));
    }

    #[test]
    fn sarif_max_depth_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/a/b/c/d/e/f",
            ViolationType::MaxDepth,
            6,
            5,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-max-depth");
        assert_eq!(result["ruleIndex"], 4);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("depth is 6"));
        assert!(message.contains("limit of 5"));
    }

    #[test]
    fn sarif_disallowed_file_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/components/test.txt",
            ViolationType::DisallowedFile,
            1,
            0,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-disallowed-file");
        assert_eq!(result["ruleIndex"], 5);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("not allowed"));
    }

    #[test]
    fn sarif_disallowed_directory_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/temp",
            ViolationType::DisallowedDirectory,
            1,
            0,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-disallowed-dir");
        assert_eq!(result["ruleIndex"], 6);
    }

    #[test]
    fn sarif_denied_file_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "build/output.exe",
            ViolationType::DeniedFile {
                pattern_or_extension: ".exe".to_string(),
            },
            1,
            0,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-denied");
        assert_eq!(result["ruleIndex"], 7);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("deny pattern '.exe'"));
    }

    #[test]
    fn sarif_denied_directory_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "vendor/node_modules",
            ViolationType::DeniedDirectory {
                pattern: "**/node_modules/".to_string(),
            },
            1,
            0,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-denied");
        assert_eq!(result["ruleIndex"], 7);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("deny pattern"));
        assert!(message.contains("node_modules"));
    }

    #[test]
    fn sarif_naming_convention_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/components/MyComponent.tsx",
            ViolationType::NamingConvention {
                expected_pattern: "^[a-z][a-z0-9_]*\\.tsx$".to_string(),
            },
            1,
            0,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-naming");
        assert_eq!(result["ruleIndex"], 8);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("does not match required pattern"));
    }

    #[test]
    fn sarif_missing_sibling_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/components/Button.tsx",
            ViolationType::MissingSibling {
                expected_sibling_pattern: "{stem}.test.tsx".to_string(),
            },
            1,
            1,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-sibling");
        assert_eq!(result["ruleIndex"], 9);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("Missing required sibling"));
        assert!(message.contains("{stem}.test.tsx"));
    }

    #[test]
    fn sarif_group_incomplete_violation() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_failed(
            "src/models/user.rs",
            ViolationType::GroupIncomplete {
                group_patterns: vec![
                    "{stem}.rs".to_string(),
                    "{stem}_test.rs".to_string(),
                    "{stem}_mock.rs".to_string(),
                ],
                missing_patterns: vec!["{stem}_mock.rs".to_string()],
            },
            1,
            1,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-sibling");
        assert_eq!(result["ruleIndex"], 9);

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("Incomplete file group"));
        assert!(message.contains("{stem}_mock.rs"));
    }

    #[test]
    fn sarif_structure_warning_level() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_warning(
            "src/large_dir",
            ViolationType::FileCount,
            18,
            20,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-file-count");
        assert_eq!(result["level"], "warning");
    }

    #[test]
    fn sarif_structure_grandfathered() {
        let formatter = SarifFormatter::new();
        let results = vec![make_structure_grandfathered(
            "legacy/big_dir",
            ViolationType::FileCount,
            30,
            20,
        )];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let result = &parsed["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "sloc-guard/structure-file-count");
        assert_eq!(result["level"], "note");
        assert!(result.get("suppressions").is_some());

        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("(grandfathered)"));
    }

    #[test]
    fn sarif_mixed_content_and_structure_violations() {
        let formatter = SarifFormatter::new();
        let results = vec![
            make_failed_result("src/big_file.rs", 600, 500),
            make_structure_failed("src/components", ViolationType::FileCount, 25, 20),
            make_warning_result("src/medium_file.rs", 460, 500),
            make_structure_warning("src/modules", ViolationType::DirCount, 9, 10),
        ];

        let output = formatter.format(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        let sarif_results = parsed["runs"][0]["results"].as_array().unwrap();
        assert_eq!(sarif_results.len(), 4);

        // Content violation
        assert_eq!(sarif_results[0]["ruleId"], "sloc-guard/line-limit-exceeded");
        // Structure violation
        assert_eq!(
            sarif_results[1]["ruleId"],
            "sloc-guard/structure-file-count"
        );
        // Content warning
        assert_eq!(sarif_results[2]["ruleId"], "sloc-guard/line-limit-warning");
        // Structure warning
        assert_eq!(sarif_results[3]["ruleId"], "sloc-guard/structure-dir-count");
    }
}
