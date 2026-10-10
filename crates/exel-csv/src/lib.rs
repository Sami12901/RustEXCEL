use chrono::{NaiveDate, NaiveDateTime};
use csv::{ReaderBuilder, WriterBuilder};
use exel_cell::{CellCoord, CellRange, CellState, CellValue, SheetId};
use exel_core::CoreError;
use exel_io::{save_atomic, IoError, SpreadsheetReader, SpreadsheetWriter};
use exel_workbook::{Workbook, Worksheet};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CsvError {
    #[error("CSV parse error: {0}")]
    Csv(#[from] csv::Error),
    #[error("JSON parse/serialize error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Persistence error: {0}")]
    Persistence(#[from] IoError),
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("Import/Export error: {0}")]
    Custom(String),
}

/// Sniffs the delimiter (',', '\t', ';', '|') with the highest consistency across lines.
pub fn sniff_delimiter(sample: &str) -> u8 {
    let candidates = *b",\t;|";
    let lines: Vec<&str> = sample
        .lines()
        .take(25)
        .filter(|l| !l.trim().is_empty())
        .collect();

    if lines.is_empty() {
        return b',';
    }

    let mut best_delim = b',';
    let mut best_score = 0.0f64;

    for &cand in &candidates {
        let mut counts = Vec::new();
        for line in &lines {
            let count = line.bytes().filter(|&b| b == cand).count();
            counts.push(count);
        }

        let total: usize = counts.iter().sum();
        if total == 0 {
            continue;
        }

        let avg = total as f64 / counts.len() as f64;
        let variance: f64 = counts
            .iter()
            .map(|&c| (c as f64 - avg).powi(2))
            .sum::<f64>()
            / counts.len() as f64;

        // Score favors higher counts and lower variance (more uniform columns)
        let score = (avg * 10.0) / (1.0 + variance);
        if score > best_score {
            best_score = score;
            best_delim = cand;
        }
    }

    best_delim
}

/// Automatically infers a typed CellValue from a raw text string.
pub fn infer_cell_value(s: &str) -> (CellValue, Option<String>) {
    let trimmed = s.trim();

    if trimmed.is_empty() {
        return (CellValue::Empty, None);
    }

    // Formula check
    if trimmed.starts_with('=') && trimmed.len() > 1 {
        let formula = trimmed[1..].to_string();
        return (CellValue::Empty, Some(formula));
    }

    // Boolean check
    if trimmed.eq_ignore_ascii_case("true") {
        return (CellValue::Boolean(true), None);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return (CellValue::Boolean(false), None);
    }

    // Numeric check (integer, float, scientific notation)
    if let Ok(num) = trimmed.parse::<f64>() {
        return (CellValue::Number(num), None);
    }

    // ISO Date: YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        return (CellValue::DateTime(dt), None);
    }

    // ISO DateTime: YYYY-MM-DD HH:MM:SS
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return (CellValue::DateTime(dt), None);
    }

    // ISO DateTime with 'T': YYYY-MM-DDTHH:MM:SS
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        return (CellValue::DateTime(dt), None);
    }

    // Fallback: Text
    (CellValue::Text(s.to_string()), None)
}

#[derive(Debug, Clone)]
pub struct CsvImportOptions {
    pub delimiter: Option<u8>,
    pub infer_types: bool,
    pub start_coord: CellCoord,
    pub sheet_name: Option<String>,
}

impl Default for CsvImportOptions {
    fn default() -> Self {
        Self {
            delimiter: None,
            infer_types: true,
            start_coord: CellCoord::must_new(1, 1),
            sheet_name: None,
        }
    }
}

/// Imports CSV/TSV from a reader into a Worksheet.
pub fn import_csv_reader<R: Read>(
    mut reader: R,
    options: &CsvImportOptions,
) -> Result<Worksheet, CsvError> {
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;

    let delimiter = options
        .delimiter
        .unwrap_or_else(|| sniff_delimiter(&buffer));

    let sheet_name = options
        .sheet_name
        .clone()
        .unwrap_or_else(|| "ImportedData".to_string());
    let mut sheet = Worksheet::new(SheetId::new("sheet_1"), sheet_name);

    let mut csv_rdr = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(buffer.as_bytes());

    for (row_idx, result) in (options.start_coord.row..).zip(csv_rdr.records()) {
        let record = result?;
        for (col_offset, field) in record.iter().enumerate() {
            let col_idx = options.start_coord.col + col_offset as u32;
            if let Ok(coord) = CellCoord::new(row_idx, col_idx) {
                let (val, formula) = if options.infer_types {
                    infer_cell_value(field)
                } else if field.is_empty() {
                    (CellValue::Empty, None)
                } else {
                    (CellValue::Text(field.to_string()), None)
                };

                let state = CellState {
                    value: val,
                    formula,
                    style_id: 0,
                    comment: None,
                };
                sheet.set_cell(coord, state);
            }
        }
    }

    Ok(sheet)
}

