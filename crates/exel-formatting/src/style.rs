use crate::alignment::Alignment;
use crate::border::Border;
use crate::fill::Fill;
use crate::font::Font;
use crate::number_format::NumberFormat;
use serde::{Deserialize, Serialize};

/// Comprehensive styling descriptor for a cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellStyle {
    pub font: Font,
    pub fill: Fill,
    pub border: Border,
    pub alignment: Alignment,
    pub number_format: NumberFormat,
    pub locked: bool,
    pub hidden: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            font: Font::default(),
            fill: Fill::default(),
            border: Border::default(),
            alignment: Alignment::default(),
            number_format: NumberFormat::General,
            locked: true,
            hidden: false,
        }
    }
}

impl CellStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    pub fn with_fill(mut self, fill: Fill) -> Self {
        self.fill = fill;
        self
    }

    pub fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_number_format(mut self, number_format: NumberFormat) -> Self {
        self.number_format = number_format;
        self
    }

    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_style_default() {
        let style = CellStyle::default();
        assert_eq!(style.font.name, "Calibri");
        assert_eq!(style.number_format, NumberFormat::General);
        assert!(style.locked);
        assert!(!style.hidden);
    }
}
