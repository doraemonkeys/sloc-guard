use super::*;

#[test]
fn comment_syntax_construction() {
    let syntax = CommentSyntax::new(vec!["//"], vec![("/*", "*/")]);
    assert_eq!(syntax.single_line, vec!["//"]);
    assert_eq!(syntax.multi_line.len(), 1);
    assert_eq!(syntax.multi_line[0].start, "/*");
    assert_eq!(syntax.multi_line[0].end, "*/");
    assert!(!syntax.multi_line[0].supports_nesting);
    assert!(!syntax.multi_line[0].must_be_at_line_start);
}

#[test]
fn language_construction() {
    let syntax = CommentSyntax::new(vec!["//"], vec![("/*", "*/")]);
    let lang = Language::new("Test", vec!["test"], syntax);
    assert_eq!(lang.name, "Test");
    assert_eq!(lang.extensions, vec!["test"]);
}

#[test]
fn registry_register_and_lookup() {
    let mut registry = LanguageRegistry::new();
    let syntax = CommentSyntax::new(vec!["#"], vec![]);
    let lang = Language::new("Shell", vec!["sh", "bash"], syntax);

    registry.register(lang);

    assert!(registry.get_by_extension("sh").is_some());
    assert!(registry.get_by_extension("bash").is_some());
    assert_eq!(registry.get_by_extension("sh").unwrap().name, "Shell");
}

#[test]
fn default_registry_has_rust() {
    let registry = LanguageRegistry::default();
    let rust = registry.get_by_extension("rs").unwrap();

    assert_eq!(rust.name, "Rust");
    assert!(rust.comment_syntax.single_line.contains(&"//".to_string()));
    assert!(rust.comment_syntax.single_line.contains(&"///".to_string()));
}

#[test]
fn default_registry_has_python() {
    let registry = LanguageRegistry::default();
    let python = registry.get_by_extension("py").unwrap();

    assert_eq!(python.name, "Python");
    assert!(python.comment_syntax.single_line.contains(&"#".to_string()));
}

#[test]
fn registry_all_returns_all_languages() {
    let registry = LanguageRegistry::default();
    let all = registry.all();

    // 20 built-in languages as of this writing
    assert!(all.len() >= 20);
}

#[test]
fn default_registry_has_java() {
    let registry = LanguageRegistry::default();
    let java = registry.get_by_extension("java").unwrap();

    assert_eq!(java.name, "Java");
    assert!(java.comment_syntax.single_line.contains(&"//".to_string()));
}

#[test]
fn default_registry_has_kotlin() {
    let registry = LanguageRegistry::default();
    let kotlin = registry.get_by_extension("kt").unwrap();

    assert_eq!(kotlin.name, "Kotlin");
    assert!(
        kotlin
            .comment_syntax
            .single_line
            .contains(&"//".to_string())
    );
    // Also supports .kts extension
    assert!(registry.get_by_extension("kts").is_some());
}

#[test]
fn default_registry_has_swift() {
    let registry = LanguageRegistry::default();
    let swift = registry.get_by_extension("swift").unwrap();

    assert_eq!(swift.name, "Swift");
    // Swift has doc comments with ///
    assert!(
        swift
            .comment_syntax
            .single_line
            .contains(&"///".to_string())
    );
    // Swift supports nested block comments
    assert!(swift.comment_syntax.multi_line[0].supports_nesting);
}

#[test]
fn rust_supports_nested_block_comments() {
    let registry = LanguageRegistry::default();
    let rust = registry.get_by_extension("rs").unwrap();

    // Rust block comments support nesting
    assert_eq!(rust.comment_syntax.multi_line[0].start, "/*");
    assert_eq!(rust.comment_syntax.multi_line[0].end, "*/");
    assert!(rust.comment_syntax.multi_line[0].supports_nesting);
}

#[test]
fn default_registry_has_ruby() {
    let registry = LanguageRegistry::default();
    let ruby = registry.get_by_extension("rb").unwrap();

    assert_eq!(ruby.name, "Ruby");
    assert!(ruby.comment_syntax.single_line.contains(&"#".to_string()));
    // Ruby uses =begin/=end for multi-line comments (must be at line start)
    assert_eq!(ruby.comment_syntax.multi_line[0].start, "=begin");
    assert_eq!(ruby.comment_syntax.multi_line[0].end, "=end");
    assert!(ruby.comment_syntax.multi_line[0].must_be_at_line_start);
}

