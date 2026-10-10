//! Excel TSV clipboard serialization, parsing, and system integration.
//!
//! Provides RFC-compliant TSV parsing and formatting for rectangular cell ranges,
//! supporting quotes, embedded tabs, newlines, formulas, and values.
//! Also provides safe wrappers around `arboard` for system clipboard operations.

pub mod error;
pub mod range;
pub mod system;
pub mod tsv;

pub use error::ClipboardError;
pub use range::{
    cells_to_tsv, copy_range_grid, copy_range_values, paste_tsv_to_sheet, tsv_to_cell_data,
};
pub use system::{
    copy_range, copy_text_to_system_clipboard, get_text_from_system_clipboard, paste_to_sheet,
};
pub use tsv::{escape_tsv_cell, format_tsv, format_tsv_values, parse_tsv, parse_tsv_values};

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::{CellCoord, CellRange, CellValue, SheetId};
    use exel_workbook::Worksheet;

    #[test]
    fn test_clipboard_tsv_full_matrix_roundtrip() {
        let grid = vec![
            vec![
                "Header 1".to_string(),
                "Header\t2".to_string(),
                "Header \"3\"".to_string(),
            ],
            vec![
                "Line 1\nLine 2".to_string(),
                "123.45".to_string(),
                "TRUE".to_string(),
            ],
            vec![
                "".to_string(),
                "Value with \"internal\" quotes".to_string(),
                "=SUM(A1:B1)".to_string(),
            ],
        ];

        let serialized = format_tsv(&grid);
        let deserialized = parse_tsv(&serialized);

        assert_eq!(grid, deserialized);
    }

    #[test]
    fn test_clipboard_worksheet_integration() {
        let mut sheet = Worksheet::new(SheetId::new("s_clipboard"), "ClipboardTest");
        let a1 = CellCoord::must_new(1, 1);
        let b1 = CellCoord::must_new(1, 2);
        let c1 = CellCoord::must_new(1, 3);
        let a2 = CellCoord::must_new(2, 1);
        let b2 = CellCoord::must_new(2, 2);
        let c2 = CellCoord::must_new(2, 3);

        sheet.set_cell_value(a1, CellValue::Text("Product\tA".to_string()));
        sheet.set_cell_value(b1, CellValue::Number(45.50));
        sheet.set_cell_value(c1, CellValue::Boolean(true));

        sheet.set_cell_value(a2, CellValue::Text("Multiline\nDescription".to_string()));
        sheet.set_cell_value(b2, CellValue::Number(90.0));
        sheet.set_cell_formula(c2, "=B1 + B2".to_string(), CellValue::Number(135.50));

        let range = CellRange::new(a1, c2);
        let tsv = cells_to_tsv(&sheet, range);

        let mut dest_sheet = Worksheet::new(SheetId::new("s_dest"), "DestSheet");
        let paste_anchor = CellCoord::must_new(5, 5); // E5
        let modified = paste_tsv_to_sheet(&mut dest_sheet, paste_anchor, &tsv);

        assert_eq!(modified.len(), 6);
        assert_eq!(
            dest_sheet.get_cell_value(CellCoord::must_new(5, 5)),
            CellValue::Text("Product\tA".to_string())
        );
        assert_eq!(
            dest_sheet.get_cell_value(CellCoord::must_new(5, 6)),
            CellValue::Number(45.50)
        );
        assert_eq!(
            dest_sheet.get_cell_value(CellCoord::must_new(5, 7)),
            CellValue::Boolean(true)
        );
        assert_eq!(
            dest_sheet.get_cell_value(CellCoord::must_new(6, 5)),
            CellValue::Text("Multiline\nDescription".to_string())
        );
        assert_eq!(
            dest_sheet.get_cell_value(CellCoord::must_new(6, 6)),
            CellValue::Number(90.0)
        );
        assert_eq!(
            dest_sheet
                .get_cell(CellCoord::must_new(6, 7))
                .unwrap()
                .formula,
            Some("=B1 + B2".to_string())
        );
    }
}
