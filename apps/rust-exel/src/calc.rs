use exel_calc::CalcEngine;
use exel_cell::CellValue;
use exel_workbook::{Workbook, Worksheet};

/// Evaluates all formula cells across the entire workbook using the shared calculation engine.
#[allow(dead_code)]
pub fn recalculate_workbook(wb: &mut Workbook) {
    let mut engine = CalcEngine::new();
    engine.recalculate_workbook(wb);
}

/// Evaluates all formula cells in a single worksheet using the shared calculation engine.
#[allow(dead_code)]
pub fn recalculate_sheet(sheet: &mut Worksheet) {
    let mut engine = CalcEngine::new();
    engine.recalculate_sheet_isolated(sheet);
}

/// Evaluates an individual formula string against worksheet cell values using the unified CalcEngine.
#[allow(dead_code)]
pub fn evaluate_formula(sheet: &Worksheet, formula: &str) -> CellValue {
    let engine = CalcEngine::new();
    engine.evaluate_worksheet_formula(sheet, formula)
}
