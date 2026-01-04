//! Tests for array merge functionality with $reset marker support.

use toml::Value;

use crate::config::merge::{
    RESET_MARKER, has_any_reset_markers, has_reset_marker, is_reset_element, merge_arrays,
};

#[test]
fn is_reset_element_detects_string_marker() {
    let value = Value::String(RESET_MARKER.to_string());
    assert!(is_reset_element(&value));
}

#[test]
fn is_reset_element_ignores_other_strings() {
    let value = Value::String("not_reset".to_string());
    assert!(!is_reset_element(&value));
}

#[test]
fn is_reset_element_detects_table_with_pattern_reset() {
    let mut table = toml::map::Map::new();
    table.insert(
        "pattern".to_string(),
        Value::String(RESET_MARKER.to_string()),
    );
    table.insert("max_lines".to_string(), Value::Integer(0));
    let value = Value::Table(table);
    assert!(is_reset_element(&value));
}

#[test]
fn is_reset_element_detects_table_with_scope_reset() {
    let mut table = toml::map::Map::new();
    table.insert("scope".to_string(), Value::String(RESET_MARKER.to_string()));
    let value = Value::Table(table);
    assert!(is_reset_element(&value));
}

#[test]
fn is_reset_element_ignores_table_without_reset() {
    let mut table = toml::map::Map::new();
    table.insert("pattern".to_string(), Value::String("**/*.rs".to_string()));
    table.insert("max_lines".to_string(), Value::Integer(500));
    let value = Value::Table(table);
    assert!(!is_reset_element(&value));
}

#[test]
fn has_reset_marker_detects_first_string() {
    let arr = vec![
        Value::String(RESET_MARKER.to_string()),
        Value::String("pattern1".to_string()),
    ];
    assert!(has_reset_marker(&arr));
}

#[test]
fn has_reset_marker_ignores_later_positions() {
    // Note: We don't use has_reset_marker to validate position - that's validate_reset_positions
    // This just checks if the array starts with reset
    let arr = vec![
        Value::String("pattern1".to_string()),
        Value::String(RESET_MARKER.to_string()),
    ];
    assert!(!has_reset_marker(&arr));
}

#[test]
fn has_reset_marker_handles_empty_array() {
    let arr: Vec<Value> = vec![];
    assert!(!has_reset_marker(&arr));
}

#[test]
fn merge_arrays_appends_by_default() {
    let base = vec![
        Value::String("a".to_string()),
        Value::String("b".to_string()),
    ];
    let child = vec![
        Value::String("c".to_string()),
        Value::String("d".to_string()),
    ];

    let result = merge_arrays(base, child);
    let arr = result.as_array().unwrap();

    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0].as_str().unwrap(), "a");
    assert_eq!(arr[1].as_str().unwrap(), "b");
    assert_eq!(arr[2].as_str().unwrap(), "c");
    assert_eq!(arr[3].as_str().unwrap(), "d");
}

#[test]
fn merge_arrays_reset_clears_base() {
    let base = vec![
        Value::String("a".to_string()),
        Value::String("b".to_string()),
    ];
    let child = vec![
        Value::String(RESET_MARKER.to_string()),
        Value::String("c".to_string()),
        Value::String("d".to_string()),
    ];

    let result = merge_arrays(base, child);
    let arr = result.as_array().unwrap();

    // Reset marker itself is skipped
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_str().unwrap(), "c");
    assert_eq!(arr[1].as_str().unwrap(), "d");
}

#[test]
fn merge_arrays_reset_with_empty_child() {
    let base = vec![
        Value::String("a".to_string()),
        Value::String("b".to_string()),
    ];
    let child = vec![Value::String(RESET_MARKER.to_string())];

    let result = merge_arrays(base, child);
    let arr = result.as_array().unwrap();

    // Reset clears base and nothing follows
    assert_eq!(arr.len(), 0);
}

#[test]
fn merge_arrays_empty_child_appends_nothing() {
    let base = vec![
        Value::String("a".to_string()),
        Value::String("b".to_string()),
    ];
    let child: Vec<Value> = vec![];

    let result = merge_arrays(base, child);
    let arr = result.as_array().unwrap();

    assert_eq!(arr.len(), 2);
}

#[test]
fn merge_arrays_empty_base_uses_child() {
    let base: Vec<Value> = vec![];
    let child = vec![
        Value::String("c".to_string()),
        Value::String("d".to_string()),
    ];

    let result = merge_arrays(base, child);
    let arr = result.as_array().unwrap();

    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_str().unwrap(), "c");
    assert_eq!(arr[1].as_str().unwrap(), "d");
}

#[test]
fn merge_arrays_preserves_table_elements() {
    let mut base_table = toml::map::Map::new();
    base_table.insert("pattern".to_string(), Value::String("**/*.rs".to_string()));
    base_table.insert("max_lines".to_string(), Value::Integer(300));

    let mut child_table = toml::map::Map::new();
    child_table.insert("pattern".to_string(), Value::String("**/*.go".to_string()));
    child_table.insert("max_lines".to_string(), Value::Integer(500));

    let base = vec![Value::Table(base_table)];
    let child = vec![Value::Table(child_table)];

    let result = merge_arrays(base, child);
    let arr = result.as_array().unwrap();

    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].get("pattern").unwrap().as_str().unwrap(), "**/*.rs");
    assert_eq!(arr[1].get("pattern").unwrap().as_str().unwrap(), "**/*.go");
}

