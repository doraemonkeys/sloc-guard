use super::*;

#[test]
fn config_deserialize_custom_language() {
    let toml_str = r#"
        version = "2"

        [languages.haskell]
        extensions = ["hs", "lhs"]
        single_line_comments = ["--"]
        multi_line_comments = [["{-", "-}"]]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.languages.contains_key("haskell"));
    let haskell = &config.languages["haskell"];
    assert_eq!(haskell.extensions, vec!["hs", "lhs"]);
    assert_eq!(haskell.single_line_comments, vec!["--"]);
    assert_eq!(
        haskell.multi_line_comments,
        vec![("{-".to_string(), "-}".to_string())]
    );
}

#[test]
fn config_deserialize_multiple_custom_languages() {
    let toml_str = r#"
        version = "2"

        [languages.haskell]
        extensions = ["hs"]
        single_line_comments = ["--"]
        multi_line_comments = [["{-", "-}"]]

        [languages.lua]
        extensions = ["lua"]
        single_line_comments = ["--"]
        multi_line_comments = [["--[[", "]]"]]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.languages.len(), 2);
    assert!(config.languages.contains_key("haskell"));
    assert!(config.languages.contains_key("lua"));
}

#[test]
fn custom_language_config_default_values() {
    let config = CustomLanguageConfig::default();
    assert!(config.extensions.is_empty());
    assert!(config.single_line_comments.is_empty());
    assert!(config.multi_line_comments.is_empty());
}

#[test]
fn config_empty_languages_by_default() {
    let config = Config::default();
    assert!(config.languages.is_empty());
}
