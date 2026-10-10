use crate::calc::recalculate_workbook;
use crate::commands::{Command, CommandStack};
use exel_cell::{CellCoord, CellState, CellValue, SparseCellStore};
use exel_core::{MAX_COLS, MAX_ROWS};
use exel_shortcuts::ShortcutRegistry;
use exel_ui::{SpreadsheetAction, SpreadsheetAppView, SpreadsheetUiState};
use exel_workbook::{Workbook, Worksheet};
use std::path::PathBuf;

/// Main native spreadsheet desktop application.
pub struct RustExelApp {
    pub workbook: Workbook,
    pub ui_state: SpreadsheetUiState,
    pub shortcuts: ShortcutRegistry,
    pub command_stack: CommandStack,
    pub file_path: Option<PathBuf>,
}

impl RustExelApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut workbook = Workbook::with_default_sheet();

        // Populate sample initial data to showcase grid features
        let sheet = workbook.active_sheet_mut();
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Product".into()));
        sheet.set_cell_value(
            CellCoord::must_new(1, 2),
            CellValue::Text("Q1 Sales".into()),
        );
        sheet.set_cell_value(
            CellCoord::must_new(1, 3),
            CellValue::Text("Q2 Sales".into()),
        );
        sheet.set_cell_value(CellCoord::must_new(1, 4), CellValue::Text("Total".into()));

        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Widgets".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 2), CellValue::Number(12500.0));
        sheet.set_cell_value(CellCoord::must_new(2, 3), CellValue::Number(14300.0));
        sheet.set_cell_formula(
            CellCoord::must_new(2, 4),
            "=SUM(B2:C2)".into(),
            CellValue::Number(26800.0),
        );

        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Text("Gadgets".into()));
        sheet.set_cell_value(CellCoord::must_new(3, 2), CellValue::Number(8900.0));
        sheet.set_cell_value(CellCoord::must_new(3, 3), CellValue::Number(11200.0));
        sheet.set_cell_formula(
            CellCoord::must_new(3, 4),
            "=SUM(B3:C3)".into(),
            CellValue::Number(20100.0),
        );

        sheet.set_cell_value(CellCoord::must_new(4, 1), CellValue::Text("Devices".into()));
        sheet.set_cell_value(CellCoord::must_new(4, 2), CellValue::Number(21400.0));
        sheet.set_cell_value(CellCoord::must_new(4, 3), CellValue::Number(19800.0));
        sheet.set_cell_formula(
            CellCoord::must_new(4, 4),
            "=SUM(B4:C4)".into(),
            CellValue::Number(41200.0),
        );

        sheet.set_cell_value(CellCoord::must_new(5, 1), CellValue::Text("Total".into()));
        sheet.set_cell_formula(
            CellCoord::must_new(5, 2),
            "=SUM(B2:B4)".into(),
            CellValue::Number(42800.0),
        );
        sheet.set_cell_formula(
            CellCoord::must_new(5, 3),
            "=SUM(C2:C4)".into(),
            CellValue::Number(45300.0),
        );
        sheet.set_cell_formula(
            CellCoord::must_new(5, 4),
            "=SUM(D2:D4)".into(),
            CellValue::Number(88100.0),
        );

        let ui_state = SpreadsheetUiState::default();
        let shortcuts = ShortcutRegistry::default_for_current_platform();
        let command_stack = CommandStack::new();

        Self {
            workbook,
            ui_state,
            shortcuts,
            command_stack,
            file_path: None,
        }
    }

    /// Handles actions produced by the UI components.
    pub fn handle_actions(&mut self, actions: Vec<SpreadsheetAction>) {
        for action in actions {
            match action {
                SpreadsheetAction::SetCellValue { coord, value } => {
                    let sheet = self.workbook.active_sheet_mut();
                    let old_state = sheet.get_cell(coord).cloned();
                    let new_state = CellState::from_value(value);

                    self.command_stack.push(Command::SetCell {
                        coord,
                        old_state,
                        new_state: new_state.clone(),
                    });
                    sheet.set_cell(coord, new_state);
                    recalculate_workbook(&mut self.workbook);
                }
                SpreadsheetAction::SetCellFormula { coord, formula } => {
                    let sheet = self.workbook.active_sheet_mut();
                    let old_state = sheet.get_cell(coord).cloned();
                    let new_state = CellState::from_formula(formula, CellValue::Empty);

                    self.command_stack.push(Command::SetCell {
                        coord,
                        old_state,
                        new_state: new_state.clone(),
                    });
                    sheet.set_cell(coord, new_state);
                    recalculate_workbook(&mut self.workbook);
                }
                SpreadsheetAction::ClearCells { range } => {
                    let sheet = self.workbook.active_sheet_mut();
                    let mut old_cells = Vec::new();
                    let mut new_cells = Vec::new();

                    for r in range.start.row..=range.end.row {
                        for c in range.start.col..=range.end.col {
                            if let Ok(coord) = CellCoord::new(r, c) {
                                old_cells.push((coord, sheet.get_cell(coord).cloned()));
                                new_cells.push((coord, CellState::default()));
                                sheet.cells.remove(coord);
                            }
                        }
                    }

                    self.command_stack.push(Command::SetRange {
                        range,
                        old_cells,
                        new_cells,
                    });
                    recalculate_workbook(&mut self.workbook);
                }
                SpreadsheetAction::ApplyStyle { range, style } => {
                    let style_id = self.ui_state.style_table.get_or_insert(style);
                    let sheet = self.workbook.active_sheet_mut();

                    for r in range.start.row..=range.end.row {
                        for c in range.start.col..=range.end.col {
                            if let Ok(coord) = CellCoord::new(r, c) {
                                let mut state = sheet.get_cell(coord).cloned().unwrap_or_default();
                                state.style_id = style_id;
                                sheet.set_cell(coord, state);
                            }
                        }
                    }
                }
                SpreadsheetAction::AutoSum { range, func } => {
                    let sheet = self.workbook.active_sheet_mut();
                    let formula = format!("={}({})", func.formula_name(), range.to_a1());

                    // Place AutoSum in cell immediately below the range
                    let target_row = (range.end.row + 1).min(MAX_ROWS);
                    let target_coord = CellCoord::must_new(target_row, range.start.col);

                    sheet.set_cell_formula(target_coord, formula, CellValue::Empty);
                    recalculate_workbook(&mut self.workbook);
                    let formula_text = self
                        .workbook
                        .active_sheet()
                        .get_cell(target_coord)
                        .and_then(|c| c.formula.clone())
                        .unwrap_or_default();
                    self.ui_state.select_cell(target_coord, formula_text);
                }
                SpreadsheetAction::AutoFill { source, target } => {
                    let sheet = self.workbook.active_sheet_mut();
                    let source_val = sheet.get_cell_value(source.start);

                    for r in target.start.row..=target.end.row {
                        for c in target.start.col..=target.end.col {
                            if let Ok(coord) = CellCoord::new(r, c) {
                                if !source.contains(coord) {
                                    sheet.set_cell_value(coord, source_val.clone());
                                }
                            }
                        }
                    }
                    recalculate_workbook(&mut self.workbook);
                }
                SpreadsheetAction::SelectSheet(id) => {
                    if let Some(pos) = self.workbook.sheets.iter().position(|s| s.id == id) {
                        let _ = self.workbook.set_active_sheet_index(pos);
                    }
                }
                SpreadsheetAction::AddSheet => {
                    let next_idx = self.workbook.sheet_count() + 1;
                    let name = format!("Sheet{}", next_idx);
                    if let Ok(id) = self.workbook.add_sheet(&name) {
                        if let Some(pos) = self.workbook.sheets.iter().position(|s| s.id == id) {
                            let _ = self.workbook.set_active_sheet_index(pos);
                        }
                        self.ui_state.status_message = format!("Added sheet '{name}'");
                    }
                }
                SpreadsheetAction::RenameSheet { id, new_name } => {
                    if let Ok(()) = self.workbook.rename_sheet(&id, &new_name) {
                        self.ui_state.status_message = format!("Renamed sheet to '{new_name}'");
                    }
                }
                SpreadsheetAction::DeleteSheet(id) => {
                    if let Ok(()) = self.workbook.remove_sheet(&id) {
                        self.ui_state.status_message = "Sheet deleted".into();
                    }
                }
                SpreadsheetAction::ResizeColumn { col, width } => {
                    self.workbook.active_sheet_mut().set_col_width(col, width);
                }
                SpreadsheetAction::ResizeRow { row, height } => {
                    self.workbook.active_sheet_mut().set_row_height(row, height);
                }
                SpreadsheetAction::InsertRow(row) => {
                    insert_row(self.workbook.active_sheet_mut(), row);
                    recalculate_workbook(&mut self.workbook);
                    self.ui_state.status_message = format!("Inserted row at {row}");
                }
                SpreadsheetAction::InsertColumn(col) => {
                    insert_column(self.workbook.active_sheet_mut(), col);
                    recalculate_workbook(&mut self.workbook);
                    self.ui_state.status_message = format!("Inserted column at {col}");
                }
                SpreadsheetAction::DeleteRow(row) => {
                    delete_row(self.workbook.active_sheet_mut(), row);
                    recalculate_workbook(&mut self.workbook);
                    self.ui_state.status_message = format!("Deleted row {row}");
                }
                SpreadsheetAction::DeleteColumn(col) => {
                    delete_column(self.workbook.active_sheet_mut(), col);
                    recalculate_workbook(&mut self.workbook);
                    self.ui_state.status_message = format!("Deleted column {col}");
                }
                SpreadsheetAction::StatusMessage(msg) => {
                    self.ui_state.status_message = msg;
                }
                SpreadsheetAction::Undo => {
                    let sheet = self.workbook.active_sheet_mut();
                    self.command_stack.undo(sheet);
                    recalculate_workbook(&mut self.workbook);
                }
                SpreadsheetAction::Redo => {
                    let sheet = self.workbook.active_sheet_mut();
                    self.command_stack.redo(sheet);
                    recalculate_workbook(&mut self.workbook);
                }
                SpreadsheetAction::Cut => {
                    let sel = self.ui_state.selection;
                    let sheet = self.workbook.active_sheet_mut();
                    if let Err(e) = exel_clipboard::copy_range(sheet, sel) {
                        tracing::warn!("Failed to cut to system clipboard: {e}");
                    }
                    for r in sel.start.row..=sel.end.row {
                        for c in sel.start.col..=sel.end.col {
                            if let Ok(coord) = CellCoord::new(r, c) {
                                sheet.cells.remove(coord);
                            }
                        }
                    }
                    recalculate_workbook(&mut self.workbook);
                    self.ui_state.status_message = "Cut selection to clipboard".into();
                }
                SpreadsheetAction::Copy => {
                    let sel = self.ui_state.selection;
                    let sheet = self.workbook.active_sheet();
                    if let Err(e) = exel_clipboard::copy_range(sheet, sel) {
                        tracing::warn!("Failed to copy to system clipboard: {e}");
                    } else {
                        self.ui_state.status_message = "Copied selection to clipboard".into();
                    }
                }
                SpreadsheetAction::Paste => {
                    let start = self.ui_state.active_cell;
                    let sheet = self.workbook.active_sheet_mut();
                    match exel_clipboard::paste_to_sheet(sheet, start) {
                        Ok(cells) => {
                            recalculate_workbook(&mut self.workbook);
                            self.ui_state.status_message =
                                format!("Pasted {} cells from clipboard", cells.len());
                        }
                        Err(e) => {
                            tracing::warn!("Paste failed: {e}");
                            self.ui_state.status_message = "Paste failed".into();
                        }
                    }
                }
                SpreadsheetAction::Recalculate => {
                    recalculate_workbook(&mut self.workbook);
                    self.ui_state.status_message = "Recalculated all formulas".into();
                }
                SpreadsheetAction::NewFile => {
                    self.workbook = Workbook::with_default_sheet();
                    self.command_stack.clear();
                    self.file_path = None;
                    self.ui_state.status_message = "New Workbook created".into();
                }
                SpreadsheetAction::OpenFile => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "All Supported Spreadsheets (*.xlsx, *.csv, *.tsv, *.json)",
                            &["xlsx", "csv", "tsv", "json"],
                        )
                        .add_filter("Excel Workbook (*.xlsx)", &["xlsx"])
                        .add_filter("CSV Comma Delimited (*.csv)", &["csv"])
                        .add_filter("TSV Tab Delimited (*.tsv)", &["tsv"])
                        .add_filter("JSON Table (*.json)", &["json"])
                        .pick_file()
                    {
                        if let Err(e) = self.load_from_path(path) {
                            self.ui_state.status_message = format!("Error opening file: {e}");
                        }
                    }
                }
                SpreadsheetAction::SaveFile => {
                    if let Some(path) = self.file_path.clone() {
                        if let Err(e) = self.save_to_path(path) {
                            self.ui_state.status_message = format!("Error saving: {e}");
                        }
                    } else {
                        self.save_as_dialog();
                    }
                }
                SpreadsheetAction::SaveAsFile => {
                    self.save_as_dialog();
                }
                SpreadsheetAction::ExportCsv => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV Comma Delimited (*.csv)", &["csv"])
                        .set_file_name("export.csv")
                        .save_file()
                    {
                        let opts = exel_csv::CsvExportOptions {
                            delimiter: b',',
                            sheet_index: self.workbook.active_sheet_index,
                            range: None,
                        };
                        match exel_csv::export_csv_file(&self.workbook, &path, &opts) {
                            Ok(()) => {
                                self.ui_state.status_message =
                                    format!("Exported CSV to {}", path.display());
                            }
                            Err(e) => {
                                self.ui_state.status_message = format!("Export CSV error: {e}");
                            }
                        }
                    }
                }
                SpreadsheetAction::ExportXlsx => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Excel Workbook (*.xlsx)", &["xlsx"])
                        .set_file_name("export.xlsx")
                        .save_file()
                    {
                        match exel_xlsx::write_xlsx_file(
                            &self.workbook,
                            &path,
                            Some(&self.ui_state.style_table),
                        ) {
                            Ok(()) => {
                                self.ui_state.status_message =
                                    format!("Exported XLSX to {}", path.display());
                            }
                            Err(e) => {
                                self.ui_state.status_message = format!("Export XLSX error: {e}");
                            }
                        }
                    }
                }
            }
        }
    }

    /// Invokes the native Save As file dialog with format filters.
    pub fn save_as_dialog(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Excel Workbook (*.xlsx)", &["xlsx"])
            .add_filter("CSV Comma Delimited (*.csv)", &["csv"])
            .add_filter("TSV Tab Delimited (*.tsv)", &["tsv"])
            .add_filter("JSON Table (*.json)", &["json"]);

        if let Some(path) = &self.file_path {
            if let Some(parent) = path.parent() {
                dialog = dialog.set_directory(parent);
            }
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                dialog = dialog.set_file_name(file_name);
            }
        } else {
            dialog = dialog.set_file_name("Workbook.xlsx");
        }

        if let Some(path) = dialog.save_file() {
            if let Err(e) = self.save_to_path(path) {
                self.ui_state.status_message = format!("Error saving: {e}");
            }
        }
    }

    /// Loads spreadsheet from a file path with extension detection.
    pub fn load_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let wb = match ext.as_str() {
            "xlsx" => {
                exel_xlsx::read_xlsx_file(&path).map_err(|e| format!("Failed to read XLSX: {e}"))?
            }
            "csv" => {
                let opts = exel_csv::CsvImportOptions {
                    delimiter: Some(b','),
                    ..Default::default()
                };
                exel_csv::import_csv_file(&path, &opts)
                    .map_err(|e| format!("Failed to read CSV: {e}"))?
            }
            "tsv" => {
                let opts = exel_csv::CsvImportOptions {
                    delimiter: Some(b'\t'),
                    ..Default::default()
                };
                exel_csv::import_csv_file(&path, &opts)
                    .map_err(|e| format!("Failed to read TSV: {e}"))?
            }
            "json" => {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read JSON file: {e}"))?;
                let sheet_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Data");
                let sheet = exel_csv::import_json_str(&content, sheet_name)
                    .map_err(|e| format!("Failed to parse JSON table: {e}"))?;
                let mut new_wb = Workbook::new();
                new_wb.sheets.push(sheet);
                new_wb
            }
            other => {
                return Err(format!(
                    "Unsupported file extension '.{other}'. Supported formats: .xlsx, .csv, .tsv, .json"
                ));
            }
        };

        self.workbook = wb;
        self.file_path = Some(path.clone());
        self.command_stack.clear();
        recalculate_workbook(&mut self.workbook);

        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        self.ui_state.status_message = format!("Opened: {file_name}");
        Ok(())
    }

    /// Saves the current workbook to disk with format detection by extension.
    pub fn save_to_path(&mut self, path: PathBuf) -> Result<(), String> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "xlsx" => {
                exel_xlsx::write_xlsx_file(&self.workbook, &path, Some(&self.ui_state.style_table))
                    .map_err(|e| format!("Failed to save XLSX: {e}"))?;
            }
            "csv" => {
                let opts = exel_csv::CsvExportOptions {
                    delimiter: b',',
                    sheet_index: self.workbook.active_sheet_index,
                    range: None,
                };
                exel_csv::export_csv_file(&self.workbook, &path, &opts)
                    .map_err(|e| format!("Failed to save CSV: {e}"))?;
            }
            "tsv" => {
                let opts = exel_csv::CsvExportOptions {
                    delimiter: b'\t',
                    sheet_index: self.workbook.active_sheet_index,
                    range: None,
                };
                exel_csv::export_csv_file(&self.workbook, &path, &opts)
                    .map_err(|e| format!("Failed to save TSV: {e}"))?;
            }
            "json" => {
                let json_str = exel_csv::export_json_records(self.workbook.active_sheet())
                    .map_err(|e| format!("Failed to serialize JSON records: {e}"))?;
                exel_io::save_atomic_bytes(&path, json_str.as_bytes())
                    .map_err(|e| format!("Failed to save JSON: {e}"))?;
            }
            other => {
                let err_msg = if other.is_empty() {
                    "Cannot save file: missing file extension. Supported extensions: .xlsx, .csv, .tsv, .json".to_string()
                } else {
                    format!("Unsupported file extension '.{other}'. Supported formats: .xlsx, .csv, .tsv, .json")
                };
                return Err(err_msg);
            }
        }

        self.file_path = Some(path.clone());
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("document");
        self.ui_state.status_message = format!("Saved: {file_name}");
        Ok(())
    }

    /// Computes the desktop window title based on current open file path.
    pub fn window_title(&self) -> String {
        match &self.file_path {
            Some(path) => {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Workbook");
                format!("Rust EXEL - {name}")
            }
            None => "Rust EXEL - Professional Spreadsheet".to_string(),
        }
    }
}

