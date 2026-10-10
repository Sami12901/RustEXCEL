use exel_cell::{CellCoord, CellRange, CellState};
use exel_workbook::Worksheet;

/// Reversible document edit command.
#[derive(Debug, Clone)]
pub enum Command {
    SetCell {
        coord: CellCoord,
        old_state: Option<CellState>,
        new_state: CellState,
    },
    #[allow(dead_code)]
    SetRange {
        range: CellRange,
        old_cells: Vec<(CellCoord, Option<CellState>)>,
        new_cells: Vec<(CellCoord, CellState)>,
    },
}

impl Command {
    pub fn undo(&self, sheet: &mut Worksheet) {
        match self {
            Command::SetCell {
                coord, old_state, ..
            } => {
                if let Some(old) = old_state {
                    sheet.set_cell(*coord, old.clone());
                } else {
                    sheet.cells.remove(*coord);
                }
            }
            Command::SetRange { old_cells, .. } => {
                for (coord, old) in old_cells {
                    if let Some(old_state) = old {
                        sheet.set_cell(*coord, old_state.clone());
                    } else {
                        sheet.cells.remove(*coord);
                    }
                }
            }
        }
    }

    pub fn redo(&self, sheet: &mut Worksheet) {
        match self {
            Command::SetCell {
                coord, new_state, ..
            } => {
                sheet.set_cell(*coord, new_state.clone());
            }
            Command::SetRange { new_cells, .. } => {
                for (coord, new_state) in new_cells {
                    sheet.set_cell(*coord, new_state.clone());
                }
            }
        }
    }
}

/// Central undo/redo command stack.
#[derive(Debug, Default)]
pub struct CommandStack {
    undo_stack: Vec<Command>,
    redo_stack: Vec<Command>,
    max_history: usize,
}

impl CommandStack {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }

    pub fn push(&mut self, cmd: Command) {
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, sheet: &mut Worksheet) -> bool {
        if let Some(cmd) = self.undo_stack.pop() {
            cmd.undo(sheet);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, sheet: &mut Worksheet) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.redo(sheet);
            self.undo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