/// Imports CSV text into a full Workbook.
pub fn import_csv_str(csv_str: &str, options: &CsvImportOptions) -> Result<Workbook, CsvError> {
    let sheet = import_csv_reader(csv_str.as_bytes(), options)?;
    let mut wb = Workbook::new();
    wb.sheets.push(sheet);
    Ok(wb)
}

/// Imports a CSV file from disk into a Workbook.
pub fn import_csv_file<P: AsRef<Path>>(
    path: P,
    options: &CsvImportOptions,
) -> Result<Workbook, CsvError> {
    let file = std::fs::File::open(path.as_ref())?;
    let mut opts = options.clone();
    if opts.sheet_name.is_none() {
        if let Some(stem) = path.as_ref().file_stem().and_then(|s| s.to_str()) {
            opts.sheet_name = Some(stem.to_string());
        }
    }
    let sheet = import_csv_reader(file, &opts)?;
    let mut wb = Workbook::new();
    wb.sheets.push(sheet);
    Ok(wb)
}

#[derive(Debug, Clone)]
pub struct CsvExportOptions {
    pub delimiter: u8,
    pub sheet_index: usize,
    pub range: Option<CellRange>,
}

impl Default for CsvExportOptions {
    fn default() -> Self {
        Self {
            delimiter: b',',
            sheet_index: 0,
            range: None,
        }
    }
}

/// Exports a worksheet's data to a CSV/TSV writer.
pub fn export_csv_writer<W: Write>(
    wb: &Workbook,
    writer: W,
    options: &CsvExportOptions,
) -> Result<(), CsvError> {
    let sheet = wb
        .sheets
        .get(options.sheet_index)
        .ok_or_else(|| CsvError::Custom("Sheet index out of bounds".into()))?;

    let range = options
        .range
        .or_else(|| sheet.cells.used_range())
        .unwrap_or_else(|| CellRange::single_cell(CellCoord::must_new(1, 1)));

    let mut csv_wtr = WriterBuilder::new()
        .delimiter(options.delimiter)
        .from_writer(writer);

    for r in range.start.row..=range.end.row {
        let mut row_record = Vec::new();
        for c in range.start.col..=range.end.col {
            let coord = CellCoord::must_new(r, c);
            let val = sheet.get_cell_value(coord);
            row_record.push(val.to_display_string());
        }
        csv_wtr.write_record(&row_record)?;
    }

    csv_wtr.flush()?;
    Ok(())
}

/// Exports a worksheet to a CSV string.
pub fn export_csv_string(wb: &Workbook, options: &CsvExportOptions) -> Result<String, CsvError> {
    let mut buf = Vec::new();
    export_csv_writer(wb, &mut buf, options)?;
    String::from_utf8(buf).map_err(|e| CsvError::Custom(e.to_string()))
}

/// Atomically exports a worksheet to a CSV/TSV file.
pub fn export_csv_file<P: AsRef<Path>>(
    wb: &Workbook,
    path: P,
    options: &CsvExportOptions,
) -> Result<(), CsvError> {
    let opts = options.clone();
    save_atomic(path, |file| {
        export_csv_writer(wb, file, &opts).map_err(|e| IoError::SaveFailed(e.to_string()))
    })?;
    Ok(())
}

// ============================================================================
// JSON Import & Export
// ============================================================================

