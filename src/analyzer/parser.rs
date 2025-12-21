use regex::Regex;

use super::types::FunctionInfo;

/// Trait for language-specific function detection.
pub trait FunctionParser {
    /// Parse content and extract function information.
    fn parse(&self, content: &str) -> Vec<FunctionInfo>;
}

/// Rust function parser.
pub struct RustParser {
    fn_pattern: Regex,
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            fn_pattern: Regex::new(
                r"(?m)^[\t ]*(pub(?:\s*\([^)]*\))?\s+)?(async\s+)?(unsafe\s+)?(const\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)",
            )
            .expect("Invalid regex"),
        }
    }
}

impl FunctionParser for RustParser {
    fn parse(&self, content: &str) -> Vec<FunctionInfo> {
        let lines: Vec<&str> = content.lines().collect();
        let mut functions = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = self.fn_pattern.captures(line) {
                let name = caps.get(5).map_or("", |m| m.as_str()).to_string();
                let start_line = i + 1;

                let end_line = find_block_end(&lines, i);
                functions.push(FunctionInfo::new(name, start_line, end_line));
            }
        }

        functions
    }
}

/// Go function parser.
pub struct GoParser {
    fn_pattern: Regex,
}

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GoParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            fn_pattern: Regex::new(r"(?m)^func\s+(?:\([^)]+\)\s+)?([a-zA-Z_][a-zA-Z0-9_]*)")
                .expect("Invalid regex"),
        }
    }
}

impl FunctionParser for GoParser {
    fn parse(&self, content: &str) -> Vec<FunctionInfo> {
        let lines: Vec<&str> = content.lines().collect();
        let mut functions = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = self.fn_pattern.captures(line) {
                let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
                let start_line = i + 1;
                let end_line = find_block_end(&lines, i);
                functions.push(FunctionInfo::new(name, start_line, end_line));
            }
        }

        functions
    }
}

/// Python function/class parser.
pub struct PythonParser {
    fn_pattern: Regex,
    class_pattern: Regex,
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            fn_pattern: Regex::new(r"(?m)^(\s*)(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .expect("Invalid regex"),
            class_pattern: Regex::new(r"(?m)^(\s*)class\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .expect("Invalid regex"),
        }
    }

    fn get_indent_level(line: &str) -> usize {
        line.chars().take_while(|c| c.is_whitespace()).count()
    }
}

impl FunctionParser for PythonParser {
    fn parse(&self, content: &str) -> Vec<FunctionInfo> {
        let lines: Vec<&str> = content.lines().collect();
        let mut functions = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            // Check for function definitions
            if let Some(caps) = self.fn_pattern.captures(line) {
                let indent = caps.get(1).map_or("", |m| m.as_str());
                let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
                let start_line = i + 1;
                let indent_level = Self::get_indent_level(indent);

                let end_line = find_python_block_end(&lines, i, indent_level);
                functions.push(FunctionInfo::new(name, start_line, end_line));
            }
            // Check for class definitions (top-level only)
            else if let Some(caps) = self.class_pattern.captures(line) {
                let indent = caps.get(1).map_or("", |m| m.as_str());
                if indent.is_empty() {
                    let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
                    let start_line = i + 1;
                    let end_line = find_python_block_end(&lines, i, 0);
                    functions.push(FunctionInfo::new(name, start_line, end_line));
                }
            }
        }

        // Filter to keep only top-level functions and classes
        functions.retain(|f| {
            let line_idx = f.start_line - 1;
            let line = lines.get(line_idx).unwrap_or(&"");
            Self::get_indent_level(line) == 0
        });

        functions
    }
}

/// JavaScript/TypeScript function parser.
#[allow(clippy::struct_field_names)] // Fields are named for clarity in this specific parser
pub struct JsParser {
    fn_pattern: Regex,
    arrow_pattern: Regex,
    class_pattern: Regex,
}

