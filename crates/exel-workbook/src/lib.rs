use exel_cell::{CellCoord, CellState, CellValue, SparseCellStore};
use exel_core::{CoreError, SheetId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum WorkbookError {
    #[error("Sheet not found: {0}")]
    SheetNotFound(String),
    #[error("Sheet name already exists: {0}")]
    DuplicateSheetName(String),
    #[error("Cannot remove the only remaining worksheet")]
    CannotRemoveOnlySheet,
    #[error("Invalid sheet name: {0}")]
    InvalidSheetName(String),
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// A single worksheet containing sparse cell data, dimensions, and view state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Worksheet {
    pub id: SheetId,
    pub name: String,
    pub cells: SparseCellStore,
    pub col_widths: HashMap<u32, f32>,
    pub row_heights: HashMap<u32, f32>,
    pub hidden_rows: HashSet<u32>,
    pub hidden_cols: HashSet<u32>,
    pub freeze_panes: Option<CellCoord>,
    pub tab_color: Option<[u8; 3]>,
}

impl Worksheet {
    pub const DEFAULT_COL_WIDTH: f32 = 80.0;
    pub const DEFAULT_ROW_HEIGHT: f32 = 22.0;

    pub fn new<S: Into<String>>(id: SheetId, name: S) -> Self {
        Self {
            id,
            name: name.into(),
            cells: SparseCellStore::new(),
            col_widths: HashMap::new(),
            row_heights: HashMap::new(),
            hidden_rows: HashSet::new(),
            hidden_cols: HashSet::new(),
            freeze_panes: None,
            tab_color: None,
        }
    }

    pub fn col_width(&self, col: u32) -> f32 {
        self.col_widths
            .get(&col)
            .copied()
            .unwrap_or(Self::DEFAULT_COL_WIDTH)
    }

    pub fn set_col_width(&mut self, col: u32, width: f32) {
        self.col_widths.insert(col, width.max(20.0));
    }

    pub fn row_height(&self, row: u32) -> f32 {
        self.row_heights
            .get(&row)
            .copied()
            .unwrap_or(Self::DEFAULT_ROW_HEIGHT)
    }

    pub fn set_row_height(&mut self, row: u32, height: f32) {
        self.row_heights.insert(row, height.max(14.0));
    }

    pub fn get_cell(&self, coord: CellCoord) -> Option<&CellState> {
        self.cells.get(coord)
    }

    pub fn get_cell_value(&self, coord: CellCoord) -> CellValue {
        self.cells
            .get(coord)
            .map(|c| c.value.clone())
            .unwrap_or(CellValue::Empty)
    }

    pub fn set_cell(&mut self, coord: CellCoord, state: CellState) {
        self.cells.set(coord, state);
    }

    pub fn set_cell_value(&mut self, coord: CellCoord, value: CellValue) {
        let mut state = self.cells.get(coord).cloned().unwrap_or_default();
        state.value = value;
        state.formula = None;
        self.cells.set(coord, state);
    }

    pub fn set_cell_formula(&mut self, coord: CellCoord, formula: String, cached_value: CellValue) {
        let mut state = self.cells.get(coord).cloned().unwrap_or_default();
        state.formula = Some(formula);
        state.value = cached_value;
        self.cells.set(coord, state);
    }
}

/// A multi-sheet workbook document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workbook {
    pub sheets: Vec<Worksheet>,
    pub active_sheet_index: usize,
    pub named_ranges: HashMap<String, String>,
}

impl Default for Workbook {
    fn default() -> Self {
        Self::with_default_sheet()
    }
}

impl Workbook {
    pub fn new() -> Self {
        Self {
            sheets: Vec::new(),
            active_sheet_index: 0,
            named_ranges: HashMap::new(),
        }
    }

    pub fn with_default_sheet() -> Self {
        let mut wb = Self::new();
        wb.add_sheet("Sheet1")
            .expect("Default sheet creation cannot fail");
        wb
    }

    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    pub fn active_sheet(&self) -> &Worksheet {
        &self.sheets[self.active_sheet_index]
    }

    pub fn active_sheet_mut(&mut self) -> &mut Worksheet {
        &mut self.sheets[self.active_sheet_index]
    }

    pub fn set_active_sheet_index(&mut self, index: usize) -> Result<(), WorkbookError> {
        if index < self.sheets.len() {
            self.active_sheet_index = index;
            Ok(())
        } else {
            Err(WorkbookError::SheetNotFound(format!("Index {}", index)))
        }
    }

    pub fn add_sheet(&mut self, name: &str) -> Result<SheetId, WorkbookError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(WorkbookError::InvalidSheetName(name.to_string()));
        }
        if self
            .sheets
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(trimmed))
        {
            return Err(WorkbookError::DuplicateSheetName(trimmed.to_string()));
        }

        let id = SheetId::new(format!("sheet_{}", self.sheets.len() + 1));
        let worksheet = Worksheet::new(id.clone(), trimmed);
        self.sheets.push(worksheet);
        Ok(id)
    }

    pub fn rename_sheet(&mut self, id: &SheetId, new_name: &str) -> Result<(), WorkbookError> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return Err(WorkbookError::InvalidSheetName(new_name.to_string()));
        }
        if self
            .sheets
            .iter()
            .any(|s| &s.id != id && s.name.eq_ignore_ascii_case(trimmed))
        {
            return Err(WorkbookError::DuplicateSheetName(trimmed.to_string()));
        }

        let sheet = self
            .sheets
            .iter_mut()
            .find(|s| &s.id == id)
            .ok_or_else(|| WorkbookError::SheetNotFound(id.to_string()))?;
        sheet.name = trimmed.to_string();
        Ok(())
    }

    pub fn remove_sheet(&mut self, id: &SheetId) -> Result<(), WorkbookError> {
        if self.sheets.len() <= 1 {
            return Err(WorkbookError::CannotRemoveOnlySheet);
        }

        let pos = self
            .sheets
            .iter()
            .position(|s| &s.id == id)
            .ok_or_else(|| WorkbookError::SheetNotFound(id.to_string()))?;

        self.sheets.remove(pos);
        if self.active_sheet_index >= self.sheets.len() {
            self.active_sheet_index = self.sheets.len() - 1;
        }
        Ok(())
    }

    pub fn get_sheet(&self, id: &SheetId) -> Option<&Worksheet> {
        self.sheets.iter().find(|s| &s.id == id)
    }

    pub fn get_sheet_mut(&mut self, id: &SheetId) -> Option<&mut Worksheet> {
        self.sheets.iter_mut().find(|s| &s.id == id)
    }

    pub fn get_sheet_by_name(&self, name: &str) -> Option<&Worksheet> {
        self.sheets
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn get_sheet_by_name_mut(&mut self, name: &str) -> Option<&mut Worksheet> {
        self.sheets
            .iter_mut()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workbook_sheets() {
        let mut wb = Workbook::with_default_sheet();
        assert_eq!(wb.sheet_count(), 1);
        assert_eq!(wb.active_sheet().name, "Sheet1");

        let id2 = wb.add_sheet("Sheet2").unwrap();
        assert_eq!(wb.sheet_count(), 2);

        wb.rename_sheet(&id2, "Analysis").unwrap();
        assert_eq!(wb.get_sheet(&id2).unwrap().name, "Analysis");

        assert!(wb.add_sheet("Analysis").is_err()); // duplicate error

        wb.remove_sheet(&id2).unwrap();
        assert_eq!(wb.sheet_count(), 1);
        assert!(wb.remove_sheet(&wb.sheets[0].id.clone()).is_err()); // cannot remove only sheet
    }
}
