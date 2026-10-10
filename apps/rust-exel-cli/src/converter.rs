use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use exel_cell::{CellCoord, CellValue};
use exel_core::SheetId;
use exel_workbook::{Workbook, Worksheet};
use std::fs::File;
use std::path::Path;

/// Reads a CSV file into a single-sheet Workbook.
pub fn read_csv(path: &Path) -> Result<Workbook> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open CSV file: {}", path.display()))?;
    let mut reader = csv::Reader::from_reader(file);

    let sheet_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Sheet1");
    let mut sheet = Worksheet::new(SheetId::new("sheet_1"), sheet_name);

    for (r_idx, record) in reader.records().enumerate() {
        let row = (r_idx as u32) + 1;
        let record = record?;
        for (c_idx, field) in record.iter().enumerate() {
            let col = (c_idx as u32) + 1;
            if let Ok(coord) = CellCoord::new(row, col) {
                let trimmed = field.trim();
                if let Ok(n) = trimmed.parse::<f64>() {
                    sheet.set_cell_value(coord, CellValue::Number(n));
                } else if trimmed.starts_with('=') {
                    sheet.set_cell_formula(coord, trimmed.to_string(), CellValue::Empty);
                } else if trimmed.eq_ignore_ascii_case("true") {
                    sheet.set_cell_value(coord, CellValue::Boolean(true));
                } else if trimmed.eq_ignore_ascii_case("false") {
                    sheet.set_cell_value(coord, CellValue::Boolean(false));
                } else if !trimmed.is_empty() {
                    sheet.set_cell_value(coord, CellValue::Text(field.to_string()));
                }
            }
        }
    }

    let mut wb = Workbook::new();
    wb.sheets.push(sheet);
    Ok(wb)
}

/// Reads an Excel XLSX/XLS workbook using Calamine.
pub fn read_xlsx(path: &Path) -> Result<Workbook> {
    let mut excel = open_workbook_auto(path)
        .with_context(|| format!("Failed to open Excel workbook: {}", path.display()))?;

    let mut wb = Workbook::new();

    for (s_idx, sheet_name) in excel.sheet_names().to_owned().iter().enumerate() {
        let sheet_id = SheetId::new(format!("sheet_{}", s_idx + 1));
        let mut worksheet = Worksheet::new(sheet_id, sheet_name.as_str());

        if let Ok(range) = excel.worksheet_range(sheet_name) {
            for (r_idx, row_data) in range.rows().enumerate() {
                let row = (r_idx as u32) + 1;
                for (c_idx, cell_data) in row_data.iter().enumerate() {
                    let col = (c_idx as u32) + 1;
                    if let Ok(coord) = CellCoord::new(row, col) {
                        let cell_value = match cell_data {
                            Data::Int(i) => CellValue::Number(*i as f64),
                            Data::Float(f) => CellValue::Number(*f),
                            Data::String(s) => CellValue::Text(s.clone()),
                            Data::Bool(b) => CellValue::Boolean(*b),
                            Data::Empty => CellValue::Empty,
                            Data::Error(err) => CellValue::Text(format!("{:?}", err)),
                            Data::DateTime(dt) => CellValue::Number(dt.as_f64()),
                            _ => CellValue::Empty,
                        };
                        if !cell_value.is_empty() {
                            worksheet.set_cell_value(coord, cell_value);
                        }
                    }
                }
            }
        }

        wb.sheets.push(worksheet);
    }

    if wb.sheets.is_empty() {
        wb = Workbook::with_default_sheet();
    }

    Ok(wb)
}

/// Writes the active sheet of a Workbook to CSV.
pub fn write_csv(sheet: &Worksheet, path: &Path) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("Failed to create CSV file: {}", path.display()))?;
    let mut writer = csv::Writer::from_writer(file);

    if let Some(range) = sheet.cells.used_range() {
        for r in 1..=range.end.row {
            let mut record = Vec::new();
            for c in 1..=range.end.col {
                if let Ok(coord) = CellCoord::new(r, c) {
                    record.push(sheet.get_cell_value(coord).to_display_string());
                } else {
                    record.push(String::new());
                }
            }
            writer.write_record(&record)?;
        }
        writer.flush()?;
    }

    Ok(())
}

/// Writes all worksheets in a Workbook to a new XLSX file using rust_xlsxwriter.
pub fn write_xlsx(wb: &Workbook, path: &Path) -> Result<()> {
    let mut excel_wb = rust_xlsxwriter::Workbook::new();

    for sheet in &wb.sheets {
        let excel_ws = excel_wb.add_worksheet();
        excel_ws.set_name(&sheet.name)?;

        for (coord, state) in sheet.cells.iter() {
            let row = coord.row - 1; // 0-based for rust_xlsxwriter
            let col = (coord.col - 1) as u16;

            if let Some(formula) = &state.formula {
                let trimmed = formula.trim().strip_prefix('=').unwrap_or(formula);
                excel_ws.write_formula(row, col, trimmed)?;
            } else {
                match &state.value {
                    CellValue::Number(n) => {
                        excel_ws.write_number(row, col, *n)?;
                    }
                    CellValue::Text(s) => {
                        excel_ws.write_string(row, col, s)?;
                    }
                    CellValue::Boolean(b) => {
                        excel_ws.write_boolean(row, col, *b)?;
                    }
                    CellValue::Error(err) => {
                        excel_ws.write_string(row, col, err.as_str())?;
                    }
                    _ => {}
                }
            }
        }
    }

    excel_wb.save(path)
        .with_context(|| format!("Failed to save XLSX workbook: {}", path.display()))?;

    Ok(())
}
