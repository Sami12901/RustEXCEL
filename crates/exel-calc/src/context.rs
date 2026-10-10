use exel_cell::CellValue;
use exel_core::CellCoord;
use exel_formula::RangeRef;
use exel_workbook::{Workbook, Worksheet};

/// Execution context for formula evaluation.
pub struct CalcContext<'a> {
    pub workbook: &'a Workbook,
    pub current_sheet_name: &'a str,
    pub current_coord: Option<CellCoord>,
}

impl<'a> CalcContext<'a> {
    pub fn new(workbook: &'a Workbook, current_sheet_name: &'a str) -> Self {
        Self {
            workbook,
            current_sheet_name,
            current_coord: None,
        }
    }

    pub fn with_coord(
        workbook: &'a Workbook,
        current_sheet_name: &'a str,
        current_coord: CellCoord,
    ) -> Self {
        Self {
            workbook,
            current_sheet_name,
            current_coord: Some(current_coord),
        }
    }

    /// Retrieves the worksheet for the given optional sheet name, or the current sheet if None.
    pub fn get_sheet(&self, sheet_name: Option<&str>) -> Option<&Worksheet> {
        match sheet_name {
            Some(name) => self.workbook.get_sheet_by_name(name),
            None => self.workbook.get_sheet_by_name(self.current_sheet_name),
        }
    }

    /// Fetches the value of a single cell given its coordinate and optional sheet name.
    pub fn get_cell_value(&self, sheet_name: Option<&str>, coord: CellCoord) -> CellValue {
        if let Some(sheet) = self.get_sheet(sheet_name) {
            sheet.get_cell_value(coord)
        } else {
            CellValue::Error(exel_cell::CellError::Ref)
        }
    }

    /// Fetches all values within a rectangular range reference in row-major order.
    pub fn get_range_values(&self, sheet_name: Option<&str>, range: &RangeRef) -> Vec<CellValue> {
        let sheet = self.get_sheet(sheet_name.or(range.sheet.as_deref()));
        if let Some(sheet) = sheet {
            let mut values = Vec::new();
            for coord in range.iter_coords() {
                values.push(sheet.get_cell_value(coord));
            }
            values
        } else {
            vec![CellValue::Error(exel_cell::CellError::Ref)]
        }
    }

    /// Fetches a 2D grid of values (rows of columns) for a range reference.
    pub fn get_range_grid(
        &self,
        sheet_name: Option<&str>,
        range: &RangeRef,
    ) -> Vec<Vec<CellValue>> {
        let sheet = self.get_sheet(sheet_name.or(range.sheet.as_deref()));
        if let Some(sheet) = sheet {
            let cell_range = range.to_cell_range();
            let mut grid = Vec::new();
            for row in cell_range.start.row..=cell_range.end.row {
                let mut row_vals = Vec::new();
                for col in cell_range.start.col..=cell_range.end.col {
                    let coord = CellCoord::must_new(row, col);
                    row_vals.push(sheet.get_cell_value(coord));
                }
                grid.push(row_vals);
            }
            grid
        } else {
            vec![vec![CellValue::Error(exel_cell::CellError::Ref)]]
        }
    }
}
