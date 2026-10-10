use exel_cell::{CellCoord, CellRange, CellState, CellValue, SheetId};
use exel_core::{col_to_name, MAX_COLS, MAX_ROWS};
use exel_history::{HistoryCommand, HistoryError, HistoryManager};
use exel_workbook::Workbook;
use serde::{Deserialize, Serialize};

pub type WorkbookHistory = HistoryManager<Workbook>;
pub type BoxedWorkbookCommand = Box<dyn HistoryCommand<Workbook>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClearMode {
    All,
    ValuesOnly,
    FormatsOnly,
    CommentsOnly,
}

// ============================================================================
// 1. SetCellCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct SetCellCommand {
    pub sheet_id: SheetId,
    pub coord: CellCoord,
    pub new_state: CellState,
    pub prev_state: Option<CellState>,
    executed: bool,
}

impl SetCellCommand {
    pub fn new(sheet_id: SheetId, coord: CellCoord, new_state: CellState) -> Self {
        Self {
            sheet_id,
            coord,
            new_state,
            prev_state: None,
            executed: false,
        }
    }

    pub fn from_value(sheet_id: SheetId, coord: CellCoord, value: CellValue) -> Self {
        Self::new(sheet_id, coord, CellState::from_value(value))
    }
}

impl HistoryCommand<Workbook> for SetCellCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        if !self.executed {
            self.prev_state = sheet.get_cell(self.coord).cloned();
            self.executed = true;
        }

        sheet.set_cell(self.coord, self.new_state.clone());
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        if let Some(prev) = &self.prev_state {
            sheet.set_cell(self.coord, prev.clone());
        } else {
            sheet.cells.remove(self.coord);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Edit Cell"
    }
}

// ============================================================================
// 2. SetRangeCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct SetRangeCommand {
    pub sheet_id: SheetId,
    pub updates: Vec<(CellCoord, CellState)>,
    prev_states: Vec<(CellCoord, Option<CellState>)>,
    executed: bool,
    description: String,
}

impl SetRangeCommand {
    pub fn new<S: Into<String>>(
        sheet_id: SheetId,
        updates: Vec<(CellCoord, CellState)>,
        description: S,
    ) -> Self {
        Self {
            sheet_id,
            updates,
            prev_states: Vec::new(),
            executed: false,
            description: description.into(),
        }
    }
}

