use crate::style::CellStyle;
use serde::{Deserialize, Serialize};

/// Shared interning table for workbook cell styles.
/// Index 0 is permanently reserved for the default spreadsheet style.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleTable {
    styles: Vec<CellStyle>,
}

impl Default for StyleTable {
    fn default() -> Self {
        Self {
            styles: vec![CellStyle::default()],
        }
    }
}

impl StyleTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieve style by its unique 0-based index.
    pub fn get(&self, id: u32) -> Option<&CellStyle> {
        self.styles.get(id as usize)
    }

    /// Retrieve style or fallback to default style at index 0.
    pub fn get_or_default(&self, id: u32) -> &CellStyle {
        self.get(id).unwrap_or(&self.styles[0])
    }

    /// Find an existing matching style or insert a new one, returning its unique style ID.
    pub fn get_or_insert(&mut self, style: CellStyle) -> u32 {
        if let Some(pos) = self.styles.iter().position(|s| s == &style) {
            pos as u32
        } else {
            let id = self.styles.len() as u32;
            self.styles.push(style);
            id
        }
    }

    /// Unconditionally insert a style, returning its assigned ID.
    pub fn insert(&mut self, style: CellStyle) -> u32 {
        let id = self.styles.len() as u32;
        self.styles.push(style);
        id
    }

    pub fn len(&self) -> usize {
        self.styles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &CellStyle)> {
        self.styles.iter().enumerate().map(|(i, s)| (i as u32, s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::font::Font;

    #[test]
    fn test_style_table_interning() {
        let mut table = StyleTable::new();
        assert_eq!(table.len(), 1); // Has default at index 0

        let bold_style = CellStyle::default().with_font(Font::default().with_bold(true));
        let id1 = table.get_or_insert(bold_style.clone());
        assert_eq!(id1, 1);
        assert_eq!(table.len(), 2);

        // Interning identical style returns existing ID
        let id2 = table.get_or_insert(bold_style);
        assert_eq!(id2, 1);
        assert_eq!(table.len(), 2);

        // Different style gets new ID
        let red_style = CellStyle::default().with_font(Font::default().with_color(Color::RED));
        let id3 = table.get_or_insert(red_style);
        assert_eq!(id3, 2);
        assert_eq!(table.len(), 3);
    }
}