/// Imports a JSON string (either tabular records `[{"a": 1, ...}]` or 2D matrix `[[...], [...]]`).
pub fn import_json_str(json_str: &str, sheet_name: &str) -> Result<Worksheet, CsvError> {
    let parsed: serde_json::Value = serde_json::from_str(json_str)?;
    let mut sheet = Worksheet::new(SheetId::new("sheet_1"), sheet_name);

    match parsed {
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return Ok(sheet);
            }

            if arr[0].is_object() {
                // Array of Objects: headers from keys of the first object
                let mut headers: Vec<String> = Vec::new();
                for obj in &arr {
                    if let Some(map) = obj.as_object() {
                        for k in map.keys() {
                            if !headers.contains(k) {
                                headers.push(k.clone());
                            }
                        }
                    }
                }

                // Write header row
                for (c_idx, h) in headers.iter().enumerate() {
                    let coord = CellCoord::must_new(1, (c_idx + 1) as u32);
                    sheet.set_cell_value(coord, CellValue::Text(h.clone()));
                }

                // Write record rows
                for (r_idx, item) in arr.iter().enumerate() {
                    let row_num = (r_idx + 2) as u32;
                    if let Some(map) = item.as_object() {
                        for (c_idx, h) in headers.iter().enumerate() {
                            let col_num = (c_idx + 1) as u32;
                            let coord = CellCoord::must_new(row_num, col_num);
                            if let Some(val) = map.get(h) {
                                let cell_val = json_val_to_cell_value(val);
                                sheet.set_cell_value(coord, cell_val);
                            }
                        }
                    }
                }
            } else if arr[0].is_array() {
                // 2D Array
                for (r_idx, row_item) in arr.iter().enumerate() {
                    let row_num = (r_idx + 1) as u32;
                    if let Some(cols) = row_item.as_array() {
                        for (c_idx, val) in cols.iter().enumerate() {
                            let col_num = (c_idx + 1) as u32;
                            let coord = CellCoord::must_new(row_num, col_num);
                            sheet.set_cell_value(coord, json_val_to_cell_value(val));
                        }
                    }
                }
            }
        }
        _ => {
            return Err(CsvError::Custom("Expected top-level JSON array".into()));
        }
    }

    Ok(sheet)
}

fn json_val_to_cell_value(val: &serde_json::Value) -> CellValue {
    match val {
        serde_json::Value::Null => CellValue::Empty,
        serde_json::Value::Bool(b) => CellValue::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                CellValue::Number(f)
            } else {
                CellValue::Empty
            }
        }
        serde_json::Value::String(s) => {
            let (v, _) = infer_cell_value(s);
            v
        }
        _ => CellValue::Text(val.to_string()),
    }
}

/// Exports a worksheet as a JSON array of records (using row 1 as property keys).
pub fn export_json_records(sheet: &Worksheet) -> Result<String, CsvError> {
    let range = match sheet.cells.used_range() {
        Some(r) => r,
        None => return Ok("[]".to_string()),
    };

    if range.row_count() < 2 {
        return Ok("[]".to_string());
    }

    // Collect headers from row 1
    let mut headers = Vec::new();
    for col in range.start.col..=range.end.col {
        let coord = CellCoord::must_new(range.start.row, col);
        let name = sheet.get_cell_value(coord).to_display_string();
        headers.push((
            col,
            if name.is_empty() {
                format!("Col{}", col)
            } else {
                name
            },
        ));
    }

    let mut records = Vec::new();
    for row in (range.start.row + 1)..=range.end.row {
        let mut map = serde_json::Map::new();
        for (col, name) in &headers {
            let coord = CellCoord::must_new(row, *col);
            let val = sheet.get_cell_value(coord);
            let json_val = match val {
                CellValue::Empty => serde_json::Value::Null,
                CellValue::Number(n) => serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
                CellValue::Text(s) => serde_json::Value::String(s),
                CellValue::Boolean(b) => serde_json::Value::Bool(b),
                CellValue::DateTime(dt) => {
                    serde_json::Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string())
                }
                CellValue::Error(e) => serde_json::Value::String(e.as_str().to_string()),
            };
            map.insert(name.clone(), json_val);
        }
        records.push(serde_json::Value::Object(map));
    }

    Ok(serde_json::to_string_pretty(&records)?)
}

// ============================================================================
// SpreadsheetReader & SpreadsheetWriter implementations
// ============================================================================

pub struct CsvHandler {
    pub delimiter: u8,
}

impl CsvHandler {
    pub fn csv() -> Self {
        Self { delimiter: b',' }
    }

    pub fn tsv() -> Self {
        Self { delimiter: b'\t' }
    }
}

impl SpreadsheetReader for CsvHandler {
    fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Workbook, IoError> {
        let opts = CsvImportOptions {
            delimiter: Some(self.delimiter),
            ..Default::default()
        };
        import_csv_file(path, &opts).map_err(|e| IoError::Serialization(e.to_string()))
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<Workbook, IoError> {
        let opts = CsvImportOptions {
            delimiter: Some(self.delimiter),
            ..Default::default()
        };
        let sheet =
            import_csv_reader(bytes, &opts).map_err(|e| IoError::Serialization(e.to_string()))?;
        let mut wb = Workbook::new();
        wb.sheets.push(sheet);
        Ok(wb)
    }
}

impl SpreadsheetWriter for CsvHandler {
    fn write_file<P: AsRef<Path>>(&self, wb: &Workbook, path: P) -> Result<(), IoError> {
        let opts = CsvExportOptions {
            delimiter: self.delimiter,
            ..Default::default()
        };
        export_csv_file(wb, path, &opts).map_err(|e| IoError::SaveFailed(e.to_string()))
    }