impl HistoryCommand<Workbook> for SetRangeCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        if !self.executed {
            self.prev_states = self
                .updates
                .iter()
                .map(|(c, _)| (*c, sheet.get_cell(*c).cloned()))
                .collect();
            self.executed = true;
        }

        for (coord, state) in &self.updates {
            sheet.set_cell(*coord, state.clone());
        }
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        for (coord, prev_opt) in &self.prev_states {
            if let Some(prev) = prev_opt {
                sheet.set_cell(*coord, prev.clone());
            } else {
                sheet.cells.remove(*coord);
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

// ============================================================================
// 3. ClearRangeCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct ClearRangeCommand {
    pub sheet_id: SheetId,
    pub range: CellRange,
    pub mode: ClearMode,
    prev_states: Vec<(CellCoord, CellState)>,
    executed: bool,
}

impl ClearRangeCommand {
    pub fn new(sheet_id: SheetId, range: CellRange, mode: ClearMode) -> Self {
        Self {
            sheet_id,
            range,
            mode,
            prev_states: Vec::new(),
            executed: false,
        }
    }
}

impl HistoryCommand<Workbook> for ClearRangeCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        if !self.executed {
            self.prev_states.clear();
            for r in self.range.start.row..=self.range.end.row {
                for c in self.range.start.col..=self.range.end.col {
                    if let Ok(coord) = CellCoord::new(r, c) {
                        if let Some(state) = sheet.get_cell(coord) {
                            self.prev_states.push((coord, state.clone()));
                        }
                    }
                }
            }
            self.executed = true;
        }

        for r in self.range.start.row..=self.range.end.row {
            for c in self.range.start.col..=self.range.end.col {
                if let Ok(coord) = CellCoord::new(r, c) {
                    match self.mode {
                        ClearMode::All => {
                            sheet.cells.remove(coord);
                        }
                        ClearMode::ValuesOnly => {
                            if let Some(cell) = sheet.cells.get_mut(coord) {
                                cell.value = CellValue::Empty;
                                cell.formula = None;
                            }
                        }
                        ClearMode::FormatsOnly => {
                            if let Some(cell) = sheet.cells.get_mut(coord) {
                                cell.style_id = 0;
                            }
                        }
                        ClearMode::CommentsOnly => {
                            if let Some(cell) = sheet.cells.get_mut(coord) {
                                cell.comment = None;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        // First clean cleared range
        for r in self.range.start.row..=self.range.end.row {
            for c in self.range.start.col..=self.range.end.col {
                if let Ok(coord) = CellCoord::new(r, c) {
                    sheet.cells.remove(coord);
                }
            }
        }

        // Restore previous populated states
        for (coord, state) in &self.prev_states {
            sheet.set_cell(*coord, state.clone());
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Clear Range"
    }
}

// ============================================================================
// 4. FormatRangeCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct FormatRangeCommand {
    pub sheet_id: SheetId,
    pub range: CellRange,
    pub style_id: u32,
    prev_styles: Vec<(CellCoord, u32)>,
    executed: bool,
}

impl FormatRangeCommand {
    pub fn new(sheet_id: SheetId, range: CellRange, style_id: u32) -> Self {
        Self {
            sheet_id,
            range,
            style_id,
            prev_styles: Vec::new(),
            executed: false,
        }
    }
}

impl HistoryCommand<Workbook> for FormatRangeCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        if !self.executed {
            self.prev_styles.clear();
            for r in self.range.start.row..=self.range.end.row {
                for c in self.range.start.col..=self.range.end.col {
                    if let Ok(coord) = CellCoord::new(r, c) {
                        let current_style = sheet.get_cell(coord).map(|s| s.style_id).unwrap_or(0);
                        self.prev_styles.push((coord, current_style));
                    }
                }
            }
            self.executed = true;
        }

        for r in self.range.start.row..=self.range.end.row {
            for c in self.range.start.col..=self.range.end.col {
                if let Ok(coord) = CellCoord::new(r, c) {
                    let mut state = sheet.get_cell(coord).cloned().unwrap_or_default();
                    state.style_id = self.style_id;
                    sheet.set_cell(coord, state);
                }
            }
        }
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        for (coord, prev_style) in &self.prev_styles {
            if let Some(cell) = sheet.cells.get_mut(*coord) {
                cell.style_id = *prev_style;
            } else if *prev_style != 0 {
                let mut state = CellState::default();
                state.style_id = *prev_style;
                sheet.set_cell(*coord, state);
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Format Cells"
    }
}

// ============================================================================
// 5. InsertRowCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct InsertRowCommand {
    pub sheet_id: SheetId,
    pub row: u32,
    pub count: u32,
    description: String,
}

impl InsertRowCommand {
    pub fn new(sheet_id: SheetId, row: u32, count: u32) -> Self {
        Self {
            description: format!("Insert {} Row(s) at {}", count, row),
            sheet_id,
            row,
            count,
        }
    }
}

impl HistoryCommand<Workbook> for InsertRowCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        // Extract cells with row >= self.row
        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(coord, _)| coord.row >= self.row)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        // Remove old coordinates
        for (coord, _) in &to_shift {
            sheet.cells.remove(*coord);
        }

        // Insert at new shifted coordinates (row + count)
        for (coord, state) in to_shift {
            let new_row = coord.row + self.count;
            if new_row <= MAX_ROWS {
                let new_coord = CellCoord::must_new(new_row, coord.col);
                sheet.set_cell(new_coord, state);
            }
        }
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(coord, _)| coord.row >= self.row + self.count)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        for (coord, _) in &to_shift {
            sheet.cells.remove(*coord);
        }

        for (coord, state) in to_shift {
            let new_row = coord.row - self.count;
            let new_coord = CellCoord::must_new(new_row, coord.col);
            sheet.set_cell(new_coord, state);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

// ============================================================================
// 6. DeleteRowCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct DeleteRowCommand {
    pub sheet_id: SheetId,
    pub row: u32,
    pub count: u32,
    deleted_cells: Vec<(CellCoord, CellState)>,
    executed: bool,
    description: String,
}

impl DeleteRowCommand {
    pub fn new(sheet_id: SheetId, row: u32, count: u32) -> Self {
        Self {
            description: format!("Delete {} Row(s) at {}", count, row),
            sheet_id,
            row,
            count,
            deleted_cells: Vec::new(),
            executed: false,
        }
    }
}

impl HistoryCommand<Workbook> for DeleteRowCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        let end_del = self.row + self.count - 1;

        if !self.executed {
            self.deleted_cells = sheet
                .cells
                .iter()
                .filter(|(c, _)| c.row >= self.row && c.row <= end_del)
                .map(|(c, s)| (*c, s.clone()))
                .collect();
            self.executed = true;
        }

        // Remove deleted rows
        for (c, _) in &self.deleted_cells {
            sheet.cells.remove(*c);
        }

        // Shift rows below up
        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(c, _)| c.row > end_del)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        for (c, _) in &to_shift {
            sheet.cells.remove(*c);
        }

        for (coord, state) in to_shift {
            let new_row = coord.row - self.count;
            let new_coord = CellCoord::must_new(new_row, coord.col);
            sheet.set_cell(new_coord, state);
        }
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        // Shift rows that were shifted up back down
        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(c, _)| c.row >= self.row)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        for (c, _) in &to_shift {
            sheet.cells.remove(*c);
        }

        for (coord, state) in to_shift {
            let new_row = coord.row + self.count;
            if new_row <= MAX_ROWS {
                let new_coord = CellCoord::must_new(new_row, coord.col);
                sheet.set_cell(new_coord, state);
            }
        }

        // Restore deleted cells
        for (coord, state) in &self.deleted_cells {
            sheet.set_cell(*coord, state.clone());
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

// ============================================================================
// 7. InsertColCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct InsertColCommand {
    pub sheet_id: SheetId,
    pub col: u32,
    pub count: u32,
    description: String,
}

