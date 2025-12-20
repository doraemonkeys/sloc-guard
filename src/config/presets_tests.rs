use super::*;
use toml::Value;

#[test]
fn load_rust_strict_preset() {
    let value = load_preset("rust-strict").expect("should load rust-strict preset");

    let table = value.as_table().expect("should be a table");
    assert_eq!(table.get("version").and_then(Value::as_str), Some("2"));

    let content = table.get("content").and_then(Value::as_table).unwrap();
    let extensions = content.get("extensions").and_then(Value::as_array).unwrap();
    assert!(extensions.iter().any(|v| v.as_str() == Some("rs")));
    assert_eq!(
        content.get("max_lines").and_then(Value::as_integer),
        Some(500)
    );

    let scanner = table.get("scanner").and_then(Value::as_table).unwrap();
    let exclude = scanner.get("exclude").and_then(Value::as_array).unwrap();
    assert!(exclude.iter().any(|v| v.as_str() == Some("target/**")));

    let structure = table.get("structure").and_then(Value::as_table).unwrap();
    assert_eq!(
        structure.get("max_files").and_then(Value::as_integer),
        Some(20)
    );
    assert_eq!(
        structure.get("max_dirs").and_then(Value::as_integer),
        Some(10)
    );
}

#[test]
fn load_node_strict_preset() {
    let value = load_preset("node-strict").expect("should load node-strict preset");

    let table = value.as_table().expect("should be a table");
    let content = table.get("content").and_then(Value::as_table).unwrap();
    let extensions = content.get("extensions").and_then(Value::as_array).unwrap();

    let ext_strs: Vec<&str> = extensions.iter().filter_map(Value::as_str).collect();
    assert!(ext_strs.contains(&"js"));
    assert!(ext_strs.contains(&"ts"));
    assert!(ext_strs.contains(&"tsx"));
    assert_eq!(
        content.get("max_lines").and_then(Value::as_integer),
        Some(400)
    );

    let scanner = table.get("scanner").and_then(Value::as_table).unwrap();
    let exclude = scanner.get("exclude").and_then(Value::as_array).unwrap();
    assert!(
        exclude
            .iter()
            .any(|v| v.as_str() == Some("node_modules/**"))
    );
}

#[test]
fn load_python_strict_preset() {
    let value = load_preset("python-strict").expect("should load python-strict preset");

    let table = value.as_table().expect("should be a table");
    let content = table.get("content").and_then(Value::as_table).unwrap();
    let extensions = content.get("extensions").and_then(Value::as_array).unwrap();

    let ext_strs: Vec<&str> = extensions.iter().filter_map(Value::as_str).collect();
    assert!(ext_strs.contains(&"py"));
    assert!(ext_strs.contains(&"pyi"));
    assert_eq!(
        content.get("max_lines").and_then(Value::as_integer),
        Some(400)
    );

    let scanner = table.get("scanner").and_then(Value::as_table).unwrap();
    let exclude = scanner.get("exclude").and_then(Value::as_array).unwrap();
    assert!(exclude.iter().any(|v| v.as_str() == Some("__pycache__/**")));
}

#[test]
fn load_monorepo_base_preset() {
    let value = load_preset("monorepo-base").expect("should load monorepo-base preset");

    let table = value.as_table().expect("should be a table");
    let content = table.get("content").and_then(Value::as_table).unwrap();
    let extensions = content.get("extensions").and_then(Value::as_array).unwrap();

    let ext_strs: Vec<&str> = extensions.iter().filter_map(Value::as_str).collect();
    assert!(ext_strs.contains(&"rs"));
    assert!(ext_strs.contains(&"js"));
    assert!(ext_strs.contains(&"py"));
    assert!(ext_strs.contains(&"go"));
    assert_eq!(
        content.get("max_lines").and_then(Value::as_integer),
        Some(600)
    );

    let structure = table.get("structure").and_then(Value::as_table).unwrap();
    assert_eq!(
        structure.get("max_files").and_then(Value::as_integer),
        Some(30)
    );
    assert_eq!(
        structure.get("max_dirs").and_then(Value::as_integer),
        Some(20)
    );
}

#[test]
fn unknown_preset_returns_error() {
    let result = load_preset("nonexistent");

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Unknown preset: 'nonexistent'"));
    assert!(msg.contains("rust-strict"));
    assert!(msg.contains("node-strict"));
    assert!(msg.contains("python-strict"));
    assert!(msg.contains("monorepo-base"));
}

#[test]
fn available_presets_constant_matches_load_preset() {
    for preset_name in AVAILABLE_PRESETS {
        let result = load_preset(preset_name);
        assert!(
            result.is_ok(),
            "Preset '{preset_name}' listed in AVAILABLE_PRESETS but load_preset failed: {:?}",
            result.err()
        );
    }
}

#[test]
fn all_presets_have_version_2() {
    for preset_name in AVAILABLE_PRESETS {
        let value = load_preset(preset_name).expect("should load preset");
        let version = value
            .get("version")
            .and_then(Value::as_str)
            .expect("should have version");
        assert_eq!(version, "2", "Preset '{preset_name}' should be version 2");
    }
}

#[test]
fn all_presets_have_required_sections() {
    for preset_name in AVAILABLE_PRESETS {
        let value = load_preset(preset_name).expect("should load preset");
        let table = value.as_table().expect("should be a table");

        assert!(
            table.contains_key("scanner"),
            "Preset '{preset_name}' should have [scanner] section"
        );
        assert!(
            table.contains_key("content"),
            "Preset '{preset_name}' should have [content] section"
        );
        assert!(
            table.contains_key("structure"),
            "Preset '{preset_name}' should have [structure] section"
        );
    }
}
