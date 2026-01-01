mod registry;

pub use registry::{
    CommentSyntax, Language, LanguageRegistry, LuaLongBracket, MultiLineComment, PatternKind,
    RustRawString,
};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
