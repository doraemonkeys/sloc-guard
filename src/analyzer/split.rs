use std::path::{Path, PathBuf};

use serde::Serialize;

use super::parser::{FunctionInfo, get_parser};

/// A suggestion for splitting a file.
#[derive(Debug, Clone, Serialize)]
pub struct SplitChunk {
    /// Suggested name for the new file (without extension)
    pub suggested_name: String,
    /// Functions to move to this file
    pub functions: Vec<String>,
    /// Starting line of the chunk (1-indexed)
    pub start_line: usize,
    /// Ending line of the chunk (1-indexed)
    pub end_line: usize,
    /// Estimated line count
    pub line_count: usize,
}

/// Split suggestions for a file that exceeds its limit.
#[derive(Debug, Clone, Serialize)]
pub struct SplitSuggestion {
    /// Original file path
    pub original_path: PathBuf,
    /// Total lines in the original file
    pub total_lines: usize,
    /// Current limit for the file
    pub limit: usize,
    /// List of detected functions
    pub functions: Vec<FunctionInfo>,
    /// Suggested split chunks
    pub chunks: Vec<SplitChunk>,
}

impl SplitSuggestion {
    #[must_use]
    pub const fn new(original_path: PathBuf, total_lines: usize, limit: usize) -> Self {
        Self {
            original_path,
            total_lines,
            limit,
            functions: Vec::new(),
            chunks: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_functions(mut self, functions: Vec<FunctionInfo>) -> Self {
        self.functions = functions;
        self
    }

    #[must_use]
    pub fn with_chunks(mut self, chunks: Vec<SplitChunk>) -> Self {
        self.chunks = chunks;
        self
    }

    /// Check if any suggestions were generated.
    #[must_use]
    pub const fn has_suggestions(&self) -> bool {
        !self.chunks.is_empty()
    }
}

/// Analyze a file and generate split suggestions.
pub struct SplitAnalyzer {
    target_size: usize,
}

impl SplitAnalyzer {
    /// Create a new analyzer with the target size for each chunk.
    #[must_use]
    pub const fn new(target_size: usize) -> Self {
        Self { target_size }
    }

    /// Analyze a file and generate split suggestions.
    #[must_use]
    pub fn analyze(
        &self,
        path: &Path,
        content: &str,
        language: &str,
        limit: usize,
    ) -> Option<SplitSuggestion> {
        let parser = get_parser(language)?;
        let functions = parser.parse(content);

        if functions.is_empty() {
            return None;
        }

        let total_lines = content.lines().count();
        let mut suggestion = SplitSuggestion::new(path.to_path_buf(), total_lines, limit);
        suggestion = suggestion.with_functions(functions.clone());

        let chunks = self.generate_chunks(path, &functions, limit);
        if chunks.is_empty() {
            return None;
        }

        suggestion = suggestion.with_chunks(chunks);
        Some(suggestion)
    }

    fn generate_chunks(
        &self,
        path: &Path,
        functions: &[FunctionInfo],
        limit: usize,
    ) -> Vec<SplitChunk> {
        if functions.is_empty() {
            return Vec::new();
        }

        let base_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");

        let mut chunks = Vec::new();
        let mut current_chunk_funcs: Vec<&FunctionInfo> = Vec::new();
        let mut current_lines = 0;
        let mut chunk_index = 1;

        for func in functions {
            // If adding this function would exceed target size and we have something, start a new chunk
            if current_lines + func.line_count > self.target_size && !current_chunk_funcs.is_empty()
            {
                chunks.push(Self::create_chunk(
                    base_name,
                    chunk_index,
                    &current_chunk_funcs,
                ));
                chunk_index += 1;
                current_chunk_funcs.clear();
                current_lines = 0;
            }

            current_chunk_funcs.push(func);
            current_lines += func.line_count;

            // If single function exceeds limit, it gets its own chunk
            if func.line_count > limit {
                chunks.push(Self::create_chunk(
                    base_name,
                    chunk_index,
                    &current_chunk_funcs,
                ));
                chunk_index += 1;
                current_chunk_funcs.clear();
                current_lines = 0;
            }
        }

        // Handle remaining functions
        if !current_chunk_funcs.is_empty() {
            chunks.push(Self::create_chunk(
                base_name,
                chunk_index,
                &current_chunk_funcs,
            ));
        }

        // Only return chunks if we would actually split the file (more than 1 chunk)
        if chunks.len() > 1 { chunks } else { Vec::new() }
    }

    fn create_chunk(base_name: &str, index: usize, functions: &[&FunctionInfo]) -> SplitChunk {
        let func_names: Vec<String> = functions.iter().map(|f| f.name.clone()).collect();
        let start_line = functions.first().map_or(1, |f| f.start_line);
        let end_line = functions.last().map_or(1, |f| f.end_line);
        let line_count: usize = functions.iter().map(|f| f.line_count).sum();

        // Generate a suggested name based on the functions
        let suggested_name = if functions.len() == 1 {
            format!("{}_{}", base_name, functions[0].name.to_lowercase())
        } else {
            format!("{base_name}_part{index}")
        };

        SplitChunk {
            suggested_name,
            functions: func_names,
            start_line,
            end_line,
            line_count,
        }
    }
}

impl Default for SplitAnalyzer {
    fn default() -> Self {
        Self::new(300)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_analyzer_no_functions() {
        let analyzer = SplitAnalyzer::new(100);
        let result = analyzer.analyze(Path::new("test.rs"), "// just a comment", "rust", 500);
        assert!(result.is_none());
    }

    #[test]
    fn test_split_analyzer_generates_chunks() {
        let content = r"
fn func1() {
    // line 1
    // line 2
    // line 3
}

fn func2() {
    // line 1
    // line 2
    // line 3
}

fn func3() {
    // line 1
    // line 2
    // line 3
}
";
        let analyzer = SplitAnalyzer::new(5);
        let result = analyzer.analyze(Path::new("test.rs"), content, "rust", 10);

        assert!(result.is_some());
        let suggestion = result.unwrap();
        assert!(!suggestion.chunks.is_empty());
        assert_eq!(suggestion.functions.len(), 3);
    }

    #[test]
    fn test_split_analyzer_single_chunk_no_suggestion() {
        let content = r"
fn func1() {
    // small
}
";
        let analyzer = SplitAnalyzer::new(100);
        let result = analyzer.analyze(Path::new("test.rs"), content, "rust", 500);

        // Should return None or empty chunks since we can't meaningfully split
        assert!(result.is_none() || !result.unwrap().has_suggestions());
    }
}
