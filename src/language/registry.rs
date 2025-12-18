use std::collections::HashMap;

use crate::config::CustomLanguageConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentSyntax {
    pub single_line: Vec<String>,
    pub multi_line: Vec<(String, String)>,
}

impl CommentSyntax {
    #[must_use]
    pub fn new(single_line: Vec<&str>, multi_line: Vec<(&str, &str)>) -> Self {
        Self {
            single_line: single_line.into_iter().map(String::from).collect(),
            multi_line: multi_line
                .into_iter()
                .map(|(s, e)| (s.to_string(), e.to_string()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub name: String,
    pub extensions: Vec<String>,
    pub comment_syntax: CommentSyntax,
}

impl Language {
    #[must_use]
    pub fn new(name: &str, extensions: Vec<&str>, comment_syntax: CommentSyntax) -> Self {
        Self {
            name: name.to_string(),
            extensions: extensions.into_iter().map(String::from).collect(),
            comment_syntax,
        }
    }
}

#[derive(Debug)]
pub struct LanguageRegistry {
    languages: Vec<Language>,
    extension_map: HashMap<String, usize>,
}

impl LanguageRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            languages: Vec::new(),
            extension_map: HashMap::new(),
        }
    }

    pub fn register(&mut self, language: Language) {
        let idx = self.languages.len();
        for ext in &language.extensions {
            self.extension_map.insert(ext.clone(), idx);
        }
        self.languages.push(language);
    }

    #[must_use]
    pub fn get_by_extension(&self, ext: &str) -> Option<&Language> {
        self.extension_map
            .get(ext)
            .map(|&idx| &self.languages[idx])
    }

    #[must_use]
    pub fn all(&self) -> &[Language] {
        &self.languages
    }

    #[must_use]
    pub fn with_custom_languages(custom: &HashMap<String, CustomLanguageConfig>) -> Self {
        let mut registry = Self::default();

        for (name, config) in custom {
            let syntax = CommentSyntax {
                single_line: config.single_line_comments.clone(),
                multi_line: config.multi_line_comments.clone(),
            };
            let language = Language {
                name: name.clone(),
                extensions: config.extensions.clone(),
                comment_syntax: syntax,
            };
            registry.register(language);
        }

        registry
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        registry.register(Language::new(
            "Rust",
            vec!["rs"],
            CommentSyntax::new(vec!["//", "///", "//!"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "Go",
            vec!["go"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "Python",
            vec!["py", "pyi"],
            CommentSyntax::new(vec!["#"], vec![("'''", "'''"), ("\"\"\"", "\"\"\"")]),
        ));

        registry.register(Language::new(
            "JavaScript",
            vec!["js", "mjs", "cjs"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "TypeScript",
            vec!["ts", "mts", "cts", "tsx"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "C",
            vec!["c", "h"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "C++",
            vec!["cpp", "hpp", "cc", "cxx", "hxx"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