impl InsertColCommand {
    pub fn new(sheet_id: SheetId, col: u32, count: u32) -> Self {
        Self {
            description: format!("Insert {} Column(s) at {}", count, col_to_name(col)),
            sheet_id,
            col,
            count,
        }
    }
}

impl HistoryCommand<Workbook> for InsertColCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(coord, _)| coord.col >= self.col)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        for (coord, _) in &to_shift {
            sheet.cells.remove(*coord);
        }

        for (coord, state) in to_shift {
            let new_col = coord.col + self.count;
            if new_col <= MAX_COLS {
                let new_coord = CellCoord::must_new(coord.row, new_col);
                sheet.set_cell(new_coord, state);
            }
        }
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(coord, _)| coord.col >= self.col + self.count)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        for (coord, _) in &to_shift {
            sheet.cells.remove(*coord);
        }

        for (coord, state) in to_shift {
            let new_col = coord.col - self.count;
            let new_coord = CellCoord::must_new(coord.row, new_col);
            sheet.set_cell(new_coord, state);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

// ============================================================================
// 8. DeleteColCommand
// ============================================================================
#[derive(Debug, Clone)]
pub struct DeleteColCommand {
    pub sheet_id: SheetId,
    pub col: u32,
    pub count: u32,
    deleted_cells: Vec<(CellCoord, CellState)>,
    executed: bool,
    description: String,
}

impl DeleteColCommand {
    pub fn new(sheet_id: SheetId, col: u32, count: u32) -> Self {
        Self {
            description: format!("Delete {} Column(s) at {}", count, col_to_name(col)),
            sheet_id,
            col,
            count,
            deleted_cells: Vec::new(),
            executed: false,
        }
    }
}

impl HistoryCommand<Workbook> for DeleteColCommand {
    fn execute(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        let end_del = self.col + self.count - 1;

        if !self.executed {
            self.deleted_cells = sheet
                .cells
                .iter()
                .filter(|(c, _)| c.col >= self.col && c.col <= end_del)
                .map(|(c, s)| (*c, s.clone()))
                .collect();
            self.executed = true;
        }

        for (c, _) in &self.deleted_cells {
            sheet.cells.remove(*c);
        }

        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(c, _)| c.col > end_del)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        for (c, _) in &to_shift {
            sheet.cells.remove(*c);
        }

        for (coord, state) in to_shift {
            let new_col = coord.col - self.count;
            let new_coord = CellCoord::must_new(coord.row, new_col);
            sheet.set_cell(new_coord, state);
        }
        Ok(())
    }

    fn undo(&mut self, wb: &mut Workbook) -> Result<(), HistoryError> {
        let sheet = wb
            .get_sheet_mut(&self.sheet_id)
            .ok_or_else(|| HistoryError::ExecutionFailed(format!("Sheet not found: {}", self.sheet_id)))?;

        let to_shift: Vec<(CellCoord, CellState)> = sheet
            .cells
            .iter()
            .filter(|(c, _)| c.col >= self.col)
            .map(|(c, s)| (*c, s.clone()))
            .collect();

        for (c, _) in &to_shift {
            sheet.cells.remove(*c);
        }

        for (coord, state) in to_shift {
            let new_col = coord.col + self.count;
            if new_col <= MAX_COLS {
                let new_coord = CellCoord::must_new(coord.row, new_col);
                sheet.set_cell(new_coord, state);
            }
        }

        for (coord, state) in &self.deleted_cells {
            sheet.set_cell(*coord, state.clone());
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_wb() -> (Workbook, SheetId) {
        let wb = Workbook::with_default_sheet();
        let sheet_id = wb.sheets[0].id.clone();
        (wb, sheet_id)
    }

    #[test]
    fn test_set_cell_command() {
        let (mut wb, sheet_id) = setup_wb();
        let mut history = WorkbookHistory::new(10);
        let coord = CellCoord::from_a1("B2").unwrap();

        let cmd = SetCellCommand::from_value(sheet_id.clone(), coord, CellValue::Number(100.0));
        history.execute(Box::new(cmd), &mut wb).unwrap();

        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(coord), CellValue::Number(100.0));

        // Undo
        history.undo(&mut wb).unwrap();
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(coord), CellValue::Empty);

        // Redo
        history.redo(&mut wb).unwrap();
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(coord), CellValue::Number(100.0));
    }

    #[test]
    fn test_clear_range_command() {
        let (mut wb, sheet_id) = setup_wb();
        let mut history = WorkbookHistory::new(10);

        let c1 = CellCoord::from_a1("A1").unwrap();
        let c2 = CellCoord::from_a1("A2").unwrap();
        wb.get_sheet_mut(&sheet_id).unwrap().set_cell_value(c1, CellValue::Text("First".into()));
        wb.get_sheet_mut(&sheet_id).unwrap().set_cell_value(c2, CellValue::Text("Second".into()));

        let range = CellRange::from_a1("A1:A2").unwrap();
        let cmd = ClearRangeCommand::new(sheet_id.clone(), range, ClearMode::All);
        history.execute(Box::new(cmd), &mut wb).unwrap();

        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c1), CellValue::Empty);
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c2), CellValue::Empty);

        // Undo restores values
        history.undo(&mut wb).unwrap();
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c1), CellValue::Text("First".into()));
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c2), CellValue::Text("Second".into()));
    }

    #[test]
    fn test_insert_and_delete_row() {
        let (mut wb, sheet_id) = setup_wb();
        let mut history = WorkbookHistory::new(10);

        let c_row1 = CellCoord::from_a1("A1").unwrap();
        let c_row2 = CellCoord::from_a1("A2").unwrap();
        wb.get_sheet_mut(&sheet_id).unwrap().set_cell_value(c_row1, CellValue::Number(1.0));
        wb.get_sheet_mut(&sheet_id).unwrap().set_cell_value(c_row2, CellValue::Number(2.0));

        // Insert row at 2
        let insert_cmd = InsertRowCommand::new(sheet_id.clone(), 2, 1);
        history.execute(Box::new(insert_cmd), &mut wb).unwrap();

        // A1 stays 1.0, A2 is empty, A3 becomes 2.0
        let c_row3 = CellCoord::from_a1("A3").unwrap();
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_row1), CellValue::Number(1.0));
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_row2), CellValue::Empty);
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_row3), CellValue::Number(2.0));

        // Delete row at 2
        let del_cmd = DeleteRowCommand::new(sheet_id.clone(), 2, 1);
        history.execute(Box::new(del_cmd), &mut wb).unwrap();

        // Now A1 is 1.0, A2 is 2.0
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_row1), CellValue::Number(1.0));
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_row2), CellValue::Number(2.0));

        // Undo Delete Row restores A3 as 2.0 and A2 as empty
        history.undo(&mut wb).unwrap();
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_row2), CellValue::Empty);
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_row3), CellValue::Number(2.0));
    }

    #[test]
    fn test_insert_and_delete_col() {
        let (mut wb, sheet_id) = setup_wb();
        let mut history = WorkbookHistory::new(10);

        let c_a1 = CellCoord::from_a1("A1").unwrap();
        let c_b1 = CellCoord::from_a1("B1").unwrap();
        wb.get_sheet_mut(&sheet_id).unwrap().set_cell_value(c_a1, CellValue::Number(10.0));
        wb.get_sheet_mut(&sheet_id).unwrap().set_cell_value(c_b1, CellValue::Number(20.0));

        // Insert 1 column at B (col 2)
        let ins_cmd = InsertColCommand::new(sheet_id.clone(), 2, 1);
        history.execute(Box::new(ins_cmd), &mut wb).unwrap();

        let c_c1 = CellCoord::from_a1("C1").unwrap();
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_a1), CellValue::Number(10.0));
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_b1), CellValue::Empty);
        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_c1), CellValue::Number(20.0));

        // Delete column at B (col 2)
        let del_cmd = DeleteColCommand::new(sheet_id.clone(), 2, 1);
        history.execute(Box::new(del_cmd), &mut wb).unwrap();

        assert_eq!(wb.get_sheet(&sheet_id).unwrap().get_cell_value(c_b1), CellValue::Number(20.0));
    }
}
