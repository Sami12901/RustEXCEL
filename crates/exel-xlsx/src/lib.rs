use calamine::{CellErrorType, Data, Reader, Xlsx};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use exel_cell::{CellCoord, CellError, CellState, CellValue, SheetId};
use exel_core::CoreError;
use exel_formatting::{
    excel_serial_to_datetime, BorderStyle, CellStyle, FillPattern,
    HorizontalAlignment, StyleTable, VerticalAlignment,
};
use exel_io::{save_atomic, IoError, SpreadsheetReader, SpreadsheetWriter};
use exel_workbook::{Workbook, Worksheet};
use rust_xlsxwriter::{
    Color as XlsxColor, ExcelDateTime, Format, FormatAlign, FormatBorder, FormatUnderline,
    Workbook as XlsxWorkbook,
};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Error, Debug)]
pub enum XlsxError {
    #[error("Calamine reading error: {0}")]
    Calamine(#[from] calamine::Error),
    #[error("Calamine XLSX error: {0}")]
    CalamineXlsx(#[from] calamine::XlsxError),
    #[error("Xlsx writing error: {0}")]
    XlsxWriter(#[from] rust_xlsxwriter::XlsxError),
    #[error("Zip archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Persistence error: {0}")]
    Persistence(#[from] IoError),
    #[error("Core error: {0}")]
    Core(#[from] CoreError),
    #[error("Spreadsheet error: {0}")]
    Custom(String),
}

/// Security scan result identifying whether an XLSX / XLSM workbook contains embedded VBA macros.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroScanResult {
    pub has_macros: bool,
    pub macro_parts: Vec<String>,
    pub warning: Option<String>,
    pub is_safe_to_view: bool,
}

impl MacroScanResult {
    pub fn clean() -> Self {
        Self {
            has_macros: false,
            macro_parts: Vec::new(),
            warning: None,
            is_safe_to_view: true,
        }
    }

    pub fn with_detected_macros(parts: Vec<String>) -> Self {
        let msg = format!(
            "Security Alert: This workbook contains {} embedded VBA macro part(s) (including: {}). \
            Execution is blocked per Rust EXEL safety policy. Data view is sanitized and safe.",
            parts.len(),
            parts.join(", ")
        );
        Self {
            has_macros: true,
            macro_parts: parts,
            warning: Some(msg),
            is_safe_to_view: true,
        }
    }
}

/// Scans a zip-based spreadsheet archive for embedded VBA macro streams and ActiveX binaries.
/// Never executes any byte of code.
pub fn scan_macros<R: Read + Seek>(reader: R) -> Result<MacroScanResult, XlsxError> {
    let mut zip = ZipArchive::new(reader)?;
    let mut detected = Vec::new();

    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        let name = file.name().to_string();
        let lower = name.to_ascii_lowercase();

        if lower == "xl/vbaproject.bin"
            || lower == "xl/vbadata.xml"
            || lower.starts_with("xl/vba")
            || lower.contains("vbaproject")
            || (lower.starts_with("xl/") && lower.ends_with(".bin"))
            || lower.contains("customui")
        {
            detected.push(name);
        }
    }

    if detected.is_empty() {
        Ok(MacroScanResult::clean())
    } else {
        Ok(MacroScanResult::with_detected_macros(detected))
    }
}

/// Scans an on-disk spreadsheet file for macros.
pub fn scan_macros_file<P: AsRef<Path>>(path: P) -> Result<MacroScanResult, XlsxError> {
    let file = File::open(path.as_ref())?;
    scan_macros(file)
}

// ============================================================================
// XLSX Reader (calamine)
// ============================================================================

/// Reads an XLSX / XLSM file into a multi-sheet Workbook.
pub fn read_xlsx_reader<R: Read + Seek>(reader: R) -> Result<Workbook, XlsxError> {
    let mut excel: Xlsx<_> = Xlsx::new(reader)?;
    let mut wb = Workbook::new();

    let sheet_names = excel.sheet_names();

    for (sheet_idx, name) in sheet_names.into_iter().enumerate() {
        let sheet_id = SheetId::new(format!("sheet_{}", sheet_idx + 1));
        let mut worksheet = Worksheet::new(sheet_id, &name);

        // Try reading cell formulas
        let formula_map = if let Ok(formula_range) = excel.worksheet_formula(&name) {
            let (start_r, start_c) = formula_range.start().unwrap_or((0, 0));
            let mut map = std::collections::HashMap::new();
            for (r, c, f) in formula_range.used_cells() {
                if !f.is_empty() {
                    map.insert((start_r + r as u32 + 1, start_c + c as u32 + 1), f.to_string());
                }
            }
            Some(map)
        } else {
            None
        };

        // Read cell values
        if let Ok(range) = excel.worksheet_range(&name) {
            let (start_r, start_c) = range.start().unwrap_or((0, 0));
            for (r, c, data) in range.used_cells() {
                let row = start_r + r as u32 + 1;
                let col = start_c + c as u32 + 1;

                if let Ok(coord) = CellCoord::new(row, col) {
                    let cell_value = match data {
                        Data::Empty => CellValue::Empty,
                        Data::String(s) => CellValue::Text(s.clone()),
                        Data::Float(f) => CellValue::Number(*f),
                        Data::Int(i) => CellValue::Number(*i as f64),
                        Data::Bool(b) => CellValue::Boolean(*b),
                        Data::DateTime(dt) => {
                            let f = dt.as_f64();
                            if let Some(ndt) = excel_serial_to_datetime(f) {
                                CellValue::DateTime(ndt)
                            } else {
                                CellValue::Number(f)
                            }
                        }
                        Data::DateTimeIso(iso_str) => {
                            if let Ok(ndt) = NaiveDateTime::parse_from_str(iso_str, "%Y-%m-%d %H:%M:%S") {
                                CellValue::DateTime(ndt)
                            } else if let Ok(nd) = NaiveDate::parse_from_str(iso_str, "%Y-%m-%d") {
                                CellValue::DateTime(nd.and_hms_opt(0, 0, 0).unwrap())
                            } else {
                                CellValue::Text(iso_str.clone())
                            }
                        }
                        Data::DurationIso(d) => CellValue::Text(d.clone()),
                        Data::Error(err) => CellValue::Error(map_calamine_error(err)),
                    };

                    let formula = formula_map
                        .as_ref()
                        .and_then(|m| m.get(&(row, col)).cloned());

                    let state = CellState {
                        value: cell_value,
                        formula,
                        style_id: 0,
                        comment: None,
                    };

                    worksheet.set_cell(coord, state);
                }
            }
        }

        wb.sheets.push(worksheet);
    }

    if wb.sheets.is_empty() {
        wb.add_sheet("Sheet1")
            .map_err(|e| XlsxError::Custom(e.to_string()))?;
    }

    Ok(wb)
}

fn map_calamine_error(err: &CellErrorType) -> CellError {
    match err {
        CellErrorType::Div0 => CellError::Div0,
        CellErrorType::Value => CellError::Value,
        CellErrorType::Ref => CellError::Ref,
        CellErrorType::Name => CellError::Name,
        CellErrorType::Num => CellError::Num,
        CellErrorType::Null => CellError::Null,
        CellErrorType::NA => CellError::NA,
        _ => CellError::Value,
    }
}

pub fn read_xlsx_file<P: AsRef<Path>>(path: P) -> Result<Workbook, XlsxError> {
    let file = File::open(path.as_ref())?;
    read_xlsx_reader(file)
}

pub fn read_xlsx_bytes(bytes: &[u8]) -> Result<Workbook, XlsxError> {
    read_xlsx_reader(Cursor::new(bytes))
}

// ============================================================================
// XLSX Writer (rust_xlsxwriter)
// ============================================================================

/// Translates a CellStyle into a rust_xlsxwriter Format.
pub fn translate_style(style: &CellStyle) -> Format {
    let mut fmt = Format::new();

    // 1. Font
    fmt = fmt.set_font_name(&style.font.name);
    fmt = fmt.set_font_size(style.font.size as f64);
    if style.font.bold {
        fmt = fmt.set_bold();
    }
    if style.font.italic {
        fmt = fmt.set_italic();
    }
    if style.font.underline {
        fmt = fmt.set_underline(FormatUnderline::Single);
    }
    if style.font.strikethrough {
        fmt = fmt.set_font_strikethrough();
    }
    let [fr, fg, fb] = style.font.color.to_rgb_array();
    fmt = fmt.set_font_color(XlsxColor::RGB(rgb_to_u32(fr, fg, fb)));

    // 2. Fill
    if style.fill.pattern == FillPattern::Solid {
        if let Some(color) = style.fill.fg_color {
            let [br, bg, bb] = color.to_rgb_array();
            fmt = fmt.set_background_color(XlsxColor::RGB(rgb_to_u32(br, bg, bb)));
        }
    }

    // 3. Borders
    fmt = fmt.set_border_top(map_border_style(style.border.top.style));
    let [tr, tg, tb] = style.border.top.color.to_rgb_array();
    fmt = fmt.set_border_top_color(XlsxColor::RGB(rgb_to_u32(tr, tg, tb)));

    fmt = fmt.set_border_bottom(map_border_style(style.border.bottom.style));
    let [br, bg, bb] = style.border.bottom.color.to_rgb_array();
    fmt = fmt.set_border_bottom_color(XlsxColor::RGB(rgb_to_u32(br, bg, bb)));

    fmt = fmt.set_border_left(map_border_style(style.border.left.style));
    let [lr, lg, lb] = style.border.left.color.to_rgb_array();
    fmt = fmt.set_border_left_color(XlsxColor::RGB(rgb_to_u32(lr, lg, lb)));

    fmt = fmt.set_border_right(map_border_style(style.border.right.style));
    let [rr, rg, rb] = style.border.right.color.to_rgb_array();
    fmt = fmt.set_border_right_color(XlsxColor::RGB(rgb_to_u32(rr, rg, rb)));

    // 4. Alignment
    match style.alignment.horizontal {
        HorizontalAlignment::General => {}
        HorizontalAlignment::Left => fmt = fmt.set_align(FormatAlign::Left),
        HorizontalAlignment::Center => fmt = fmt.set_align(FormatAlign::Center),
        HorizontalAlignment::Right => fmt = fmt.set_align(FormatAlign::Right),
        HorizontalAlignment::Fill => fmt = fmt.set_align(FormatAlign::Fill),
        HorizontalAlignment::Justify => fmt = fmt.set_align(FormatAlign::Justify),
        HorizontalAlignment::CenterContinuous => fmt = fmt.set_align(FormatAlign::CenterAcross),
        HorizontalAlignment::Distributed => fmt = fmt.set_align(FormatAlign::Distributed),
    }

    match style.alignment.vertical {
        VerticalAlignment::Top => fmt = fmt.set_align(FormatAlign::Top),
        VerticalAlignment::Center => fmt = fmt.set_align(FormatAlign::VerticalCenter),
        VerticalAlignment::Bottom => fmt = fmt.set_align(FormatAlign::Bottom),
        VerticalAlignment::Justify => fmt = fmt.set_align(FormatAlign::VerticalJustify),
        VerticalAlignment::Distributed => fmt = fmt.set_align(FormatAlign::VerticalDistributed),
    }

    if style.alignment.wrap_text {
        fmt = fmt.set_text_wrap();
    }
    if style.alignment.text_rotation != 0 {
        fmt = fmt.set_rotation(style.alignment.text_rotation);
    }

    // 5. NumberFormat
    let num_fmt = style.number_format.to_format_string();
    if !num_fmt.eq_ignore_ascii_case("general") && !num_fmt.is_empty() {
        fmt = fmt.set_num_format(&num_fmt);
    }

    fmt
}

fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn map_border_style(b: BorderStyle) -> FormatBorder {
    match b {
        BorderStyle::None => FormatBorder::None,
        BorderStyle::Thin => FormatBorder::Thin,
        BorderStyle::Medium => FormatBorder::Medium,
        BorderStyle::Dashed => FormatBorder::Dashed,
        BorderStyle::Dotted => FormatBorder::Dotted,
        BorderStyle::Thick => FormatBorder::Thick,
        BorderStyle::Double => FormatBorder::Double,
        BorderStyle::Hair => FormatBorder::Hair,
        BorderStyle::MediumDashed => FormatBorder::MediumDashed,
        BorderStyle::DashDot => FormatBorder::DashDot,
        BorderStyle::MediumDashDot => FormatBorder::MediumDashDot,
        BorderStyle::DashDotDot => FormatBorder::DashDotDot,
        BorderStyle::MediumDashDotDot => FormatBorder::MediumDashDotDot,
        BorderStyle::SlantedDashDot => FormatBorder::SlantDashDot,
    }
}

/// Converts a NaiveDateTime into an ExcelDateTime.
fn to_excel_datetime(dt: &NaiveDateTime) -> Option<ExcelDateTime> {
    let date = dt.date();
    let time = dt.time();
    ExcelDateTime::from_ymd(date.year() as u16, date.month() as u8, date.day() as u8)
        .ok()?
        .and_hms(time.hour() as u16, time.minute() as u8, time.second() as f64)
        .ok()
}

/// Writes a Workbook to in-memory XLSX bytes with optional style table formatting.
pub fn write_xlsx_bytes(wb: &Workbook, styles: Option<&StyleTable>) -> Result<Vec<u8>, XlsxError> {
    let mut xlsx_wb = XlsxWorkbook::new();
    let default_style = CellStyle::default();

    for sheet in &wb.sheets {
        let ws = xlsx_wb.add_worksheet();
        let _ = ws.set_name(&sheet.name);

        // Column widths
        for (&col, &width) in &sheet.col_widths {
            if col >= 1 {
                let _ = ws.set_column_width((col - 1) as u16, (width / 8.0) as f64);
            }
        }

        // Row heights
        for (&row, &height) in &sheet.row_heights {
            if row >= 1 {
                let _ = ws.set_row_height(row - 1, (height * 0.75) as f64);
            }
        }

        // Freeze panes
        if let Some(freeze) = sheet.freeze_panes {
            if freeze.row >= 1 && freeze.col >= 1 {
                let _ = ws.set_freeze_panes(freeze.row - 1, (freeze.col - 1) as u16);
            }
        }

        // Tab color
        if let Some([r, g, b]) = sheet.tab_color {
            let _ = ws.set_tab_color(XlsxColor::RGB(rgb_to_u32(r, g, b)));
        }

        // Populate cells
        for (&coord, state) in sheet.cells.iter() {
            let r = coord.row - 1;
            let c = (coord.col - 1) as u16;

            let cell_style = styles
                .and_then(|st| st.get(state.style_id))
                .unwrap_or(&default_style);

            let fmt = translate_style(cell_style);

            if let Some(formula) = &state.formula {
                let formula_str = if formula.starts_with('=') {
                    formula.clone()
                } else {
                    format!("={formula}")
                };
                let _ = ws.write_formula_with_format(r, c, formula_str.as_str(), &fmt);
            } else {
                match &state.value {
                    CellValue::Empty => {}
                    CellValue::Number(n) => {
                        let _ = ws.write_number_with_format(r, c, *n, &fmt);
                    }
                    CellValue::Text(s) => {
                        let _ = ws.write_string_with_format(r, c, s, &fmt);
                    }
                    CellValue::Boolean(b) => {
                        let _ = ws.write_boolean_with_format(r, c, *b, &fmt);
                    }
                    CellValue::DateTime(dt) => {
                        if let Some(edt) = to_excel_datetime(dt) {
                            let _ = ws.write_datetime_with_format(r, c, edt, &fmt);
                        } else {
                            let _ = ws.write_string_with_format(r, c, &dt.to_string(), &fmt);
                        }
                    }
                    CellValue::Error(err) => {
                        let _ = ws.write_string_with_format(r, c, err.as_str(), &fmt);
                    }
                }
            }
        }
    }

    let buffer = xlsx_wb.save_to_buffer()?;
    Ok(buffer)
}

/// Atomically writes a Workbook to an on-disk XLSX file.
pub fn write_xlsx_file<P: AsRef<Path>>(
    wb: &Workbook,
    path: P,
    styles: Option<&StyleTable>,
) -> Result<(), XlsxError> {
    let bytes = write_xlsx_bytes(wb, styles)?;
    save_atomic(path, |file| {
        use std::io::Write;
        file.write_all(&bytes)?;
        Ok(())
    })?;
    Ok(())
}

// ============================================================================
// SpreadsheetReader & SpreadsheetWriter implementations
// ============================================================================

pub struct XlsxHandler<'a> {
    pub styles: Option<&'a StyleTable>,
}

impl<'a> Default for XlsxHandler<'a> {
    fn default() -> Self {
        Self { styles: None }
    }
}

impl<'a> XlsxHandler<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_styles(styles: &'a StyleTable) -> Self {
        Self {
            styles: Some(styles),
        }
    }
}