#[test]
fn default_registry_has_lua() {
    let registry = LanguageRegistry::default();
    let lua = registry.get_by_extension("lua").unwrap();

    assert_eq!(lua.name, "Lua");
    // Lua uses -- for single-line comments
    assert!(lua.comment_syntax.single_line.contains(&"--".to_string()));
    // Lua uses --[[ / ]] for multi-line comments
    assert_eq!(lua.comment_syntax.multi_line[0].start, "--[[");
    assert_eq!(lua.comment_syntax.multi_line[0].end, "]]");
}

#[test]
fn custom_language_overrides_builtin() {
    use std::collections::HashMap;

    let mut custom = HashMap::new();
    custom.insert(
        "CustomRust".to_string(),
        CustomLanguageConfig {
            extensions: vec!["rs".to_string()],
            single_line_comments: vec!["--".to_string()],
            multi_line_comments: vec![("{-".to_string(), "-}".to_string())],
        },
    );

    let registry = LanguageRegistry::with_custom_languages(&custom);
    let rust = registry.get_by_extension("rs").unwrap();

    assert_eq!(rust.name, "CustomRust");
    assert!(rust.comment_syntax.single_line.contains(&"--".to_string()));
}

#[test]
fn custom_language_adds_new_extension() {
    use std::collections::HashMap;

    let mut custom = HashMap::new();
    custom.insert(
        "Haskell".to_string(),
        CustomLanguageConfig {
            extensions: vec!["hs".to_string(), "lhs".to_string()],
            single_line_comments: vec!["--".to_string()],
            multi_line_comments: vec![("{-".to_string(), "-}".to_string())],
        },
    );

    let registry = LanguageRegistry::with_custom_languages(&custom);

    assert!(registry.get_by_extension("hs").is_some());
    assert!(registry.get_by_extension("lhs").is_some());
    assert_eq!(registry.get_by_extension("hs").unwrap().name, "Haskell");

    // Built-in languages should still be available
    assert!(registry.get_by_extension("rs").is_some());
    assert_eq!(registry.get_by_extension("rs").unwrap().name, "Rust");
}

#[test]
fn with_custom_languages_checked_reports_no_overrides_for_new_extensions() {
    use std::collections::HashMap;

    let mut custom = HashMap::new();
    custom.insert(
        "Haskell".to_string(),
        CustomLanguageConfig {
            extensions: vec!["hs".to_string()],
            single_line_comments: vec!["--".to_string()],
            multi_line_comments: vec![("{-".to_string(), "-}".to_string())],
        },
    );

    let (registry, overrides) = LanguageRegistry::with_custom_languages_checked(&custom);

    assert!(overrides.is_empty());
    assert_eq!(registry.get_by_extension("hs").unwrap().name, "Haskell");
}

#[test]
fn with_custom_languages_checked_reports_builtin_override() {
    use std::collections::HashMap;

    let mut custom = HashMap::new();
    custom.insert(
        "CustomRust".to_string(),
        CustomLanguageConfig {
            extensions: vec!["rs".to_string()],
            single_line_comments: vec!["--".to_string()],
            multi_line_comments: vec![],
        },
    );

    let (registry, overrides) = LanguageRegistry::with_custom_languages_checked(&custom);

    // Should report the override
    assert_eq!(overrides.len(), 1);
    let (ext, original, new) = &overrides[0];
    assert_eq!(ext, "rs");
    assert_eq!(original, "Rust");
    assert_eq!(new, "CustomRust");

    // Registry should use the custom language
    assert_eq!(registry.get_by_extension("rs").unwrap().name, "CustomRust");
}

#[test]
fn with_custom_languages_checked_reports_multiple_overrides() {
    use std::collections::HashMap;

    let mut custom = HashMap::new();
    custom.insert(
        "MyCStyle".to_string(),
        CustomLanguageConfig {
            extensions: vec!["c".to_string(), "h".to_string()],
            single_line_comments: vec!["//".to_string()],
            multi_line_comments: vec![("/*".to_string(), "*/".to_string())],
        },
    );

    let (_registry, overrides) = LanguageRegistry::with_custom_languages_checked(&custom);

    // Should report both .c and .h overrides (both were C language built-in)
    assert_eq!(overrides.len(), 2);

    let exts: Vec<&str> = overrides.iter().map(|(e, _, _)| e.as_str()).collect();
    assert!(exts.contains(&"c"));
    assert!(exts.contains(&"h"));

    // All overrides should be from "C" to "MyCStyle"
    for (_, original, new) in &overrides {
        assert_eq!(original, "C");
        assert_eq!(new, "MyCStyle");
    }
}