fn insert_row(sheet: &mut Worksheet, at_row: u32) {
    let mut new_cells = SparseCellStore::new();
    for (coord, state) in sheet.cells.iter() {
        if coord.row >= at_row && coord.row < MAX_ROWS {
            if let Ok(new_coord) = CellCoord::new(coord.row + 1, coord.col) {
                new_cells.set(new_coord, state.clone());
            }
        } else if coord.row < at_row {
            new_cells.set(*coord, state.clone());
        }
    }
    sheet.cells = new_cells;
}

fn delete_row(sheet: &mut Worksheet, at_row: u32) {
    let mut new_cells = SparseCellStore::new();
    for (coord, state) in sheet.cells.iter() {
        if coord.row > at_row {
            if let Ok(new_coord) = CellCoord::new(coord.row - 1, coord.col) {
                new_cells.set(new_coord, state.clone());
            }
        } else if coord.row < at_row {
            new_cells.set(*coord, state.clone());
        }
    }
    sheet.cells = new_cells;
}

fn insert_column(sheet: &mut Worksheet, at_col: u32) {
    let mut new_cells = SparseCellStore::new();
    for (coord, state) in sheet.cells.iter() {
        if coord.col >= at_col && coord.col < MAX_COLS {
            if let Ok(new_coord) = CellCoord::new(coord.row, coord.col + 1) {
                new_cells.set(new_coord, state.clone());
            }
        } else if coord.col < at_col {
            new_cells.set(*coord, state.clone());
        }
    }
    sheet.cells = new_cells;
}

