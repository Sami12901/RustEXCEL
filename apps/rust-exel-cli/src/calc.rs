use exel_calc::CalcEngine;
use exel_cell::CellValue;
use exel_workbook::{Workbook, Worksheet};

/// Recalculates all formulas across all sheets in the workbook using the unified CalcEngine.
pub fn recalculate_workbook(wb: &mut Workbook) {
    let mut engine = CalcEngine::new();
    engine.recalculate_workbook(wb);
}

/// Evaluates all formula cells in a single worksheet using the unified CalcEngine.
#[allow(dead_code)]
pub fn recalculate_sheet(sheet: &mut Worksheet) {
    let mut engine = CalcEngine::new();
    engine.recalculate_sheet_isolated(sheet);
}

/// Evaluates an individual formula string against worksheet cell values using the unified CalcEngine.
pub fn evaluate_formula(sheet: &Worksheet, formula: &str) -> CellValue {
    let engine = CalcEngine::new();
    engine.evaluate_worksheet_formula(sheet, formula)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::{CellCoord, CellValue};

    #[test]
    fn test_unified_cli_recalculate_workbook() {
        let mut wb = Workbook::with_default_sheet();
        let sheet = wb.active_sheet_mut();
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Number(5.0));
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Number(15.0));
        sheet.set_cell_formula(
            CellCoord::must_new(1, 3),
            "=AVERAGE(A1:B1)".to_string(),
            CellValue::Empty,
        );

        recalculate_workbook(&mut wb);
        assert_eq!(
            wb.active_sheet().get_cell_value(CellCoord::must_new(1, 3)),
            CellValue::Number(10.0)
        );
    }
}
