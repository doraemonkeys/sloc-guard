//! SVG composition builder for custom chart layouts.

use std::fmt::Write;

use super::element::SvgElement;
use super::format::html_escape;

/// Builder for custom SVG compositions.
#[derive(Debug, Default)]
pub struct SvgBuilder {
    width: f64,
    height: f64,
    title: String,
    elements: Vec<String>,
}

impl SvgBuilder {
    #[must_use]
    pub const fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            title: String::new(),
            elements: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    #[must_use]
    pub fn push_element<E: SvgElement>(mut self, element: &E) -> Self {
        self.elements.push(element.render());
        self
    }

    #[must_use]
    pub fn push_raw(mut self, svg: impl Into<String>) -> Self {
        self.elements.push(svg.into());
        self
    }

    #[must_use]
    pub fn build(self) -> String {
        let mut output = String::new();

        let _ = writeln!(
            output,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" role="img">"#,
            self.width, self.height
        );

        if !self.title.is_empty() {
            let escaped = html_escape(&self.title);
            let _ = writeln!(output, r"    <title>{escaped}</title>");
        }

        for element in self.elements {
            for line in element.lines() {
                let _ = writeln!(output, "    {line}");
            }
        }

        output.push_str("</svg>");
        output
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
