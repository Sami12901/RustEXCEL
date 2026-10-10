use crate::calc::recalculate_sheet;
use crate::commands::{Command, CommandStack};
use exel_cell::{CellCoord, CellState, CellValue};
use exel_core::{MAX_COLS, MAX_ROWS};
use exel_shortcuts::ShortcutRegistry;
use exel_ui::{SpreadsheetAction, SpreadsheetAppView, SpreadsheetUiState};
use exel_workbook::Workbook;
use std::fs::File;
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
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Text("Q1 Sales".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 3), CellValue::Text("Q2 Sales".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 4), CellValue::Text("Total".into()));

        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Widgets".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 2), CellValue::Number(12500.0));
        sheet.set_cell_value(CellCoord::must_new(2, 3), CellValue::Number(14300.0));
        sheet.set_cell_formula(CellCoord::must_new(2, 4), "=SUM(B2:C2)".into(), CellValue::Number(26800.0));

        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Text("Gadgets".into()));
        sheet.set_cell_value(CellCoord::must_new(3, 2), CellValue::Number(8900.0));
        sheet.set_cell_value(CellCoord::must_new(3, 3), CellValue::Number(11200.0));
        sheet.set_cell_formula(CellCoord::must_new(3, 4), "=SUM(B3:C3)".into(), CellValue::Number(20100.0));

        sheet.set_cell_value(CellCoord::must_new(4, 1), CellValue::Text("Devices".into()));
        sheet.set_cell_value(CellCoord::must_new(4, 2), CellValue::Number(21400.0));
        sheet.set_cell_value(CellCoord::must_new(4, 3), CellValue::Number(19800.0));
        sheet.set_cell_formula(CellCoord::must_new(4, 4), "=SUM(B4:C4)".into(), CellValue::Number(41200.0));

        sheet.set_cell_value(CellCoord::must_new(5, 1), CellValue::Text("Total".into()));
        sheet.set_cell_formula(CellCoord::must_new(5, 2), "=SUM(B2:B4)".into(), CellValue::Number(42800.0));
        sheet.set_cell_formula(CellCoord::must_new(5, 3), "=SUM(C2:C4)".into(), CellValue::Number(45300.0));
        sheet.set_cell_formula(CellCoord::must_new(5, 4), "=SUM(D2:D4)".into(), CellValue::Number(88100.0));

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
                    recalculate_sheet(sheet);
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
                    recalculate_sheet(sheet);
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
                    recalculate_sheet(sheet);
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
                    recalculate_sheet(sheet);
                    self.ui_state.select_cell(target_coord, sheet.get_cell(target_coord).unwrap().formula.clone().unwrap());
                }
                SpreadsheetAction::AutoFill { source, target } => {
                    let sheet = self.workbook.active_sheet_mut();
                    // Copy pattern from source to target
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
                    recalculate_sheet(sheet);
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
                    }
                }
                SpreadsheetAction::RenameSheet { id, new_name } => {
                    let _ = self.workbook.rename_sheet(&id, &new_name);
                }
                SpreadsheetAction::DeleteSheet(id) => {
                    let _ = self.workbook.remove_sheet(&id);
                }
                SpreadsheetAction::ResizeColumn { col, width } => {
                    self.workbook.active_sheet_mut().set_col_width(col, width);
                }
                SpreadsheetAction::ResizeRow { row, height } => {
                    self.workbook.active_sheet_mut().set_row_height(row, height);
                }
                SpreadsheetAction::Undo => {
                    let sheet = self.workbook.active_sheet_mut();
                    self.command_stack.undo(sheet);
                    recalculate_sheet(sheet);
                }
                SpreadsheetAction::Redo => {
                    let sheet = self.workbook.active_sheet_mut();
                    self.command_stack.redo(sheet);
                    recalculate_sheet(sheet);
                }
                SpreadsheetAction::Cut => {
                    self.copy_selection_to_clipboard();
                    let sheet = self.workbook.active_sheet_mut();
                    for r in self.ui_state.selection.start.row..=self.ui_state.selection.end.row {
                        for c in self.ui_state.selection.start.col..=self.ui_state.selection.end.col {
                            if let Ok(coord) = CellCoord::new(r, c) {
                                sheet.cells.remove(coord);
                            }
                        }
                    }
                }
                SpreadsheetAction::Copy => {
                    self.copy_selection_to_clipboard();
                }
                SpreadsheetAction::Paste => {
                    self.paste_from_clipboard();
                }
                SpreadsheetAction::Recalculate => {
                    let sheet = self.workbook.active_sheet_mut();
                    recalculate_sheet(sheet);
                }
                SpreadsheetAction::NewFile => {
                    self.workbook = Workbook::with_default_sheet();
                    self.command_stack.clear();
                    self.file_path = None;
                }
                SpreadsheetAction::OpenFile => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Spreadsheet", &["csv", "json", "xlsx"])
                        .pick_file()
                    {
                        self.load_from_path(path);
                    }
                }
                SpreadsheetAction::SaveFile => {
                    if let Some(path) = self.file_path.clone() {
                        self.save_to_path(path);
                    } else if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .add_filter("JSON Workbook", &["json"])
                        .save_file()
                    {
                        self.save_to_path(path);
                    }
                }
                SpreadsheetAction::SaveAsFile => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .add_filter("JSON Workbook", &["json"])
                        .save_file()
                    {
                        self.save_to_path(path);
                    }
                }
                SpreadsheetAction::ExportCsv => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .save_file()
                    {
                        self.export_csv(path);
                    }
                }
                SpreadsheetAction::ExportXlsx => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Excel XLSX", &["xlsx"])
                        .save_file()
                    {
                        self.export_csv(path); // Fallback to saving data
                    }
                }
            }
        }
    }

    fn copy_selection_to_clipboard(&self) {
        let sel = self.ui_state.selection;
        let sheet = self.workbook.active_sheet();

        let mut tsv = String::new();
        for r in sel.start.row..=sel.end.row {
            let mut row_parts = Vec::new();
            for c in sel.start.col..=sel.end.col {
                if let Ok(coord) = CellCoord::new(r, c) {
                    let val = sheet.get_cell_value(coord).to_display_string();
                    row_parts.push(val);
                }
            }
            tsv.push_str(&row_parts.join("\t"));
            tsv.push('\n');
        }

        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(tsv);
        }
    }

    fn paste_from_clipboard(&mut self) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                let sheet = self.workbook.active_sheet_mut();
                let start = self.ui_state.active_cell;

                for (r_offset, line) in text.lines().enumerate() {
                    let r = start.row + (r_offset as u32);
                    if r > MAX_ROWS {
                        break;
                    }
                    for (c_offset, part) in line.split('\t').enumerate() {
                        let c = start.col + (c_offset as u32);
                        if c > MAX_COLS {
                            break;
                        }
                        if let Ok(coord) = CellCoord::new(r, c) {
                            let trimmed = part.trim();
                            if let Ok(n) = trimmed.parse::<f64>() {
                                sheet.set_cell_value(coord, CellValue::Number(n));
                            } else if trimmed.starts_with('=') {
                                sheet.set_cell_formula(coord, trimmed.to_string(), CellValue::Empty);
                            } else {
                                sheet.set_cell_value(coord, CellValue::Text(part.to_string()));
                            }
                        }
                    }
                }
                recalculate_sheet(sheet);
            }
        }
    }

    fn load_from_path(&mut self, path: PathBuf) {
        if let Ok(file) = File::open(&path) {
            if path.extension().and_then(|s| s.to_str()) == Some("csv") {
                let mut reader = csv::Reader::from_reader(file);
                let mut sheet = exel_workbook::Worksheet::new(
                    exel_core::SheetId::new("sheet_1"),
                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("Data"),
                );

                for (r_idx, result) in reader.records().enumerate() {
                    let row = (r_idx as u32) + 1;
                    if let Ok(record) = result {
                        for (c_idx, field) in record.iter().enumerate() {
                            let col = (c_idx as u32) + 1;
                            if let Ok(coord) = CellCoord::new(row, col) {
                                if let Ok(n) = field.trim().parse::<f64>() {
                                    sheet.set_cell_value(coord, CellValue::Number(n));
                                } else {
                                    sheet.set_cell_value(coord, CellValue::Text(field.to_string()));
                                }
                            }
                        }
                    }
                }

                self.workbook.sheets = vec![sheet];
                self.workbook.active_sheet_index = 0;
                self.file_path = Some(path);
            }
        }
    }

    fn save_to_path(&mut self, path: PathBuf) {
        self.export_csv(path.clone());
        self.file_path = Some(path);
    }

    fn export_csv(&self, path: PathBuf) {
        if let Ok(file) = File::create(path) {
            let mut writer = csv::Writer::from_writer(file);
            let sheet = self.workbook.active_sheet();

            if let Some(range) = sheet.cells.used_range() {
                for r in 1..=range.end.row {
                    let mut record = Vec::new();
                    for c in 1..=range.end.col {
                        if let Ok(coord) = CellCoord::new(r, c) {
                            record.push(sheet.get_cell_value(coord).to_display_string());
                        }
                    }
                    let _ = writer.write_record(&record);
                }
                let _ = writer.flush();
            }
        }
    }
}

impl eframe::App for RustExelApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let actions = SpreadsheetAppView::new(
            &mut self.workbook,
            &mut self.ui_state,
            &self.shortcuts,
        )
        .show(ui);

        self.handle_actions(actions);
    }
}
