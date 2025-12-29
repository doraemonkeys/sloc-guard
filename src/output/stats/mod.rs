mod html;
mod json;
mod markdown;
mod statistics;
mod text;

pub use html::StatsHtmlFormatter;
pub use json::StatsJsonFormatter;
pub use markdown::StatsMarkdownFormatter;
pub use statistics::{
    DirectoryStats, FileSortOrder, FileStatistics, LanguageStats, ProjectStatistics,
};
pub use text::StatsTextFormatter;

use crate::error::Result;

pub trait StatsFormatter {
    /// Format the project statistics into a string.
    ///
    /// # Errors
    /// Returns an error if the formatting fails.
    fn format(&self, stats: &ProjectStatistics) -> Result<String>;
}