impl<'a> SpreadsheetReader for XlsxHandler<'a> {
    fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Workbook, IoError> {
        read_xlsx_file(path).map_err(|e| IoError::Serialization(e.to_string()))
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<Workbook, IoError> {
        read_xlsx_bytes(bytes).map_err(|e| IoError::Serialization(e.to_string()))
    }
}

impl<'a> SpreadsheetWriter for XlsxHandler<'a> {
    fn write_file<P: AsRef<Path>>(&self, wb: &Workbook, path: P) -> Result<(), IoError> {
        write_xlsx_file(wb, path, self.styles).map_err(|e| IoError::SaveFailed(e.to_string()))
    }

    fn write_bytes(&self, wb: &Workbook) -> Result<Vec<u8>, IoError> {
        write_xlsx_bytes(wb, self.styles).map_err(|e| IoError::SaveFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_formatting::{Font, NumberFormat};

    #[test]
    fn test_xlsx_write_and_read_roundtrip() {
        let mut wb = Workbook::new();
        let sheet_id = wb.add_sheet("Sales").unwrap();
        let sheet = wb.get_sheet_mut(&sheet_id).unwrap();

        sheet.set_cell_value(CellCoord::from_a1("A1").unwrap(), CellValue::Text("Quarter".into()));
        sheet.set_cell_value(CellCoord::from_a1("B1").unwrap(), CellValue::Text("Revenue".into()));
        sheet.set_cell_value(CellCoord::from_a1("A2").unwrap(), CellValue::Text("Q1".into()));
        sheet.set_cell_value(CellCoord::from_a1("B2").unwrap(), CellValue::Number(25000.0));
        sheet.set_cell_value(CellCoord::from_a1("A3").unwrap(), CellValue::Text("Q2".into()));
        sheet.set_cell_value(CellCoord::from_a1("B3").unwrap(), CellValue::Number(31000.5));
        sheet.set_cell_formula(
            CellCoord::from_a1("B4").unwrap(),
            "SUM(B2:B3)".to_string(),
            CellValue::Number(56000.5),
        );

        let mut styles = StyleTable::new();
        let bold_currency = CellStyle::default()
            .with_font(Font::default().with_bold(true))
            .with_number_format(NumberFormat::currency("$", 2));
        let cur_id = styles.get_or_insert(bold_currency);
        sheet.cells.get_mut(CellCoord::from_a1("B4").unwrap()).unwrap().style_id = cur_id;

        // Write to buffer
        let bytes = write_xlsx_bytes(&wb, Some(&styles)).unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..4], b"PK\x03\x04");

        // Read back with calamine
        let roundtrip_wb = read_xlsx_bytes(&bytes).unwrap();
        assert_eq!(roundtrip_wb.sheets.len(), 1);
        let rt_sheet = roundtrip_wb.active_sheet();
        assert_eq!(rt_sheet.name, "Sales");

        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::from_a1("A1").unwrap()),
            CellValue::Text("Quarter".into())
        );
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::from_a1("B2").unwrap()),
            CellValue::Number(25000.0)
        );
        assert_eq!(
            rt_sheet.get_cell_value(CellCoord::from_a1("B3").unwrap()),
            CellValue::Number(31000.5)
        );

        let b4 = rt_sheet.get_cell(CellCoord::from_a1("B4").unwrap()).unwrap();
        assert_eq!(b4.formula, Some("SUM(B2:B3)".to_string()));
    }

    #[test]
    fn test_macro_detection_on_clean_xlsx() {
        let wb = Workbook::with_default_sheet();
        let bytes = write_xlsx_bytes(&wb, None).unwrap();
        let scan = scan_macros(Cursor::new(bytes)).unwrap();
        assert!(!scan.has_macros);
        assert!(scan.is_safe_to_view);
        assert!(scan.macro_parts.is_empty());
        assert!(scan.warning.is_none());
    }

    #[test]
    fn test_macro_detection_on_xlsm_with_vba() {
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        // Synthesize an archive with a VBA project
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            let options = SimpleFileOptions::default();
            zip.start_file("xl/workbook.xml", options).unwrap();
            zip.start_file("xl/vbaProject.bin", options).unwrap();
            zip.finish().unwrap();
        }

        let scan = scan_macros(Cursor::new(buf)).unwrap();
        assert!(scan.has_macros);
        assert!(scan.is_safe_to_view);
        assert_eq!(scan.macro_parts, vec!["xl/vbaProject.bin"]);
        assert!(scan.warning.is_some());
        assert!(scan.warning.unwrap().contains("Security Alert"));
    }
}
