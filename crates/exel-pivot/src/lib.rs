//! Pivot Table calculation engine and data structures for Rust EXCEL.
//!
//! Provides `PivotTableDefinition`, multidimensional grouping by row and column fields,
//! data field aggregation (Sum, Count, Average, Min, Max), grand totals, and
//! rendering into worksheet cells.

use exel_cell::{CellCoord, CellRange, CellValue};
use exel_workbook::Worksheet;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PivotError {
    #[error("Source range is invalid or empty")]
    EmptySourceRange,
    #[error("Field '{0}' not found in source range headers")]
    FieldNotFound(String),
    #[error("No data rows found below headers")]
    NoDataRows,
}

/// Aggregation function applied to pivot data fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PivotAggregation {
    Sum,
    Count,
    Average,
    Min,
    Max,
}

/// A value field to be aggregated in the pivot table body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PivotDataField {
    pub source_field: String,
    pub display_name: String,
    pub aggregation: PivotAggregation,
}

impl PivotDataField {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        source_field: S1,
        display_name: S2,
        aggregation: PivotAggregation,
    ) -> Self {
        Self {
            source_field: source_field.into(),
            display_name: display_name.into(),
            aggregation,
        }
    }
}

/// Definition specifying the layout and fields of a Pivot Table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotTableDefinition {
    pub name: String,
    pub source_range: CellRange,
    pub row_fields: Vec<String>,
    pub column_fields: Vec<String>,
    pub data_fields: Vec<PivotDataField>,
    pub show_row_grand_totals: bool,
    pub show_col_grand_totals: bool,
}

impl PivotTableDefinition {
    pub fn new<S: Into<String>>(name: S, source_range: CellRange) -> Self {
        Self {
            name: name.into(),
            source_range,
            row_fields: Vec::new(),
            column_fields: Vec::new(),
            data_fields: Vec::new(),
            show_row_grand_totals: true,
            show_col_grand_totals: true,
        }
    }

    pub fn with_row_field<S: Into<String>>(mut self, field: S) -> Self {
        self.row_fields.push(field.into());
        self
    }

    pub fn with_column_field<S: Into<String>>(mut self, field: S) -> Self {
        self.column_fields.push(field.into());
        self
    }

    pub fn with_data_field(mut self, field: PivotDataField) -> Self {
        self.data_fields.push(field);
        self
    }
}

/// Calculated multi-dimensional pivot table results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotResult {
    pub row_keys: Vec<Vec<String>>,
    pub col_keys: Vec<Vec<String>>,
    /// 3D array of aggregated values: [row_idx][col_idx][data_field_idx]
    pub values: Vec<Vec<Vec<f64>>>,
    /// Row grand totals: [row_idx][data_field_idx]
    pub row_totals: Vec<Vec<f64>>,
    /// Column grand totals: [col_idx][data_field_idx]
    pub col_totals: Vec<Vec<f64>>,
    /// Grand totals for entire dataset: [data_field_idx]
    pub grand_totals: Vec<f64>,
}

#[derive(Default, Clone)]
struct Accumulator {
    sum: f64,
    count: usize,
    min: f64,
    max: f64,
}

impl Accumulator {
    fn add(&mut self, val: f64) {
        self.sum += val;
        self.count += 1;
        if self.count == 1 {
            self.min = val;
            self.max = val;
        } else {
            self.min = self.min.min(val);
            self.max = self.max.max(val);
        }
    }

    fn finalize(&self, agg: PivotAggregation) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        match agg {
            PivotAggregation::Sum => self.sum,
            PivotAggregation::Count => self.count as f64,
            PivotAggregation::Average => self.sum / (self.count as f64),
            PivotAggregation::Min => self.min,
            PivotAggregation::Max => self.max,
        }
    }
}

