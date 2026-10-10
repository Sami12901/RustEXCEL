use crate::color::Color;
use serde::{Deserialize, Serialize};

/// Border line styles matching OpenXML / Excel border specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum BorderStyle {
    #[default]
    None,
    Thin,
    Medium,
    Dashed,
    Dotted,
    Thick,
    Double,
    Hair,
    MediumDashed,
    DashDot,
    MediumDashDot,
    DashDotDot,
    MediumDashDotDot,
    SlantedDashDot,
}

impl BorderStyle {
    pub fn is_none(&self) -> bool {
        matches!(self, BorderStyle::None)
    }
}

/// A single side border specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BorderSide {
    pub style: BorderStyle,
    pub color: Color,
}

impl Default for BorderSide {
    fn default() -> Self {
        Self {
            style: BorderStyle::None,
            color: Color::BLACK,
        }
    }
}

impl BorderSide {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn new(style: BorderStyle, color: Color) -> Self {
        Self { style, color }
    }

    pub fn thin(color: Color) -> Self {
        Self::new(BorderStyle::Thin, color)
    }

    pub fn medium(color: Color) -> Self {
        Self::new(BorderStyle::Medium, color)
    }

    pub fn thick(color: Color) -> Self {
        Self::new(BorderStyle::Thick, color)
    }

    pub fn double(color: Color) -> Self {
        Self::new(BorderStyle::Double, color)
    }

    pub fn is_none(&self) -> bool {
        self.style.is_none()
    }
}

/// Full border settings for all edges of a cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Border {
    pub top: BorderSide,
    pub bottom: BorderSide,
    pub left: BorderSide,
    pub right: BorderSide,
    pub diagonal_up: BorderSide,
    pub diagonal_down: BorderSide,
}

impl Border {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn all(style: BorderStyle, color: Color) -> Self {
        let side = BorderSide::new(style, color);
        Self {
            top: side,
            bottom: side,
            left: side,
            right: side,
            diagonal_up: BorderSide::none(),
            diagonal_down: BorderSide::none(),
        }
    }

    pub fn outline(style: BorderStyle, color: Color) -> Self {
        Self::all(style, color)
    }

    pub fn bottom_only(style: BorderStyle, color: Color) -> Self {
        Self {
            bottom: BorderSide::new(style, color),
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.top.is_none()
            && self.bottom.is_none()
            && self.left.is_none()
            && self.right.is_none()
            && self.diagonal_up.is_none()
            && self.diagonal_down.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_construction() {
        let empty = Border::none();
        assert!(empty.is_empty());

        let thin_box = Border::all(BorderStyle::Thin, Color::BLACK);
        assert_eq!(thin_box.top.style, BorderStyle::Thin);
        assert_eq!(thin_box.bottom.style, BorderStyle::Thin);
        assert_eq!(thin_box.left.style, BorderStyle::Thin);
        assert_eq!(thin_box.right.style, BorderStyle::Thin);
        assert!(!thin_box.is_empty());
    }
}
