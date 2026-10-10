use serde::{Deserialize, Serialize};

/// Horizontal text alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum HorizontalAlignment {
    #[default]
    General,
    Left,
    Center,
    Right,
    Fill,
    Justify,
    CenterContinuous,
    Distributed,
}

/// Vertical text alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
    Justify,
    Distributed,
}

/// Alignment and text flow attributes for a cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
    pub wrap_text: bool,
    pub text_rotation: i16,
    pub indent: u8,
    pub shrink_to_fit: bool,
}

impl Default for Alignment {
    fn default() -> Self {
        Self {
            horizontal: HorizontalAlignment::General,
            vertical: VerticalAlignment::Bottom,
            wrap_text: false,
            text_rotation: 0,
            indent: 0,
            shrink_to_fit: false,
        }
    }
}

impl Alignment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_horizontal(mut self, horizontal: HorizontalAlignment) -> Self {
        self.horizontal = horizontal;
        self
    }

    pub fn with_vertical(mut self, vertical: VerticalAlignment) -> Self {
        self.vertical = vertical;
        self
    }

    pub fn with_wrap_text(mut self, wrap: bool) -> Self {
        self.wrap_text = wrap;
        self
    }

    pub fn with_indent(mut self, indent: u8) -> Self {
        self.indent = indent;
        self
    }

    pub fn with_rotation(mut self, degrees: i16) -> Self {
        self.text_rotation = degrees.clamp(-90, 90);
        self
    }

    pub fn center() -> Self {
        Self {
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_builders() {
        let align = Alignment::new()
            .with_horizontal(HorizontalAlignment::Right)
            .with_vertical(VerticalAlignment::Top)
            .with_wrap_text(true);
        assert_eq!(align.horizontal, HorizontalAlignment::Right);
        assert_eq!(align.vertical, VerticalAlignment::Top);
        assert!(align.wrap_text);
    }
}
