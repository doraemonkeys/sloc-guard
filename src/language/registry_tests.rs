use super::*;

#[test]
fn comment_syntax_construction() {
    let syntax = CommentSyntax::new(vec!["//"], vec![("/*", "*/")]);
    assert_eq!(syntax.single_line, vec!["//"]);
    assert_eq!(syntax.multi_line, vec![("/*", "*/")]);
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
    assert!(rust.comment_syntax.single_line.contains(&"//"));
    assert!(rust.comment_syntax.single_line.contains(&"///"));
}

#[test]
fn default_registry_has_python() {
    let registry = LanguageRegistry::default();
    let python = registry.get_by_extension("py").unwrap();

    assert_eq!(python.name, "Python");
    assert!(python.comment_syntax.single_line.contains(&"#"));
}

#[test]
fn registry_all_returns_all_languages() {
    let registry = LanguageRegistry::default();
    let all = registry.all();

    assert!(all.len() >= 7);
}
