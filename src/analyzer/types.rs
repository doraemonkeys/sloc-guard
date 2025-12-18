use std::path::PathBuf;

use serde::Serialize;

/// Information about a detected function or method in a file.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,
    /// Starting line (1-indexed)
    pub start_line: usize,
    /// Ending line (1-indexed)
    pub end_line: usize,
    /// Number of code lines
    pub line_count: usize,
}

impl FunctionInfo {
    #[must_use]
    pub const fn new(name: String, start_line: usize, end_line: usize) -> Self {
        Self {
            name,
            start_line,
            end_line,
            line_count: end_line.saturating_sub(start_line) + 1,
        }
    }
}

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
