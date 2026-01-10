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
        Some(600)
    );

    // Verify test file relaxation rules
    let rules = content.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        !rules.is_empty(),
        "rust-strict should have content rules for test files"
    );
    let test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("_test.rs"))
    });
    assert!(
        test_rule.is_some(),
        "Should have a rule for *_test.rs files"
    );
    assert_eq!(
        test_rule
            .unwrap()
            .get("max_lines")
            .and_then(Value::as_integer),
        Some(1000),
        "Test files should have relaxed limit of 1000 lines"
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

    // Verify structure rules for test directories
    let struct_rules = structure.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        !struct_rules.is_empty(),
        "rust-strict should have structure rules"
    );

    // Verify deny lists
    let deny_files = structure
        .get("deny_files")
        .and_then(Value::as_array)
        .unwrap();
    assert!(
        deny_files.iter().any(|v| v.as_str() == Some("*.bak")),
        "Should deny .bak files"
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
    assert!(
        ext_strs.contains(&"vue"),
        "node-strict should include vue files"
    );
    assert!(
        ext_strs.contains(&"svelte"),
        "node-strict should include svelte files"
    );
    assert_eq!(
        content.get("max_lines").and_then(Value::as_integer),
        Some(600)
    );

    // Verify test file relaxation rules
    let rules = content.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        !rules.is_empty(),
        "node-strict should have content rules for test files"
    );
    let test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains(".test."))
    });
    assert!(test_rule.is_some(), "Should have a rule for *.test.* files");

    // Verify storybook rules
    let story_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("stories"))
    });
    assert!(story_rule.is_some(), "Should have a rule for story files");

    let scanner = table.get("scanner").and_then(Value::as_table).unwrap();
    let exclude = scanner.get("exclude").and_then(Value::as_array).unwrap();
    assert!(
        exclude
            .iter()
            .any(|v| v.as_str() == Some("node_modules/**"))
    );
    assert!(
        exclude.iter().any(|v| v.as_str() == Some(".nuxt/**")),
        "Should exclude Nuxt build directory"
    );

    let structure = table.get("structure").and_then(Value::as_table).unwrap();
    // Verify structure rules for test and component directories
    let struct_rules = structure.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        struct_rules.len() >= 3,
        "node-strict should have multiple structure rules"
    );

    // Verify deny lists
    let deny_files = structure
        .get("deny_files")
        .and_then(Value::as_array)
        .unwrap();
    assert!(
        deny_files
            .iter()
            .any(|v| v.as_str() == Some("npm-debug.log*")),
        "Should deny npm debug logs"
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
        Some(600)
    );

    // Verify test file relaxation rules
    let rules = content.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        !rules.is_empty(),
        "python-strict should have content rules for test files"
    );
    let test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("test_"))
    });
    assert!(
        test_rule.is_some(),
        "Should have a rule for test_*.py files"
    );

    // Verify conftest rule
    let conftest_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("conftest"))
    });
    assert!(
        conftest_rule.is_some(),
        "Should have a rule for conftest.py"
    );

    // Verify migrations rule
    let migration_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("migrations"))
    });
    assert!(
        migration_rule.is_some(),
        "Should have a rule for migration files"
    );

    let scanner = table.get("scanner").and_then(Value::as_table).unwrap();
    let exclude = scanner.get("exclude").and_then(Value::as_array).unwrap();
    assert!(exclude.iter().any(|v| v.as_str() == Some("__pycache__/**")));
    assert!(
        exclude.iter().any(|v| v.as_str() == Some(".mypy_cache/**")),
        "Should exclude mypy cache"
    );
    assert!(
        exclude.iter().any(|v| v.as_str() == Some(".ruff_cache/**")),
        "Should exclude ruff cache"
    );

    let structure = table.get("structure").and_then(Value::as_table).unwrap();
    // Verify structure rules for test directories
    let struct_rules = structure.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        !struct_rules.is_empty(),
        "python-strict should have structure rules"
    );

    // Verify deny lists
    let deny_dirs = structure
        .get("deny_dirs")
        .and_then(Value::as_array)
        .unwrap();
    assert!(
        deny_dirs.iter().any(|v| v.as_str() == Some("__pycache__")),
        "Should deny __pycache__ directories"
    );
}

