use crate::color::Color;
use serde::{Deserialize, Serialize};

/// Typography and text styling options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Font {
    pub name: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Color,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            name: "Calibri".to_string(),
            size: 11.0,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: Color::BLACK,
        }
    }
}

impl Font {
    pub fn new<S: Into<String>>(name: S, size: f32) -> Self {
        Self {
            name: name.into(),
            size,
            ..Default::default()
        }
    }

    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    pub fn with_strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = strikethrough;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_builder() {
        let font = Font::new("Arial", 12.0)
            .with_bold(true)
            .with_italic(true)
            .with_color(Color::RED);
        assert_eq!(font.name, "Arial");
        assert_eq!(font.size, 12.0);
        assert!(font.bold);
        assert!(font.italic);
        assert_eq!(font.color, Color::RED);
    }
}
