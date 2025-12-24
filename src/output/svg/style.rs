//! SVG styling primitives: colors and text anchoring.

use std::fmt;

/// Color specification supporting CSS variables for dark mode.
#[derive(Debug, Clone)]
pub enum ChartColor {
    /// Direct hex color (e.g., "#22c55e")
    Hex(String),
    /// CSS variable reference (e.g., "passed" → "var(--color-passed)")
    CssVar(String),
}

impl ChartColor {
    /// Create a CSS variable color reference.
    #[must_use]
    pub fn css_var(name: &str) -> Self {
        Self::CssVar(name.to_string())
    }

    /// Create a hex color.
    #[must_use]
    pub fn hex(color: &str) -> Self {
        Self::Hex(color.to_string())
    }

    /// Convert to CSS value string.
    #[must_use]
    pub fn to_css(&self) -> String {
        match self {
            Self::Hex(h) => h.clone(),
            Self::CssVar(name) => format!("var(--color-{name})"),
        }
    }
}

/// Text anchor position for labels.
#[derive(Debug, Clone, Copy, Default)]
pub enum TextAnchor {
    #[default]
    Start,
    Middle,
    End,
}

impl fmt::Display for TextAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Middle => write!(f, "middle"),
            Self::End => write!(f, "end"),
        }
    }
}

#[cfg(test)]
#[path = "style_tests.rs"]
mod tests;
