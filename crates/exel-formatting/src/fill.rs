use crate::color::Color;
use serde::{Deserialize, Serialize};

/// Predefined fill pattern types matching standard spreadsheet patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FillPattern {
    #[default]
    None,
    Solid,
    MediumGray,
    DarkGray,
    LightGray,
    DarkHorizontal,
    DarkVertical,
    DarkDown,
    DarkUp,
    DarkGrid,
    DarkTrellis,
    LightHorizontal,
    LightVertical,
    LightDown,
    LightUp,
    LightGrid,
    LightTrellis,
    Gray125,
    Gray0625,
}

/// Cell background fill definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fill {
    pub pattern: FillPattern,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

impl Default for Fill {
    fn default() -> Self {
        Self {
            pattern: FillPattern::None,
            fg_color: None,
            bg_color: None,
        }
    }
}

impl Fill {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn solid(color: Color) -> Self {
        Self {
            pattern: FillPattern::Solid,
            fg_color: Some(color),
            bg_color: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pattern == FillPattern::None && self.fg_color.is_none() && self.bg_color.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_constructors() {
        let empty = Fill::none();
        assert!(empty.is_empty());

        let solid_blue = Fill::solid(Color::BLUE);
        assert_eq!(solid_blue.pattern, FillPattern::Solid);
        assert_eq!(solid_blue.fg_color, Some(Color::BLUE));
        assert!(!solid_blue.is_empty());
    }
}