fn delete_column(sheet: &mut Worksheet, at_col: u32) {
    let mut new_cells = SparseCellStore::new();
    for (coord, state) in sheet.cells.iter() {
        if coord.col > at_col {
            if let Ok(new_coord) = CellCoord::new(coord.row, coord.col - 1) {
                new_cells.set(new_coord, state.clone());
            }
        } else if coord.col < at_col {
            new_cells.set(*coord, state.clone());
        }
    }
    sheet.cells = new_cells;
}

impl eframe::App for RustExelApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let title = self.window_title();
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(title));

        let actions =
            SpreadsheetAppView::new(&mut self.workbook, &mut self.ui_state, &self.shortcuts)
                .show(ui);

        self.handle_actions(actions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> RustExelApp {
        let mut workbook = Workbook::with_default_sheet();
        let sheet = workbook.active_sheet_mut();
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Item".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Text("Price".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Apple".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 2), CellValue::Number(2.5));
        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Text("Orange".into()));
        sheet.set_cell_value(CellCoord::must_new(3, 2), CellValue::Number(3.5));
        sheet.set_cell_formula(
            CellCoord::must_new(4, 2),
            "=SUM(B2:B3)".into(),
            CellValue::Number(6.0),
        );

        let ui_state = SpreadsheetUiState::default();
        let shortcuts = ShortcutRegistry::default_for_current_platform();
        let command_stack = CommandStack::new();

        RustExelApp {
            workbook,
            ui_state,
            shortcuts,
            command_stack,
            file_path: None,
        }
    }

    #[test]
    fn test_save_and_reopen_xlsx_regression() {
        let mut app = create_test_app();
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("sales.xlsx");

        // Save to XLSX
        let save_res = app.save_to_path(test_file.clone());
        assert!(save_res.is_ok(), "XLSX saving should succeed");
        assert_eq!(app.file_path, Some(test_file.clone()));
        assert!(test_file.exists(), "Saved XLSX file must exist on disk");

        // Verify window title
        assert_eq!(app.window_title(), "Rust EXEL - sales.xlsx");

        // Open XLSX into a new app instance
        let mut reopened_app = create_test_app();
        reopened_app.workbook = Workbook::with_default_sheet(); // clear
        let open_res = reopened_app.load_from_path(test_file);
        assert!(open_res.is_ok(), "XLSX reopening should succeed");

        let rt_sheet = reopened_app.workbook.active_sheet();
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::must_new(1, 1)),
            CellValue::Text("Item".into())
        );
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::must_new(2, 2)),
            CellValue::Number(2.5)
        );
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::must_new(3, 2)),
            CellValue::Number(3.5)
        );
    }

    #[test]
    fn test_save_and_reopen_csv_regression() {
        let mut app = create_test_app();
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("data.csv");

        let save_res = app.save_to_path(test_file.clone());
        assert!(save_res.is_ok(), "CSV saving should succeed");
        assert!(test_file.exists());

        let mut reopened_app = create_test_app();
        let open_res = reopened_app.load_from_path(test_file);
        assert!(open_res.is_ok(), "CSV reopening should succeed");

        let rt_sheet = reopened_app.workbook.active_sheet();
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::must_new(1, 1)),
            CellValue::Text("Item".into())
        );
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::must_new(2, 2)),
            CellValue::Number(2.5)
        );
    }

    #[test]
    fn test_save_and_reopen_tsv_regression() {
        let mut app = create_test_app();
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("data.tsv");

        let save_res = app.save_to_path(test_file.clone());
        assert!(save_res.is_ok(), "TSV saving should succeed");
        assert!(test_file.exists());

        let mut reopened_app = create_test_app();
        let open_res = reopened_app.load_from_path(test_file);
        assert!(open_res.is_ok(), "TSV reopening should succeed");

        let rt_sheet = reopened_app.workbook.active_sheet();
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::must_new(1, 1)),
            CellValue::Text("Item".into())
        );
    }

    #[test]
    fn test_save_and_reopen_json_regression() {
        let mut app = create_test_app();
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("records.json");

        let save_res = app.save_to_path(test_file.clone());
        assert!(save_res.is_ok(), "JSON saving should succeed");
        assert!(test_file.exists());

        let mut reopened_app = create_test_app();
        let open_res = reopened_app.load_from_path(test_file);
        assert!(open_res.is_ok(), "JSON reopening should succeed");

        let rt_sheet = reopened_app.workbook.active_sheet();
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::must_new(1, 1)),
            CellValue::Text("Item".into())
        );
    }

    #[test]
    fn test_unsupported_file_extension_error() {
        let mut app = create_test_app();
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("report.docx");

        let save_res = app.save_to_path(test_file.clone());
        assert!(save_res.is_err(), "Saving to .docx should fail");
        let err = save_res.unwrap_err();
        assert!(err.contains("Unsupported file extension '.docx'"));

        let open_res = app.load_from_path(test_file);
        assert!(open_res.is_err(), "Loading from .docx should fail");
    }

    #[test]
    fn test_insert_and_delete_row_column() {
        let mut app = create_test_app();

        // Initial check: Row 2 Col 1 is Apple
        assert_eq!(
            app.workbook
                .active_sheet()
                .get_cell_value(CellCoord::must_new(2, 1)),
            CellValue::Text("Apple".into())
        );

        // Insert row at 2
        app.handle_actions(vec![SpreadsheetAction::InsertRow(2)]);
        // Row 2 is now empty, Row 3 is Apple
        assert_eq!(
            app.workbook
                .active_sheet()
                .get_cell_value(CellCoord::must_new(2, 1)),
            CellValue::Empty
        );
        assert_eq!(
            app.workbook
                .active_sheet()
                .get_cell_value(CellCoord::must_new(3, 1)),
            CellValue::Text("Apple".into())
        );

        // Delete row at 2
        app.handle_actions(vec![SpreadsheetAction::DeleteRow(2)]);
        // Row 2 is Apple again
        assert_eq!(
            app.workbook
                .active_sheet()
                .get_cell_value(CellCoord::must_new(2, 1)),
            CellValue::Text("Apple".into())
        );

        // Insert column at 1
        app.handle_actions(vec![SpreadsheetAction::InsertColumn(1)]);
        assert_eq!(
            app.workbook
                .active_sheet()
                .get_cell_value(CellCoord::must_new(2, 1)),
            CellValue::Empty
        );
        assert_eq!(
            app.workbook
                .active_sheet()
                .get_cell_value(CellCoord::must_new(2, 2)),
            CellValue::Text("Apple".into())
        );

        // Delete column at 1
        app.handle_actions(vec![SpreadsheetAction::DeleteColumn(1)]);
        assert_eq!(
            app.workbook
                .active_sheet()
                .get_cell_value(CellCoord::must_new(2, 1)),
            CellValue::Text("Apple".into())
        );
    }

    #[test]
    fn test_multi_sheet_actions() {
        let mut app = create_test_app();
        assert_eq!(app.workbook.sheet_count(), 1);

        // Add sheet
        app.handle_actions(vec![SpreadsheetAction::AddSheet]);
        assert_eq!(app.workbook.sheet_count(), 2);
        assert_eq!(app.workbook.active_sheet().name, "Sheet2");

        // Rename sheet
        let s2_id = app.workbook.active_sheet().id.clone();
        app.handle_actions(vec![SpreadsheetAction::RenameSheet {
            id: s2_id,
            new_name: "Summary".into(),
        }]);
        assert_eq!(app.workbook.active_sheet().name, "Summary");

        // Switch sheet back to Sheet1
        let s1_id = app.workbook.sheets[0].id.clone();
        app.handle_actions(vec![SpreadsheetAction::SelectSheet(s1_id)]);
        assert_eq!(app.workbook.active_sheet().name, "Sheet1");
    }
}