/// Evaluates a pivot table definition against source data in a worksheet.
pub fn calculate_pivot_table(
    sheet: &Worksheet,
    def: &PivotTableDefinition,
) -> Result<PivotResult, PivotError> {
    let range = def.source_range;
    if range.start.row >= range.end.row || range.start.col > range.end.col {
        return Err(PivotError::EmptySourceRange);
    }

    // 1. Read header mapping (name -> column index)
    let header_row = range.start.row;
    let mut header_cols: BTreeMap<String, u32> = BTreeMap::new();
    for c in range.start.col..=range.end.col {
        let coord = CellCoord::must_new(header_row, c);
        let name = sheet.get_cell_value(coord).to_display_string();
        if !name.is_empty() {
            header_cols.insert(name, c);
        }
    }

    // Verify all specified fields exist
    for f in &def.row_fields {
        if !header_cols.contains_key(f) {
            return Err(PivotError::FieldNotFound(f.clone()));
        }
    }
    for f in &def.column_fields {
        if !header_cols.contains_key(f) {
            return Err(PivotError::FieldNotFound(f.clone()));
        }
    }
    for df in &def.data_fields {
        if !header_cols.contains_key(&df.source_field) {
            return Err(PivotError::FieldNotFound(df.source_field.clone()));
        }
    }

    let first_data_row = header_row + 1;
    let last_data_row = range.end.row;
    if first_data_row > last_data_row {
        return Err(PivotError::NoDataRows);
    }

    // 2. Scan data rows, collect distinct keys and accumulate cell values
    let mut distinct_rows: BTreeSet<Vec<String>> = BTreeSet::new();
    let mut distinct_cols: BTreeSet<Vec<String>> = BTreeSet::new();

    // Map: (row_key, col_key, data_field_idx) -> Accumulator
    let mut cells: BTreeMap<(Vec<String>, Vec<String>, usize), Accumulator> = BTreeMap::new();
    let mut row_acc: BTreeMap<(Vec<String>, usize), Accumulator> = BTreeMap::new();
    let mut col_acc: BTreeMap<(Vec<String>, usize), Accumulator> = BTreeMap::new();
    let mut grand_acc: Vec<Accumulator> = vec![Accumulator::default(); def.data_fields.len()];

    for r in first_data_row..=last_data_row {
        let row_key: Vec<String> = def
            .row_fields
            .iter()
            .map(|f| {
                let c = header_cols[f];
                sheet
                    .get_cell_value(CellCoord::must_new(r, c))
                    .to_display_string()
            })
            .collect();

        let col_key: Vec<String> = def
            .column_fields
            .iter()
            .map(|f| {
                let c = header_cols[f];
                sheet
                    .get_cell_value(CellCoord::must_new(r, c))
                    .to_display_string()
            })
            .collect();

        distinct_rows.insert(row_key.clone());
        distinct_cols.insert(col_key.clone());

        for (df_idx, df) in def.data_fields.iter().enumerate() {
            let c = header_cols[&df.source_field];
            let val = sheet
                .get_cell_value(CellCoord::must_new(r, c))
                .as_number()
                .unwrap_or(0.0);

            cells
                .entry((row_key.clone(), col_key.clone(), df_idx))
                .or_default()
                .add(val);

            row_acc
                .entry((row_key.clone(), df_idx))
                .or_default()
                .add(val);

            col_acc
                .entry((col_key.clone(), df_idx))
                .or_default()
                .add(val);

            grand_acc[df_idx].add(val);
        }
    }

    let row_keys: Vec<Vec<String>> = distinct_rows.into_iter().collect();
    let col_keys: Vec<Vec<String>> = distinct_cols.into_iter().collect();

    // 3. Assemble 3D values grid
    let mut values = Vec::with_capacity(row_keys.len());
    for r_k in &row_keys {
        let mut row_entries = Vec::with_capacity(col_keys.len());
        for c_k in &col_keys {
            let mut df_entries = Vec::with_capacity(def.data_fields.len());
            for (df_idx, df) in def.data_fields.iter().enumerate() {
                let acc = cells.get(&(r_k.clone(), c_k.clone(), df_idx));
                let val = acc.map(|a| a.finalize(df.aggregation)).unwrap_or(0.0);
                df_entries.push(val);
            }
            row_entries.push(df_entries);
        }
        values.push(row_entries);
    }

    // 4. Assemble row totals
    let mut row_totals = Vec::with_capacity(row_keys.len());
    for r_k in &row_keys {
        let mut totals = Vec::with_capacity(def.data_fields.len());
        for (df_idx, df) in def.data_fields.iter().enumerate() {
            let acc = row_acc.get(&(r_k.clone(), df_idx));
            totals.push(acc.map(|a| a.finalize(df.aggregation)).unwrap_or(0.0));
        }
        row_totals.push(totals);
    }

    // 5. Assemble column totals
    let mut col_totals = Vec::with_capacity(col_keys.len());
    for c_k in &col_keys {
        let mut totals = Vec::with_capacity(def.data_fields.len());
        for (df_idx, df) in def.data_fields.iter().enumerate() {
            let acc = col_acc.get(&(c_k.clone(), df_idx));
            totals.push(acc.map(|a| a.finalize(df.aggregation)).unwrap_or(0.0));
        }
        col_totals.push(totals);
    }

    // 6. Grand totals
    let grand_totals = def
        .data_fields
        .iter()
        .enumerate()
        .map(|(df_idx, df)| grand_acc[df_idx].finalize(df.aggregation))
        .collect();

    Ok(PivotResult {
        row_keys,
        col_keys,
        values,
        row_totals,
        col_totals,
        grand_totals,
    })
}