impl Default for JsParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JsParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            fn_pattern: Regex::new(
                r"(?m)^[\t ]*(export\s+)?(async\s+)?function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)",
            )
            .expect("Invalid regex"),
            arrow_pattern: Regex::new(
                r"(?m)^[\t ]*(export\s+)?(const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*=\s*(async\s+)?\(",
            )
            .expect("Invalid regex"),
            class_pattern: Regex::new(
                r"(?m)^[\t ]*(export\s+)?class\s+([a-zA-Z_$][a-zA-Z0-9_$]*)",
            )
            .expect("Invalid regex"),
        }
    }
}

impl FunctionParser for JsParser {
    fn parse(&self, content: &str) -> Vec<FunctionInfo> {
        let lines: Vec<&str> = content.lines().collect();
        let mut functions = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let name = self
                .fn_pattern
                .captures(line)
                .and_then(|caps| caps.get(3))
                .or_else(|| {
                    self.arrow_pattern
                        .captures(line)
                        .and_then(|caps| caps.get(3))
                })
                .or_else(|| {
                    self.class_pattern
                        .captures(line)
                        .and_then(|caps| caps.get(2))
                })
                .map(|m| m.as_str().to_string());

            if let Some(name) = name {
                let start_line = i + 1;
                let end_line = find_block_end(&lines, i);
                functions.push(FunctionInfo::new(name, start_line, end_line));
            }
        }

        functions
    }
}

/// C/C++ function parser.
pub struct CParser {
    fn_pattern: Regex,
}

impl Default for CParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CParser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            // Match function definitions: return_type name(params) {
            fn_pattern: Regex::new(
                r"(?m)^[\t ]*(?:static\s+|inline\s+|extern\s+|virtual\s+|explicit\s+)*(?:[a-zA-Z_][a-zA-Z0-9_:*&<>\s]*)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\([^)]*\)\s*(?:const\s*)?(?:noexcept\s*)?(?:override\s*)?(?:final\s*)?\{",
            )
            .expect("Invalid regex"),
        }
    }
}

impl FunctionParser for CParser {
    fn parse(&self, content: &str) -> Vec<FunctionInfo> {
        let lines: Vec<&str> = content.lines().collect();
        let mut functions = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = self.fn_pattern.captures(line) {
                let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
                // Skip common false positives
                if matches!(name.as_str(), "if" | "while" | "for" | "switch" | "catch") {
                    continue;
                }
                let start_line = i + 1;
                let end_line = find_block_end(&lines, i);
                functions.push(FunctionInfo::new(name, start_line, end_line));
            }
        }

        functions
    }
}

/// Find the end of a brace-delimited block.
fn find_block_end(lines: &[&str], start: usize) -> usize {
    let mut brace_count = 0;
    let mut found_open = false;

    for (i, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    brace_count += 1;
                    found_open = true;
                }
                '}' => {
                    brace_count -= 1;
                    if found_open && brace_count == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            }
        }
    }

    // If no matching brace found, return the last line
    lines.len()
}

/// Find the end of a Python indentation-based block.
fn find_python_block_end(lines: &[&str], start: usize, base_indent: usize) -> usize {
    let mut end_line = start + 1;

    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = line.trim();
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            end_line = i + 1;
            continue;
        }

        let current_indent = line.chars().take_while(|c| c.is_whitespace()).count();
        if current_indent <= base_indent && !trimmed.is_empty() {
            break;
        }
        end_line = i + 1;
    }

    end_line
}

/// Get a parser for the given language.
#[must_use]
pub fn get_parser(language: &str) -> Option<Box<dyn FunctionParser>> {
    match language.to_lowercase().as_str() {
        "rust" => Some(Box::new(RustParser::new())),
        "go" => Some(Box::new(GoParser::new())),
        "python" => Some(Box::new(PythonParser::new())),
        "javascript" | "typescript" | "jsx" | "tsx" => Some(Box::new(JsParser::new())),
        "c" | "c++" | "cpp" => Some(Box::new(CParser::new())),
        _ => None,
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rust_parser_tests.rs"]
mod rust_tests;

#[cfg(test)]
#[path = "go_parser_tests.rs"]
mod go_tests;

#[cfg(test)]
#[path = "python_parser_tests.rs"]
mod python_tests;

#[cfg(test)]
#[path = "js_parser_tests.rs"]
mod js_tests;

#[cfg(test)]
#[path = "c_parser_tests.rs"]
mod c_tests;
