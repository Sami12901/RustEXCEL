use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub const MAX_ROWS: u32 = 1_048_576;
pub const MAX_COLS: u32 = 16_384;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("Invalid row index: {0}. Row must be between 1 and {MAX_ROWS}")]
    InvalidRow(u32),
    #[error("Invalid column index: {0}. Column must be between 1 and {MAX_COLS}")]
    InvalidColumn(u32),
    #[error("Invalid column name: {0}")]
    InvalidColumnName(String),
    #[error("Invalid cell address: {0}")]
    InvalidCellAddress(String),
    #[error("Invalid cell range: {0}")]
    InvalidCellRange(String),
}

/// A 1-based cell coordinate representing (row, column).
/// Row: 1..=1,048,576
/// Column: 1..=16,384 (A..=XFD)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellCoord {
    pub row: u32,
    pub col: u32,
}

impl CellCoord {
    pub const fn new(row: u32, col: u32) -> Result<Self, CoreError> {
        if row == 0 || row > MAX_ROWS {
            return Err(CoreError::InvalidRow(row));
        }
        if col == 0 || col > MAX_COLS {
            return Err(CoreError::InvalidColumn(col));
        }
        Ok(Self { row, col })
    }

    /// Panics if out of bounds; convenient for known constants.
    pub const fn must_new(row: u32, col: u32) -> Self {
        assert!(row >= 1 && row <= MAX_ROWS, "Row out of bounds");
        assert!(col >= 1 && col <= MAX_COLS, "Col out of bounds");
        Self { row, col }
    }

    pub fn to_a1(&self) -> String {
        format!("{}{}", col_to_name(self.col), self.row)
    }

    pub fn from_a1(s: &str) -> Result<Self, CoreError> {
        let trimmed = s.trim();
        let mut col_str = String::new();
        let mut row_str = String::new();

        for ch in trimmed.chars() {
            if ch == '$' {
                continue; // Ignore absolute indicators for coordinate value
            } else if ch.is_ascii_alphabetic() {
                if !row_str.is_empty() {
                    return Err(CoreError::InvalidCellAddress(s.to_string()));
                }
                col_str.push(ch.to_ascii_uppercase());
            } else if ch.is_ascii_digit() {
                row_str.push(ch);
            } else {
                return Err(CoreError::InvalidCellAddress(s.to_string()));
            }
        }

        if col_str.is_empty() || row_str.is_empty() {
            return Err(CoreError::InvalidCellAddress(s.to_string()));
        }

        let col = name_to_col(&col_str)?;
        let row: u32 = row_str.parse().map_err(|_| CoreError::InvalidRow(0))?;

        Self::new(row, col)
    }
}

impl fmt::Display for CellCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

/// Convert 1-based column number to Excel column letters (1 -> "A", 28 -> "AB", 16384 -> "XFD").
pub fn col_to_name(mut col: u32) -> String {
    let mut result = Vec::new();
    while col > 0 {
        col -= 1;
        result.push((b'A' + (col % 26) as u8) as char);
        col /= 26;
    }
    result.into_iter().rev().collect()
}

/// Convert Excel column letters to 1-based column number ("A" -> 1, "AB" -> 28, "XFD" -> 16384).
pub fn name_to_col(name: &str) -> Result<u32, CoreError> {
    if name.is_empty() {
        return Err(CoreError::InvalidColumnName(name.to_string()));
    }
    let mut col: u32 = 0;
    for ch in name.chars() {
        if !ch.is_ascii_alphabetic() {
            return Err(CoreError::InvalidColumnName(name.to_string()));
        }
        let val = (ch.to_ascii_uppercase() as u32) - ('A' as u32) + 1;
        col = col
            .checked_mul(26)
            .and_then(|c| c.checked_add(val))
            .ok_or_else(|| CoreError::InvalidColumnName(name.to_string()))?;
    }
    if col == 0 || col > MAX_COLS {
        return Err(CoreError::InvalidColumn(col));
    }
    Ok(col)
}

/// A rectangular range bounded by top-left and bottom-right coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellRange {
    pub start: CellCoord,
    pub end: CellCoord,
}

impl CellRange {
    pub fn new(c1: CellCoord, c2: CellCoord) -> Self {
        let min_row = c1.row.min(c2.row);
        let max_row = c1.row.max(c2.row);
        let min_col = c1.col.min(c2.col);
        let max_col = c1.col.max(c2.col);
        Self {
            start: CellCoord {
                row: min_row,
                col: min_col,
            },
            end: CellCoord {
                row: max_row,
                col: max_col,
            },
        }
    }

    pub fn single_cell(coord: CellCoord) -> Self {
        Self {
            start: coord,
            end: coord,
        }
    }

    pub fn contains(&self, coord: CellCoord) -> bool {
        coord.row >= self.start.row
            && coord.row <= self.end.row
            && coord.col >= self.start.col
            && coord.col <= self.end.col
    }

    pub fn row_count(&self) -> u32 {
        self.end.row - self.start.row + 1
    }

    pub fn col_count(&self) -> u32 {
        self.end.col - self.start.col + 1
    }

    pub fn cell_count(&self) -> u64 {
        self.row_count() as u64 * self.col_count() as u64
    }

    pub fn to_a1(&self) -> String {
        if self.start == self.end {
            self.start.to_a1()
        } else {
            format!("{}:{}", self.start.to_a1(), self.end.to_a1())
        }
    }

    pub fn from_a1(s: &str) -> Result<Self, CoreError> {
        let trimmed = s.trim();
        if let Some((start_s, end_s)) = trimmed.split_once(':') {
            let start = CellCoord::from_a1(start_s)?;
            let end = CellCoord::from_a1(end_s)?;
            Ok(Self::new(start, end))
        } else {
            let coord = CellCoord::from_a1(trimmed)?;
            Ok(Self::single_cell(coord))
        }
    }
}

impl fmt::Display for CellRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

/// Identifies a specific sheet in a workbook.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SheetId(pub String);

impl SheetId {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for SheetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_col_conversion() {
        assert_eq!(col_to_name(1), "A");
        assert_eq!(col_to_name(26), "Z");
        assert_eq!(col_to_name(27), "AA");
        assert_eq!(col_to_name(28), "AB");
        assert_eq!(col_to_name(16384), "XFD");

        assert_eq!(name_to_col("A").unwrap(), 1);
        assert_eq!(name_to_col("Z").unwrap(), 26);
        assert_eq!(name_to_col("AA").unwrap(), 27);
        assert_eq!(name_to_col("XFD").unwrap(), 16384);
    }

    #[test]
    fn test_cell_coord_a1() {
        let coord = CellCoord::from_a1("C12").unwrap();
        assert_eq!(coord.col, 3);
        assert_eq!(coord.row, 12);
        assert_eq!(coord.to_a1(), "C12");

        let absolute_coord = CellCoord::from_a1("$XFD$1048576").unwrap();
        assert_eq!(absolute_coord.col, 16384);
        assert_eq!(absolute_coord.row, 1048576);
    }

    #[test]
    fn test_cell_range() {
        let range = CellRange::from_a1("A1:B10").unwrap();
        assert_eq!(range.row_count(), 10);
        assert_eq!(range.col_count(), 2);
        assert_eq!(range.cell_count(), 20);
        assert!(range.contains(CellCoord::new(5, 2).unwrap()));
        assert!(!range.contains(CellCoord::new(11, 2).unwrap()));
    }
}