/// Writes a calculated PivotResult as formatted cells into a target Worksheet starting at `anchor`.
pub fn render_pivot_to_sheet(
    sheet: &mut Worksheet,
    anchor: CellCoord,
    result: &PivotResult,
    def: &PivotTableDefinition,
) {
    let mut cur_row = anchor.row;
    let cur_col = anchor.col;

    // Header title
    sheet.set_cell_value(
        CellCoord::must_new(cur_row, cur_col),
        CellValue::Text(format!("Pivot Table: {}", def.name)),
    );
    cur_row += 2;

    // Column headers
    let data_col_start = cur_col + (def.row_fields.len() as u32).max(1);
    for (c_idx, c_key) in result.col_keys.iter().enumerate() {
        let col_title = if c_key.is_empty() {
            "Total".to_string()
        } else {
            c_key.join(" - ")
        };
        sheet.set_cell_value(
            CellCoord::must_new(cur_row, data_col_start + (c_idx as u32)),
            CellValue::Text(col_title),
        );
    }

    if def.show_row_grand_totals {
        sheet.set_cell_value(
            CellCoord::must_new(cur_row, data_col_start + (result.col_keys.len() as u32)),
            CellValue::Text("Grand Total".into()),
        );
    }
    cur_row += 1;

    // Data rows
    for (r_idx, r_key) in result.row_keys.iter().enumerate() {
        let row_title = if r_key.is_empty() {
            format!("Row {}", r_idx + 1)
        } else {
            r_key.join(" - ")
        };
        sheet.set_cell_value(
            CellCoord::must_new(cur_row, cur_col),
            CellValue::Text(row_title),
        );

        for (c_idx, _) in result.col_keys.iter().enumerate() {
            let val = result.values[r_idx][c_idx].first().copied().unwrap_or(0.0);
            sheet.set_cell_value(
                CellCoord::must_new(cur_row, data_col_start + (c_idx as u32)),
                CellValue::Number(val),
            );
        }

        if def.show_row_grand_totals {
            let row_total = result.row_totals[r_idx].first().copied().unwrap_or(0.0);
            sheet.set_cell_value(
                CellCoord::must_new(cur_row, data_col_start + (result.col_keys.len() as u32)),
                CellValue::Number(row_total),
            );
        }
        cur_row += 1;
    }

    // Grand totals row
    if def.show_col_grand_totals {
        sheet.set_cell_value(
            CellCoord::must_new(cur_row, cur_col),
            CellValue::Text("Grand Total".into()),
        );

        for (c_idx, _) in result.col_keys.iter().enumerate() {
            let col_total = result.col_totals[c_idx].first().copied().unwrap_or(0.0);
            sheet.set_cell_value(
                CellCoord::must_new(cur_row, data_col_start + (c_idx as u32)),
                CellValue::Number(col_total),
            );
        }

        let gt = result.grand_totals.first().copied().unwrap_or(0.0);
        sheet.set_cell_value(
            CellCoord::must_new(cur_row, data_col_start + (result.col_keys.len() as u32)),
            CellValue::Number(gt),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::SheetId;

    #[test]
    fn test_pivot_sales_by_region_and_product() {
        let mut sheet = Worksheet::new(SheetId::new("s_pivot"), "Sales");

        // Headers: Region | Product | Revenue
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Region".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Text("Product".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 3), CellValue::Text("Revenue".into()));

        // Data
        let rows = [
            ("North", "Apples", 100.0),
            ("North", "Bananas", 50.0),
            ("South", "Apples", 200.0),
            ("South", "Bananas", 150.0),
            ("North", "Apples", 80.0),
        ];

        for (idx, (reg, prod, rev)) in rows.iter().enumerate() {
            let r = (idx + 2) as u32;
            sheet.set_cell_value(CellCoord::must_new(r, 1), CellValue::Text((*reg).into()));
            sheet.set_cell_value(CellCoord::must_new(r, 2), CellValue::Text((*prod).into()));
            sheet.set_cell_value(CellCoord::must_new(r, 3), CellValue::Number(*rev));
        }

        let range = CellRange::new(CellCoord::must_new(1, 1), CellCoord::must_new(6, 3));
        let def = PivotTableDefinition::new("RegionalSales", range)
            .with_row_field("Region")
            .with_column_field("Product")
            .with_data_field(PivotDataField::new(
                "Revenue",
                "Total Revenue",
                PivotAggregation::Sum,
            ));

        let res = calculate_pivot_table(&sheet, &def).unwrap();

        assert_eq!(res.row_keys, vec![vec!["North"], vec!["South"]]);
        assert_eq!(res.col_keys, vec![vec!["Apples"], vec!["Bananas"]]);

        // North: Apples = 100 + 80 = 180, Bananas = 50
        assert_eq!(res.values[0][0][0], 180.0);
        assert_eq!(res.values[0][1][0], 50.0);
        assert_eq!(res.row_totals[0][0], 230.0);

        // South: Apples = 200, Bananas = 150
        assert_eq!(res.values[1][0][0], 200.0);
        assert_eq!(res.values[1][1][0], 150.0);
        assert_eq!(res.row_totals[1][0], 350.0);

        // Column totals: Apples = 380, Bananas = 200
        assert_eq!(res.col_totals[0][0], 380.0);
        assert_eq!(res.col_totals[1][0], 200.0);

        // Grand total = 580.0
        assert_eq!(res.grand_totals[0], 580.0);
    }

    #[test]
    fn test_pivot_render_to_sheet() {
        let mut sheet = Worksheet::new(SheetId::new("s_source"), "Source");
        sheet.set_cell_value(
            CellCoord::must_new(1, 1),
            CellValue::Text("Category".into()),
        );
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Text("Amount".into()));

        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Tools".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 2), CellValue::Number(25.0));

        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Text("Tools".into()));
        sheet.set_cell_value(CellCoord::must_new(3, 2), CellValue::Number(75.0));

        let range = CellRange::new(CellCoord::must_new(1, 1), CellCoord::must_new(3, 2));
        let def = PivotTableDefinition::new("CategorySummary", range)
            .with_row_field("Category")
            .with_data_field(PivotDataField::new(
                "Amount",
                "Total Amount",
                PivotAggregation::Sum,
            ));

        let res = calculate_pivot_table(&sheet, &def).unwrap();
        let mut out_sheet = Worksheet::new(SheetId::new("s_out"), "PivotView");
        render_pivot_to_sheet(&mut out_sheet, CellCoord::must_new(1, 1), &res, &def);

        assert_eq!(res.grand_totals[0], 100.0);
    }
}
