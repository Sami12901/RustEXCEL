use crate::tsv::{format_tsv, parse_tsv};
use exel_cell::{CellCoord, CellRange, CellValue};
use exel_core::{MAX_COLS, MAX_ROWS};
use exel_workbook::Worksheet;

/// Extracts cell data in a range as a 2D grid of display strings/formula strings.
pub fn copy_range_grid(sheet: &Worksheet, range: CellRange) -> Vec<Vec<String>> {
    let mut grid = Vec::new();
    for r in range.start.row..=range.end.row {
        let mut row = Vec::new();
        for c in range.start.col..=range.end.col {
            if let Ok(coord) = CellCoord::new(r, c) {
                let cell = sheet.get_cell(coord);
                let text = cell
                    .and_then(|st| {
                        st.formula
                            .clone()
                            .or_else(|| Some(st.value.to_display_string()))
                    })
                    .unwrap_or_default();
                row.push(text);
            }
        }
        grid.push(row);
    }
    grid
}

/// Extracts cell values in a range as a 2D grid of `CellValue`.
pub fn copy_range_values(sheet: &Worksheet, range: CellRange) -> Vec<Vec<CellValue>> {
    let mut grid = Vec::new();
    for r in range.start.row..=range.end.row {
        let mut row = Vec::new();
        for c in range.start.col..=range.end.col {
            if let Ok(coord) = CellCoord::new(r, c) {
                row.push(sheet.get_cell_value(coord));
            }
        }
        grid.push(row);
    }
    grid
}

/// Serializes a rectangular cell range from a worksheet into a full RFC-compliant TSV string.
pub fn cells_to_tsv(sheet: &Worksheet, range: CellRange) -> String {
    let grid = copy_range_grid(sheet, range);
    format_tsv(&grid)
}

/// Parses TSV text into individual cell entries anchored at `anchor`, respecting quotes and newlines.
pub fn tsv_to_cell_data(
    tsv: &str,
    anchor: CellCoord,
) -> Vec<(CellCoord, CellValue, Option<String>)> {
    let grid = parse_tsv(tsv);
    let mut results = Vec::new();

    for (r_offset, row) in grid.into_iter().enumerate() {
        let r = anchor.row + (r_offset as u32);
        if r > MAX_ROWS {
            break;
        }
        for (c_offset, text) in row.into_iter().enumerate() {
            let c = anchor.col + (c_offset as u32);
            if c > MAX_COLS {
                break;
            }
            if let Ok(coord) = CellCoord::new(r, c) {
                let trimmed = text.trim();
                if trimmed.starts_with('=') && trimmed.len() > 1 {
                    results.push((coord, CellValue::Empty, Some(trimmed.to_string())));
                } else if trimmed.is_empty() {
                    results.push((coord, CellValue::Empty, None));
                } else if let Ok(n) = trimmed.parse::<f64>() {
                    results.push((coord, CellValue::Number(n), None));
                } else if trimmed.eq_ignore_ascii_case("true") {
                    results.push((coord, CellValue::Boolean(true), None));
                } else if trimmed.eq_ignore_ascii_case("false") {
                    results.push((coord, CellValue::Boolean(false), None));
                } else {
                    results.push((coord, CellValue::Text(text), None));
                }
            }
        }
    }

    results
}

/// Pastes TSV text into a worksheet starting at `anchor`, applying formulas and values.
pub fn paste_tsv_to_sheet(sheet: &mut Worksheet, anchor: CellCoord, tsv: &str) -> Vec<CellCoord> {
    let entries = tsv_to_cell_data(tsv, anchor);
    let mut modified = Vec::with_capacity(entries.len());

    for (coord, value, formula) in entries {
        if let Some(formula) = formula {
            sheet.set_cell_formula(coord, formula, value);
        } else {
            sheet.set_cell_value(coord, value);
        }
        modified.push(coord);
    }

    modified
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::SheetId;

    #[test]
    fn test_sheet_copy_paste_with_formulas_and_quotes() {
        let mut sheet = Worksheet::new(SheetId::new("s1"), "Sheet1");
        let a1 = CellCoord::must_new(1, 1);
        let b1 = CellCoord::must_new(1, 2);
        let a2 = CellCoord::must_new(2, 1);
        let b2 = CellCoord::must_new(2, 2);

        sheet.set_cell_value(a1, CellValue::Text("Item\t1".to_string()));
        sheet.set_cell_value(b1, CellValue::Number(100.0));
        sheet.set_cell_value(a2, CellValue::Text("Line1\nLine2".to_string()));
        sheet.set_cell_formula(b2, "=B1 * 2".to_string(), CellValue::Number(200.0));

        let range = CellRange::new(a1, b2);
        let tsv = cells_to_tsv(&sheet, range);

        // Verify TSV escaped quotes
        assert!(tsv.contains("\"Item\t1\""));
        assert!(tsv.contains("\"Line1\nLine2\""));

        // Paste into new sheet at C10
        let mut target_sheet = Worksheet::new(SheetId::new("s2"), "Sheet2");
        let anchor = CellCoord::must_new(10, 3);
        let modified = paste_tsv_to_sheet(&mut target_sheet, anchor, &tsv);
        assert_eq!(modified.len(), 4);

        assert_eq!(
            target_sheet.get_cell_value(CellCoord::must_new(10, 3)),
            CellValue::Text("Item\t1".to_string())
        );
        assert_eq!(
            target_sheet.get_cell_value(CellCoord::must_new(10, 4)),
            CellValue::Number(100.0)
        );
        assert_eq!(
            target_sheet.get_cell_value(CellCoord::must_new(11, 3)),
            CellValue::Text("Line1\nLine2".to_string())
        );
        assert_eq!(
            target_sheet
                .get_cell(CellCoord::must_new(11, 4))
                .unwrap()
                .formula,
            Some("=B1 * 2".to_string())
        );
    }
}
