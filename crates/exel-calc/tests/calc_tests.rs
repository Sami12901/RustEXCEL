use exel_calc::CalcEngine;
use exel_cell::{CellError, CellState, CellValue};
use exel_core::CellCoord;
use exel_workbook::Workbook;

#[test]
fn test_excel_error_propagation_and_handling() {
    let mut wb = Workbook::with_default_sheet();
    let s_id = wb.sheets[0].id.clone();
    let mut engine = CalcEngine::new();

    // A1 = 1 / 0 -> #DIV/0!
    let a1 = CellCoord::from_a1("A1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, a1, "=1 / 0".into());
    assert_eq!(wb.sheets[0].get_cell_value(a1), CellValue::Error(CellError::Div0));

    // B1 = A1 + 10 -> propagates #DIV/0!
    let b1 = CellCoord::from_a1("B1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, b1, "=A1 + 10".into());
    assert_eq!(wb.sheets[0].get_cell_value(b1), CellValue::Error(CellError::Div0));

    // C1 = IFERROR(A1, 999) -> catches #DIV/0! and returns 999
    let c1 = CellCoord::from_a1("C1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, c1, "=IFERROR(A1, 999)".into());
    assert_eq!(wb.sheets[0].get_cell_value(c1), CellValue::Number(999.0));

    // D1 = SQRT(-5) -> #NUM!
    let d1 = CellCoord::from_a1("D1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, d1, "=SQRT(-5)".into());
    assert_eq!(wb.sheets[0].get_cell_value(d1), CellValue::Error(CellError::Num));

    // E1 = UNKNOWN_FUNC(1, 2) -> #NAME?
    let e1 = CellCoord::from_a1("E1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, e1, "=UNKNOWN_FUNC(1, 2)".into());
    assert_eq!(wb.sheets[0].get_cell_value(e1), CellValue::Error(CellError::Name));
}

#[test]
fn test_statistical_with_empty_and_text_cells() {
    let mut wb = Workbook::with_default_sheet();
    let sheet = wb.active_sheet_mut();

    // A1: 10, A2: "Text", A3: empty, A4: 20
    sheet.set_cell(CellCoord::from_a1("A1").unwrap(), CellState::from_value(CellValue::Number(10.0)));
    sheet.set_cell(CellCoord::from_a1("A2").unwrap(), CellState::from_value(CellValue::Text("Ignored".into())));
    sheet.set_cell(CellCoord::from_a1("A4").unwrap(), CellState::from_value(CellValue::Number(20.0)));

    let mut engine = CalcEngine::new();
    let s_id = wb.sheets[0].id.clone();

    // SUM(A1:A4) should be 30 (text & empty ignored)
    let sum_coord = CellCoord::from_a1("B1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, sum_coord, "=SUM(A1:A4)".into());
    assert_eq!(wb.sheets[0].get_cell_value(sum_coord), CellValue::Number(30.0));

    // AVERAGE(A1:A4) should be 15 (count of numbers is 2: 10 and 20)
    let avg_coord = CellCoord::from_a1("B2").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, avg_coord, "=AVERAGE(A1:A4)".into());
    assert_eq!(wb.sheets[0].get_cell_value(avg_coord), CellValue::Number(15.0));

    // COUNT(A1:A4) should be 2
    let count_coord = CellCoord::from_a1("B3").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, count_coord, "=COUNT(A1:A4)".into());
    assert_eq!(wb.sheets[0].get_cell_value(count_coord), CellValue::Number(2.0));

    // COUNTA(A1:A4) should be 3 (A1, A2, A4 are non-empty)
    let counta_coord = CellCoord::from_a1("B4").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, counta_coord, "=COUNTA(A1:A4)".into());
    assert_eq!(wb.sheets[0].get_cell_value(counta_coord), CellValue::Number(3.0));
}

#[test]
fn test_advanced_lookup_vlookup_approximate() {
    let mut wb = Workbook::with_default_sheet();
    let sheet = wb.active_sheet_mut();

    // Tax brackets table:
    // A1: 0,     B1: 0.10
    // A2: 10000, B2: 0.15
    // A3: 50000, B3: 0.25
    // A4: 100000, B4: 0.35
    sheet.set_cell(CellCoord::from_a1("A1").unwrap(), CellState::from_value(CellValue::Number(0.0)));
    sheet.set_cell(CellCoord::from_a1("B1").unwrap(), CellState::from_value(CellValue::Number(0.10)));

    sheet.set_cell(CellCoord::from_a1("A2").unwrap(), CellState::from_value(CellValue::Number(10000.0)));
    sheet.set_cell(CellCoord::from_a1("B2").unwrap(), CellState::from_value(CellValue::Number(0.15)));

    sheet.set_cell(CellCoord::from_a1("A3").unwrap(), CellState::from_value(CellValue::Number(50000.0)));
    sheet.set_cell(CellCoord::from_a1("B3").unwrap(), CellState::from_value(CellValue::Number(0.25)));

    sheet.set_cell(CellCoord::from_a1("A4").unwrap(), CellState::from_value(CellValue::Number(100000.0)));
    sheet.set_cell(CellCoord::from_a1("B4").unwrap(), CellState::from_value(CellValue::Number(0.35)));

    let mut engine = CalcEngine::new();
    let s_id = wb.sheets[0].id.clone();

    // Income: 25000 -> should match bracket 10000 -> 0.15
    let res1 = CellCoord::from_a1("C1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, res1, "=VLOOKUP(25000, A1:B4, 2, TRUE)".into());
    assert_eq!(wb.sheets[0].get_cell_value(res1), CellValue::Number(0.15));

    // Income: 75000 -> should match bracket 50000 -> 0.25
    let res2 = CellCoord::from_a1("C2").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, res2, "=VLOOKUP(75000, A1:B4, 2, TRUE)".into());
    assert_eq!(wb.sheets[0].get_cell_value(res2), CellValue::Number(0.25));

    // Match index of 50000 in A1:A4 -> 3
    let res3 = CellCoord::from_a1("C3").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, res3, "=MATCH(50000, A1:A4, 0)".into());
    assert_eq!(wb.sheets[0].get_cell_value(res3), CellValue::Number(3.0));
}

#[test]
fn test_complex_excel_formulas_combined() {
    let mut wb = Workbook::with_default_sheet();
    let s_id = wb.sheets[0].id.clone();
    let mut engine = CalcEngine::new();

    // String concat with TRIM and UPPER
    let r1 = CellCoord::from_a1("A1").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, r1, r#"=CONCAT(UPPER(TRIM("  rust  ")), " ", LOWER("EXEL"))"#.into());
    assert_eq!(wb.sheets[0].get_cell_value(r1), CellValue::Text("RUST exel".to_string()));

    // Nested IF with AND / OR
    let r2 = CellCoord::from_a1("A2").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, r2, r#"=IF(AND(10 > 5, OR(1 = 2, 3 = 3)), "VALID", "INVALID")"#.into());
    assert_eq!(wb.sheets[0].get_cell_value(r2), CellValue::Text("VALID".to_string()));

    // Math combinations
    let r3 = CellCoord::from_a1("A3").unwrap();
    engine.update_cell_formula(&mut wb, &s_id, r3, "=ROUND(POWER(SQRT(144) + 8, 1.5), 1)".into());
    // sqrt(144) = 12; 12 + 8 = 20; 20^1.5 = 89.4427... rounded to 1 digit = 89.4
    assert_eq!(wb.sheets[0].get_cell_value(r3), CellValue::Number(89.4));
}
