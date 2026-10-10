use chrono::NaiveDateTime;
pub use exel_core::{CellCoord, CellRange, SheetId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Standard Excel formula error representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellError {
    /// Division by zero: #DIV/0!
    Div0,
    /// Invalid operand or type mismatch: #VALUE!
    Value,
    /// Invalid cell reference: #REF!
    Ref,
    /// Unrecognized formula or function name: #NAME?
    Name,
    /// Value not available: #N/A
    NA,
    /// Number outside calculation domain: #NUM!
    Num,
    /// Intersection of two ranges that do not intersect: #NULL!
    Null,
    /// Dynamic array cannot expand: #SPILL!
    Spill,
    /// Circular dependency detected: #CIRCULAR!
    Circular,
}

impl CellError {
    pub fn as_str(&self) -> &'static str {
        match self {
            CellError::Div0 => "#DIV/0!",
            CellError::Value => "#VALUE!",
            CellError::Ref => "#REF!",
            CellError::Name => "#NAME?",
            CellError::NA => "#N/A",
            CellError::Num => "#NUM!",
            CellError::Null => "#NULL!",
            CellError::Spill => "#SPILL!",
            CellError::Circular => "#CIRCULAR!",
        }
    }
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The evaluated value of a cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CellValue {
    Empty,
    Number(f64),
    Text(String),
    Boolean(bool),
    DateTime(NaiveDateTime),
    Error(CellError),
}

impl CellValue {
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }

    pub fn to_display_string(&self) -> String {
        match self {
            CellValue::Empty => String::new(),
            CellValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{:.0}", n)
                } else {
                    format!("{:.4}", n)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string()
                }
            }
            CellValue::Text(s) => s.clone(),
            CellValue::Boolean(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            CellValue::DateTime(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            CellValue::Error(err) => err.as_str().to_string(),
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            CellValue::Number(n) => Some(*n),
            CellValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            CellValue::Text(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

/// The full state of a single cell, including its formula, evaluated value, style, and comments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellState {
    pub value: CellValue,
    pub formula: Option<String>,
    pub style_id: u32,
    pub comment: Option<String>,
}

impl Default for CellState {
    fn default() -> Self {
        Self {
            value: CellValue::Empty,
            formula: None,
            style_id: 0,
            comment: None,
        }
    }
}

impl CellState {
    pub fn from_value(value: CellValue) -> Self {
        Self {
            value,
            formula: None,
            style_id: 0,
            comment: None,
        }
    }

    pub fn from_formula(formula: String, cached_value: CellValue) -> Self {
        Self {
            value: cached_value,
            formula: Some(formula),
            style_id: 0,
            comment: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
            && self.formula.is_none()
            && self.style_id == 0
            && self.comment.is_none()
    }
}

/// High-performance sparse storage for spreadsheet cells.
/// Uses a BTreeMap keyed by CellCoord to avoid allocating millions of empty cells.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SparseCellStore {
    cells: BTreeMap<CellCoord, CellState>,
}

impl SparseCellStore {
    pub fn new() -> Self {
        Self {
            cells: BTreeMap::new(),
        }
    }

    pub fn get(&self, coord: CellCoord) -> Option<&CellState> {
        self.cells.get(&coord)
    }

    pub fn get_mut(&mut self, coord: CellCoord) -> Option<&mut CellState> {
        self.cells.get_mut(&coord)
    }

    pub fn set(&mut self, coord: CellCoord, state: CellState) {
        if state.is_empty() {
            self.cells.remove(&coord);
        } else {
            self.cells.insert(coord, state);
        }
    }

    pub fn remove(&mut self, coord: CellCoord) -> Option<CellState> {
        self.cells.remove(&coord)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CellCoord, &CellState)> {
        self.cells.iter()
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// Computes the bounding rectangle encompassing all populated cells.
    pub fn used_range(&self) -> Option<CellRange> {
        if self.cells.is_empty() {
            return None;
        }

        let mut min_row = u32::MAX;
        let mut max_row = 1;
        let mut min_col = u32::MAX;
        let mut max_col = 1;

        for coord in self.cells.keys() {
            min_row = min_row.min(coord.row);
            max_row = max_row.max(coord.row);
            min_col = min_col.min(coord.col);
            max_col = max_col.max(coord.col);
        }

        Some(CellRange::new(
            CellCoord::must_new(min_row, min_col),
            CellCoord::must_new(max_row, max_col),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_store() {
        let mut store = SparseCellStore::new();
        assert_eq!(store.len(), 0);
        assert_eq!(store.used_range(), None);

        let c1 = CellCoord::must_new(1, 1);
        let c2 = CellCoord::must_new(100, 50);

        store.set(c1, CellState::from_value(CellValue::Number(42.0)));
        store.set(c2, CellState::from_value(CellValue::Text("Hello".into())));

        assert_eq!(store.len(), 2);
        let range = store.used_range().unwrap();
        assert_eq!(range.start, c1);
        assert_eq!(range.end, c2);

        assert_eq!(store.get(c1).unwrap().value, CellValue::Number(42.0));
    }
}