    fn write_bytes(&self, wb: &Workbook) -> Result<Vec<u8>, IoError> {
        let opts = CsvExportOptions {
            delimiter: self.delimiter,
            ..Default::default()
        };
        let mut buf = Vec::new();
        export_csv_writer(wb, &mut buf, &opts).map_err(|e| IoError::SaveFailed(e.to_string()))?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delimiter_sniffing() {
        let csv_sample = "name,age,city\nAlice,30,NYC\nBob,25,LA\nCharlie,35,SF\n";
        assert_eq!(sniff_delimiter(csv_sample), b',');

        let tsv_sample = "name\tage\tcity\nAlice\t30\tNYC\nBob\t25\tLA\n";
        assert_eq!(sniff_delimiter(tsv_sample), b'\t');

        let semi_sample = "id;price;qty\n1;19.99;5\n2;9.50;10\n3;100.0;1\n";
        assert_eq!(sniff_delimiter(semi_sample), b';');
    }

    #[test]
    fn test_type_inference() {
        assert_eq!(infer_cell_value("").0, CellValue::Empty);
        assert_eq!(infer_cell_value("   ").0, CellValue::Empty);
        assert_eq!(infer_cell_value("12345").0, CellValue::Number(12345.0));
        assert_eq!(infer_cell_value("-42.5").0, CellValue::Number(-42.5));
        assert_eq!(infer_cell_value("TRUE").0, CellValue::Boolean(true));
        assert_eq!(infer_cell_value("false").0, CellValue::Boolean(false));
        assert_eq!(
            infer_cell_value("Hello World").0,
            CellValue::Text("Hello World".into())
        );

        let (val, formula) = infer_cell_value("=SUM(A1:A10)");
        assert_eq!(val, CellValue::Empty);
        assert_eq!(formula, Some("SUM(A1:A10)".into()));

        if let CellValue::DateTime(dt) = infer_cell_value("2024-05-01").0 {
            assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-05-01");
        } else {
            panic!("Expected date value");
        }
    }

    #[test]
    fn test_csv_import_export_roundtrip() {
        let csv_data = "Item,Qty,Price\nApple,10,1.5\nBanana,20,0.75\nOrange,15,2\n";
        let opts = CsvImportOptions::default();
        let wb = import_csv_str(csv_data, &opts).unwrap();

        let sheet = wb.active_sheet();
        assert_eq!(
            sheet.get_cell_value(CellCoord::from_a1("A1").unwrap()),
            CellValue::Text("Item".into())
        );
        assert_eq!(
            sheet.get_cell_value(CellCoord::from_a1("B2").unwrap()),
            CellValue::Number(10.0)
        );
        assert_eq!(
            sheet.get_cell_value(CellCoord::from_a1("C2").unwrap()),
            CellValue::Number(1.5)
        );

        let export_opts = CsvExportOptions::default();
        let exported = export_csv_string(&wb, &export_opts).unwrap();
        assert!(exported.contains("Apple,10,1.5"));
        assert!(exported.contains("Banana,20,0.75"));
    }

    #[test]
    fn test_json_records_roundtrip() {
        let json_input = r#"[
            {"Name": "Alice", "Score": 95.0, "Passed": true},
            {"Name": "Bob", "Score": 82.5, "Passed": true}
        ]"#;

        let sheet = import_json_str(json_input, "Grades").unwrap();
        assert_eq!(
            sheet.get_cell_value(CellCoord::from_a1("A1").unwrap()),
            CellValue::Text("Name".into())
        );
        assert_eq!(
            sheet.get_cell_value(CellCoord::from_a1("A2").unwrap()),
            CellValue::Text("Alice".into())
        );
        assert_eq!(
            sheet.get_cell_value(CellCoord::from_a1("B2").unwrap()),
            CellValue::Boolean(true)
        );
        assert_eq!(
            sheet.get_cell_value(CellCoord::from_a1("C2").unwrap()),
            CellValue::Number(95.0)
        );

        let exported_json = export_json_records(&sheet).unwrap();
        assert!(exported_json.contains("Alice"));
        assert!(exported_json.contains("95"));
    }
}
