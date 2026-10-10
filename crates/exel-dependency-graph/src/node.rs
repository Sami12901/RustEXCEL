use exel_core::CellCoord;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A node in the dependency graph, identifying a cell either locally or with sheet qualification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellNode {
    /// Optional sheet name. None indicates current/active sheet context.
    pub sheet: Option<String>,
    /// The cell coordinates (1-based row and column).
    pub coord: CellCoord,
}

impl CellNode {
    /// Creates a new cell node without a sheet name.
    pub const fn new(coord: CellCoord) -> Self {
        Self { sheet: None, coord }
    }

    /// Creates a new cell node with a specific sheet name.
    pub fn with_sheet<S: Into<String>>(sheet: S, coord: CellCoord) -> Self {
        Self {
            sheet: Some(sheet.into()),
            coord,
        }
    }

    /// Returns a string representation (e.g., "A1" or "'Sheet1'!A1").
    pub fn to_a1(&self) -> String {
        match &self.sheet {
            Some(sheet) => {
                if sheet.contains(' ') || sheet.contains('\'') {
                    format!("'{}'!{}", sheet.replace('\'', "''"), self.coord.to_a1())
                } else {
                    format!("{}!{}", sheet, self.coord.to_a1())
                }
            }
            None => self.coord.to_a1(),
        }
    }
}

impl From<CellCoord> for CellNode {
    fn from(coord: CellCoord) -> Self {
        Self::new(coord)
    }
}

impl fmt::Display for CellNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}
