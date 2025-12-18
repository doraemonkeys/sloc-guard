use std::path::PathBuf;

use super::*;
use crate::counter::LineStats;

fn make_result(path: &str, code: usize, limit: usize, status: CheckStatus) -> CheckResult {
    CheckResult {
        path: PathBuf::from(path),
        status,
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit,
        override_reason: None,
    }
}

#[test]
fn sarif_output_is_valid_json() {
    let formatter = SarifFormatter;
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(parsed.get("$schema").is_some());
    assert_eq!(parsed.get("version").unwrap(), "2.1.0");
    assert!(parsed.get("runs").is_some());
}

#[test]
fn sarif_has_correct_schema() {
    let formatter = SarifFormatter;
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(
        parsed.get("$schema").unwrap(),
        "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json"
    );
}

#[test]
fn sarif_tool_info() {
    let formatter = SarifFormatter;
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let driver = &parsed["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "sloc-guard");
    assert!(driver.get("version").is_some());
    assert!(driver.get("rules").is_some());
}

#[test]
fn sarif_rules_defined() {
    let formatter = SarifFormatter;
    let results: Vec<CheckResult> = vec![];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0]["id"], "sloc-guard/line-limit-exceeded");
    assert_eq!(rules[1]["id"], "sloc-guard/line-limit-warning");
}

#[test]
fn sarif_failed_result() {
    let formatter = SarifFormatter;
    let results = vec![make_result("src/big_file.rs", 600, 500, CheckStatus::Failed)];

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
    let formatter = SarifFormatter;
    let results = vec![make_result("src/medium_file.rs", 460, 500, CheckStatus::Warning)];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let result = &parsed["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "sloc-guard/line-limit-warning");
    assert_eq!(result["level"], "warning");
    assert_eq!(result["ruleIndex"], 1);
}

#[test]
fn sarif_grandfathered_result_has_suppression() {
    let formatter = SarifFormatter;
    let results = vec![make_result("src/legacy.rs", 700, 500, CheckStatus::Grandfathered)];

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
    let formatter = SarifFormatter;
    let results = vec![
        make_result("pass.rs", 100, 500, CheckStatus::Passed),
        make_result("fail.rs", 600, 500, CheckStatus::Failed),
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
    let formatter = SarifFormatter;
    let results: Vec<CheckResult> = vec![];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let sarif_results = parsed["runs"][0]["results"].as_array().unwrap();
    assert!(sarif_results.is_empty());
}

#[test]
fn sarif_result_properties() {
    let formatter = SarifFormatter;
    let results = vec![make_result("test.rs", 600, 500, CheckStatus::Failed)];

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
    let formatter = SarifFormatter;
    let results = vec![CheckResult {
        path: PathBuf::from("src\\subdir\\file.rs"),
        status: CheckStatus::Failed,
        stats: LineStats {
            total: 610,
            code: 600,
            comment: 5,
            blank: 5, ignored: 0,
        },
        limit: 500,
        override_reason: None,
    }];

    let output = formatter.format(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let uri =
        &parsed["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
            ["uri"];
    assert_eq!(uri, "src/subdir/file.rs");
}
