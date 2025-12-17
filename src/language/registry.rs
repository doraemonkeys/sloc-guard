use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentSyntax {
    pub single_line: Vec<&'static str>,
    pub multi_line: Vec<(&'static str, &'static str)>,
}

impl CommentSyntax {
    #[must_use]
    pub const fn new(
        single_line: Vec<&'static str>,
        multi_line: Vec<(&'static str, &'static str)>,
    ) -> Self {
        Self {
            single_line,
            multi_line,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub name: &'static str,
    pub extensions: Vec<&'static str>,
    pub comment_syntax: CommentSyntax,
}

impl Language {
    #[must_use]
    pub const fn new(
        name: &'static str,
        extensions: Vec<&'static str>,
        comment_syntax: CommentSyntax,
    ) -> Self {
        Self {
            name,
            extensions,
            comment_syntax,
        }
    }
}

#[derive(Debug)]
pub struct LanguageRegistry {
    languages: Vec<Language>,
    extension_map: HashMap<&'static str, usize>,
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
            self.extension_map.insert(ext, idx);
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