#[test]
fn load_go_strict_preset() {
    let value = load_preset("go-strict").expect("should load go-strict preset");

    let table = value.as_table().expect("should be a table");
    assert_eq!(table.get("version").and_then(Value::as_str), Some("2"));

    let content = table.get("content").and_then(Value::as_table).unwrap();
    let extensions = content.get("extensions").and_then(Value::as_array).unwrap();

    assert!(
        extensions
            .iter()
            .filter_map(Value::as_str)
            .any(|x| x == "go")
    );
    assert_eq!(
        content.get("max_lines").and_then(Value::as_integer),
        Some(600)
    );

    // Verify test file relaxation rules
    let rules = content.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        !rules.is_empty(),
        "go-strict should have content rules for test files"
    );
    let test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("_test.go"))
    });
    assert!(
        test_rule.is_some(),
        "Should have a rule for *_test.go files"
    );
    assert_eq!(
        test_rule
            .unwrap()
            .get("max_lines")
            .and_then(Value::as_integer),
        Some(1000),
        "Test files should have relaxed limit of 1000 lines"
    );

    // Verify generated file rules (protobuf)
    let pb_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains(".pb.go"))
    });
    assert!(
        pb_rule.is_some(),
        "Should have a rule for protobuf generated files"
    );

    // Verify mock file rules
    let mock_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("mock"))
    });
    assert!(mock_rule.is_some(), "Should have a rule for mock files");

    let scanner = table.get("scanner").and_then(Value::as_table).unwrap();
    let exclude = scanner.get("exclude").and_then(Value::as_array).unwrap();
    assert!(
        exclude.iter().any(|v| v.as_str() == Some("vendor/**")),
        "Should exclude vendor directory"
    );
    assert!(
        exclude.iter().any(|v| v.as_str() == Some("bin/**")),
        "Should exclude bin directory"
    );

    let structure = table.get("structure").and_then(Value::as_table).unwrap();
    assert_eq!(
        structure.get("max_files").and_then(Value::as_integer),
        Some(20)
    );
    assert_eq!(
        structure.get("max_dirs").and_then(Value::as_integer),
        Some(10)
    );

    // Verify structure rules
    let struct_rules = structure.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        !struct_rules.is_empty(),
        "go-strict should have structure rules"
    );

    // Verify testdata rule
    let testdata_rule = struct_rules.iter().find(|r| {
        r.get("scope")
            .and_then(Value::as_str)
            .is_some_and(|s| s.contains("testdata"))
    });
    assert!(
        testdata_rule.is_some(),
        "Should have a structure rule for testdata directory"
    );

    // Verify deny lists
    let deny_files = structure
        .get("deny_files")
        .and_then(Value::as_array)
        .unwrap();
    assert!(
        deny_files.iter().any(|v| v.as_str() == Some("*.bak")),
        "Should deny .bak files"
    );
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
    assert!(
        ext_strs.contains(&"vue"),
        "monorepo-base should include vue files"
    );
    assert!(
        ext_strs.contains(&"svelte"),
        "monorepo-base should include svelte files"
    );
    assert_eq!(
        content.get("max_lines").and_then(Value::as_integer),
        Some(600)
    );

    // Verify test file relaxation rules for multiple languages
    let rules = content.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        rules.len() >= 5,
        "monorepo-base should have multiple test file rules for different languages"
    );

    // Check for Rust test rules
    let rust_test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("_test.rs"))
    });
    assert!(rust_test_rule.is_some(), "Should have Rust test file rules");

    // Check for Node test rules
    let node_test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains(".test."))
    });
    assert!(node_test_rule.is_some(), "Should have Node test file rules");

    // Check for Python test rules
    let python_test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("test_"))
    });
    assert!(
        python_test_rule.is_some(),
        "Should have Python test file rules"
    );

    // Check for Go test rules
    let go_test_rule = rules.iter().find(|r| {
        r.get("pattern")
            .and_then(Value::as_str)
            .is_some_and(|p| p.contains("_test.go"))
    });
    assert!(go_test_rule.is_some(), "Should have Go test file rules");

    let structure = table.get("structure").and_then(Value::as_table).unwrap();
    assert_eq!(
        structure.get("max_files").and_then(Value::as_integer),
        Some(30)
    );
    assert_eq!(
        structure.get("max_dirs").and_then(Value::as_integer),
        Some(20)
    );

    // Verify structure rules for test directories
    let struct_rules = structure.get("rules").and_then(Value::as_array).unwrap();
    assert!(
        struct_rules.len() >= 3,
        "monorepo-base should have multiple structure rules for test directories"
    );

    // Verify deny lists
    let deny_files = structure
        .get("deny_files")
        .and_then(Value::as_array)
        .unwrap();
    assert!(
        deny_files.iter().any(|v| v.as_str() == Some("*.bak")),
        "Should deny .bak files"
    );
    let deny_ext = structure
        .get("deny_extensions")
        .and_then(Value::as_array)
        .unwrap();
    assert!(
        deny_ext.iter().any(|v| v.as_str() == Some(".exe")),
        "Should deny .exe files"
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
    assert!(msg.contains("go-strict"));
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