#[test]
fn merge_arrays_reset_table_clears_base() {
    let mut base_table = toml::map::Map::new();
    base_table.insert("pattern".to_string(), Value::String("**/*.rs".to_string()));
    base_table.insert("max_lines".to_string(), Value::Integer(300));

    let mut reset_table = toml::map::Map::new();
    reset_table.insert(
        "pattern".to_string(),
        Value::String(RESET_MARKER.to_string()),
    );
    reset_table.insert("max_lines".to_string(), Value::Integer(0));

    let mut child_table = toml::map::Map::new();
    child_table.insert("pattern".to_string(), Value::String("**/*.go".to_string()));
    child_table.insert("max_lines".to_string(), Value::Integer(500));

    let base = vec![Value::Table(base_table)];
    let child = vec![Value::Table(reset_table), Value::Table(child_table)];

    let result = merge_arrays(base, child);
    let arr = result.as_array().unwrap();

    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].get("pattern").unwrap().as_str().unwrap(), "**/*.go");
}

// ===== has_any_reset_markers tests =====

#[test]
fn has_any_reset_markers_returns_false_for_empty_table() {
    let value = Value::Table(toml::map::Map::new());
    assert!(!has_any_reset_markers(&value));
}

#[test]
fn has_any_reset_markers_returns_false_for_simple_values() {
    assert!(!has_any_reset_markers(&Value::String("hello".to_string())));
    assert!(!has_any_reset_markers(&Value::Integer(42)));
    assert!(!has_any_reset_markers(&Value::Boolean(true)));
}

#[test]
fn has_any_reset_markers_returns_false_for_array_without_markers() {
    let arr = Value::Array(vec![
        Value::String("pattern1".to_string()),
        Value::String("pattern2".to_string()),
    ]);
    assert!(!has_any_reset_markers(&arr));
}

#[test]
fn has_any_reset_markers_detects_string_marker_in_array() {
    let arr = Value::Array(vec![
        Value::String(RESET_MARKER.to_string()),
        Value::String("pattern1".to_string()),
    ]);
    assert!(has_any_reset_markers(&arr));
}

#[test]
fn has_any_reset_markers_detects_table_marker_in_array() {
    let mut reset_table = toml::map::Map::new();
    reset_table.insert(
        "pattern".to_string(),
        Value::String(RESET_MARKER.to_string()),
    );

    let arr = Value::Array(vec![Value::Table(reset_table)]);
    assert!(has_any_reset_markers(&arr));
}

#[test]
fn has_any_reset_markers_detects_nested_marker_in_table() {
    // Create: { scanner: { exclude: ["$reset", "pattern"] } }
    let exclude_arr = Value::Array(vec![
        Value::String(RESET_MARKER.to_string()),
        Value::String("**/build/**".to_string()),
    ]);

    let mut scanner = toml::map::Map::new();
    scanner.insert("exclude".to_string(), exclude_arr);

    let mut root = toml::map::Map::new();
    root.insert("scanner".to_string(), Value::Table(scanner));

    assert!(has_any_reset_markers(&Value::Table(root)));
}

#[test]
fn has_any_reset_markers_detects_deeply_nested_marker() {
    // Create: { content: { rules: [{ pattern: "$reset" }] } }
    let mut reset_rule = toml::map::Map::new();
    reset_rule.insert(
        "pattern".to_string(),
        Value::String(RESET_MARKER.to_string()),
    );

    let rules_arr = Value::Array(vec![Value::Table(reset_rule)]);

    let mut content = toml::map::Map::new();
    content.insert("rules".to_string(), rules_arr);

    let mut root = toml::map::Map::new();
    root.insert("content".to_string(), Value::Table(content));

    assert!(has_any_reset_markers(&Value::Table(root)));
}

#[test]
fn has_any_reset_markers_returns_false_for_typical_config() {
    // Typical config without any reset markers
    let mut content = toml::map::Map::new();
    content.insert("max_lines".to_string(), Value::Integer(500));
    content.insert(
        "extensions".to_string(),
        Value::Array(vec![
            Value::String("rs".to_string()),
            Value::String("py".to_string()),
        ]),
    );

    let mut rule = toml::map::Map::new();
    rule.insert("pattern".to_string(), Value::String("**/*.rs".to_string()));
    rule.insert("max_lines".to_string(), Value::Integer(300));
    content.insert("rules".to_string(), Value::Array(vec![Value::Table(rule)]));

    let mut root = toml::map::Map::new();
    root.insert("version".to_string(), Value::String("2".to_string()));
    root.insert("content".to_string(), Value::Table(content));

    assert!(!has_any_reset_markers(&Value::Table(root)));
}

#[test]
fn has_any_reset_markers_detects_marker_in_non_first_position() {
    // "$reset" in an invalid (non-first) position must still be detected
    // so that validate_reset_positions can catch and reject it
    let arr = Value::Array(vec![
        Value::String("pattern1".to_string()),
        Value::String(RESET_MARKER.to_string()), // Invalid position
        Value::String("pattern2".to_string()),
    ]);
    assert!(has_any_reset_markers(&arr));
}

#[test]
fn has_any_reset_markers_detects_marker_at_end_of_array() {
    // Edge case: marker at the very end
    let arr = Value::Array(vec![
        Value::String("first".to_string()),
        Value::String("second".to_string()),
        Value::String(RESET_MARKER.to_string()), // End position
    ]);
    assert!(has_any_reset_markers(&arr));
}

#[test]
fn has_any_reset_markers_detects_nested_table_marker_in_non_first_position() {
    // Table marker in non-first position of rules array
    let mut normal_rule = toml::map::Map::new();
    normal_rule.insert("pattern".to_string(), Value::String("src/**".to_string()));

    let mut reset_rule = toml::map::Map::new();
    reset_rule.insert(
        "pattern".to_string(),
        Value::String(RESET_MARKER.to_string()),
    );

    let arr = Value::Array(vec![
        Value::Table(normal_rule),
        Value::Table(reset_rule), // Invalid: not in first position
    ]);
    assert!(has_any_reset_markers(&arr));
}
