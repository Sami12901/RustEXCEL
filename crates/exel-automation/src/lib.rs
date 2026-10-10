//! Batch scriptable spreadsheet automation engine for Rust EXCEL.
//!
//! Provides scriptable commands (`SetCell`, `InsertRow`, `DeleteRow`, `Recalculate`, `ExportFile`),
//! JSON script serialization/deserialization, and an execution runner operating on `Workbook`.

use exel_calc::CalcEngine;
use exel_cell::{CellCoord, CellValue};
use exel_core::{MAX_COLS, MAX_ROWS};
use exel_workbook::Workbook;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AutomationError {
    #[error("Sheet '{0}' not found in workbook")]
    SheetNotFound(String),
    #[error("Invalid coordinate: {0}")]
    InvalidCoordinate(String),
    #[error("IO / Export error: {0}")]
    ExportFailed(String),
    #[error("JSON script parse error: {0}")]
    ParseError(String),
}

/// Target export file format for automated export jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Xlsx,
}

/// A single scriptable batch automation command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AutomationCommand {
    /// Sets a cell's static value or formula by A1 string reference.
    SetCell {
        sheet: Option<String>,
        a1: String,
        formula: Option<String>,
        value: Option<CellValue>,
    },
    /// Inserts a blank row at `row_index` shifting subsequent rows down.
    InsertRow {
        sheet: Option<String>,
        row_index: u32,
    },
    /// Inserts a blank column at `col_index` shifting subsequent columns right.
    InsertColumn {
        sheet: Option<String>,
        col_index: u32,
    },
    /// Deletes a row at `row_index` shifting subsequent rows up.
    DeleteRow {
        sheet: Option<String>,
        row_index: u32,
    },
    /// Deletes a column at `col_index` shifting subsequent columns left.
    DeleteColumn {
        sheet: Option<String>,
        col_index: u32,
    },
    /// Adds a new sheet with the given name.
    AddSheet { name: String },
    /// Triggers a full workbook formula recalculation pass.
    Recalculate,
    /// Exports the workbook or active sheet to disk.
    ExportFile { format: ExportFormat, path: PathBuf },
}

/// A batch automation script containing an ordered sequence of commands.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AutomationScript {
    pub name: Option<String>,
    pub commands: Vec<AutomationCommand>,
}

impl AutomationScript {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_command(mut self, cmd: AutomationCommand) -> Self {
        self.commands.push(cmd);
        self
    }

    pub fn from_json(json_str: &str) -> Result<Self, AutomationError> {
        serde_json::from_str(json_str).map_err(|e| AutomationError::ParseError(e.to_string()))
    }

    pub fn to_json(&self) -> Result<String, AutomationError> {
        serde_json::to_string_pretty(self).map_err(|e| AutomationError::ParseError(e.to_string()))
    }
}

/// Execution summary report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AutomationExecutionReport {
    pub commands_executed: usize,
    pub cells_modified: usize,
    pub recalculated: bool,
    pub exported_files: Vec<PathBuf>,
}

/// Execution runner for scriptable batch operations.
pub struct AutomationRunner;

impl AutomationRunner {
    /// Executes all commands in `script` sequentially against the `Workbook`.
    pub fn run_script(
        wb: &mut Workbook,
        script: &AutomationScript,
    ) -> Result<AutomationExecutionReport, AutomationError> {
        let mut report = AutomationExecutionReport::default();

        for cmd in &script.commands {
            Self::execute_command(wb, cmd, &mut report)?;
            report.commands_executed += 1;
        }

        Ok(report)
    }

