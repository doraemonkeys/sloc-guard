use super::*;

#[test]
fn registry_contains_builtin_languages() {
    let registry = LanguageRegistry::default();

    assert!(registry.get_by_extension("rs").is_some());
    assert!(registry.get_by_extension("go").is_some());
    assert!(registry.get_by_extension("py").is_some());
}

#[test]
fn registry_returns_none_for_unknown_extension() {
    let registry = LanguageRegistry::default();
    assert!(registry.get_by_extension("xyz").is_none());
}
