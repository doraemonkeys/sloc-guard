mod registry;

pub use registry::{CommentSyntax, Language, LanguageRegistry};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