    /// Executes an individual automation command against the workbook.
    pub fn execute_command(
        wb: &mut Workbook,
        cmd: &AutomationCommand,
        report: &mut AutomationExecutionReport,
    ) -> Result<(), AutomationError> {
        match cmd {
            AutomationCommand::SetCell {
                sheet,
                a1,
                formula,
                value,
            } => {
                let coord = CellCoord::from_a1(a1)
                    .map_err(|e| AutomationError::InvalidCoordinate(e.to_string()))?;

                let worksheet = match sheet {
                    Some(s) => wb
                        .get_sheet_by_name_mut(s)
                        .ok_or_else(|| AutomationError::SheetNotFound(s.clone()))?,
                    None => wb.active_sheet_mut(),
                };

                let mut state = worksheet.get_cell(coord).cloned().unwrap_or_default();
                if let Some(f) = formula {
                    state.formula = Some(f.clone());
                }
                if let Some(v) = value {
                    state.value = v.clone();
                } else if formula.is_some() {
                    state.value = CellValue::Empty;
                }

                worksheet.set_cell(coord, state);
                report.cells_modified += 1;
            }

            AutomationCommand::InsertRow { sheet, row_index } => {
                let worksheet = match sheet {
                    Some(s) => wb
                        .get_sheet_by_name_mut(s)
                        .ok_or_else(|| AutomationError::SheetNotFound(s.clone()))?,
                    None => wb.active_sheet_mut(),
                };

                let mut shifted = Vec::new();
                for (&coord, state) in worksheet.cells.iter() {
                    if coord.row >= *row_index && coord.row < MAX_ROWS {
                        let new_coord = CellCoord::must_new(coord.row + 1, coord.col);
                        shifted.push((coord, new_coord, state.clone()));
                    }
                }

                // Apply in descending order to avoid collisions
                shifted.sort_by_key(|a| std::cmp::Reverse(a.0.row));
                for (old_c, new_c, state) in shifted {
                    worksheet.cells.remove(old_c);
                    worksheet.cells.set(new_c, state);
                }
            }

            AutomationCommand::InsertColumn { sheet, col_index } => {
                let worksheet = match sheet {
                    Some(s) => wb
                        .get_sheet_by_name_mut(s)
                        .ok_or_else(|| AutomationError::SheetNotFound(s.clone()))?,
                    None => wb.active_sheet_mut(),
                };

                let mut shifted = Vec::new();
                for (&coord, state) in worksheet.cells.iter() {
                    if coord.col >= *col_index && coord.col < MAX_COLS {
                        let new_coord = CellCoord::must_new(coord.row, coord.col + 1);
                        shifted.push((coord, new_coord, state.clone()));
                    }
                }

                shifted.sort_by_key(|a| std::cmp::Reverse(a.0.col));
                for (old_c, new_c, state) in shifted {
                    worksheet.cells.remove(old_c);
                    worksheet.cells.set(new_c, state);
                }
            }

            AutomationCommand::DeleteRow { sheet, row_index } => {
                let worksheet = match sheet {
                    Some(s) => wb
                        .get_sheet_by_name_mut(s)
                        .ok_or_else(|| AutomationError::SheetNotFound(s.clone()))?,
                    None => wb.active_sheet_mut(),
                };

                let mut to_remove = Vec::new();
                let mut shifted = Vec::new();

                for (&coord, state) in worksheet.cells.iter() {
                    if coord.row == *row_index {
                        to_remove.push(coord);
                    } else if coord.row > *row_index {
                        let new_coord = CellCoord::must_new(coord.row - 1, coord.col);
                        shifted.push((coord, new_coord, state.clone()));
                    }
                }

                for c in to_remove {
                    worksheet.cells.remove(c);
                }

                shifted.sort_by_key(|a| a.0.row);
                for (old_c, new_c, state) in shifted {
                    worksheet.cells.remove(old_c);
                    worksheet.cells.set(new_c, state);
                }
            }

            AutomationCommand::DeleteColumn { sheet, col_index } => {
                let worksheet = match sheet {
                    Some(s) => wb
                        .get_sheet_by_name_mut(s)
                        .ok_or_else(|| AutomationError::SheetNotFound(s.clone()))?,
                    None => wb.active_sheet_mut(),
                };

                let mut to_remove = Vec::new();
                let mut shifted = Vec::new();

                for (&coord, state) in worksheet.cells.iter() {
                    if coord.col == *col_index {
                        to_remove.push(coord);
                    } else if coord.col > *col_index {
                        let new_coord = CellCoord::must_new(coord.row, coord.col - 1);
                        shifted.push((coord, new_coord, state.clone()));
                    }
                }

                for c in to_remove {
                    worksheet.cells.remove(c);
                }

                shifted.sort_by_key(|a| a.0.col);
                for (old_c, new_c, state) in shifted {
                    worksheet.cells.remove(old_c);
                    worksheet.cells.set(new_c, state);
                }
            }

            AutomationCommand::AddSheet { name } => {
                let _ = wb
                    .add_sheet(name)
                    .map_err(|e| AutomationError::ExportFailed(e.to_string()))?;
            }

            AutomationCommand::Recalculate => {
                let mut engine = CalcEngine::new();
                engine.recalculate_workbook(wb);
                report.recalculated = true;
            }

            AutomationCommand::ExportFile { format, path } => {
                match format {
                    ExportFormat::Csv => {
                        exel_csv::export_csv_file(wb, path, &exel_csv::CsvExportOptions::default())
                            .map_err(|e| AutomationError::ExportFailed(e.to_string()))?;
                    }
                    ExportFormat::Xlsx => {
                        exel_xlsx::write_xlsx_file(wb, path, None)
                            .map_err(|e| AutomationError::ExportFailed(e.to_string()))?;
                    }
                }
                report.exported_files.push(path.clone());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_script_set_cell_and_recalc() {
        let mut wb = Workbook::with_default_sheet();

        let script = AutomationScript::new()
            .with_command(AutomationCommand::SetCell {
                sheet: None,
                a1: "A1".into(),
                formula: None,
                value: Some(CellValue::Number(50.0)),
            })
            .with_command(AutomationCommand::SetCell {
                sheet: None,
                a1: "A2".into(),
                formula: None,
                value: Some(CellValue::Number(100.0)),
            })
            .with_command(AutomationCommand::SetCell {
                sheet: None,
                a1: "A3".into(),
                formula: Some("=A1 + A2".into()),
                value: None,
            })
            .with_command(AutomationCommand::Recalculate);

        let report = AutomationRunner::run_script(&mut wb, &script).unwrap();

        assert_eq!(report.commands_executed, 4);
        assert_eq!(report.cells_modified, 3);
        assert!(report.recalculated);

        let active = wb.active_sheet();
        assert_eq!(
            active.get_cell_value(CellCoord::must_new(3, 1)),
            CellValue::Number(150.0)
        );
    }

    #[test]
    fn test_insert_and_delete_row() {
        let mut wb = Workbook::with_default_sheet();
        let sheet = wb.active_sheet_mut();
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Row 1".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Row 2".into()));

        // Insert row at 2
        let mut report = AutomationExecutionReport::default();
        AutomationRunner::execute_command(
            &mut wb,
            &AutomationCommand::InsertRow {
                sheet: None,
                row_index: 2,
            },
            &mut report,
        )
        .unwrap();

        let active = wb.active_sheet();
        assert_eq!(
            active.get_cell_value(CellCoord::must_new(1, 1)),
            CellValue::Text("Row 1".into())
        );
        assert_eq!(
            active.get_cell_value(CellCoord::must_new(2, 1)),
            CellValue::Empty
        );
        assert_eq!(
            active.get_cell_value(CellCoord::must_new(3, 1)),
            CellValue::Text("Row 2".into())
        );

        // Delete row at 2
        AutomationRunner::execute_command(
            &mut wb,
            &AutomationCommand::DeleteRow {
                sheet: None,
                row_index: 2,
            },
            &mut report,
        )
        .unwrap();

        let active_after = wb.active_sheet();
        assert_eq!(
            active_after.get_cell_value(CellCoord::must_new(2, 1)),
            CellValue::Text("Row 2".into())
        );
    }

    #[test]
    fn test_script_json_roundtrip() {
        let script = AutomationScript::new()
            .with_command(AutomationCommand::SetCell {
                sheet: Some("Sheet1".into()),
                a1: "B5".into(),
                formula: Some("=SUM(A1:A10)".into()),
                value: None,
            })
            .with_command(AutomationCommand::Recalculate);

        let json = script.to_json().unwrap();
        let parsed = AutomationScript::from_json(&json).unwrap();
        assert_eq!(script, parsed);
    }
}
