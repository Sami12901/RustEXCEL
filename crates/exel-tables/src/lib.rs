use exel_cell::{CellCoord, CellRange, CellValue, SheetId};
use exel_core::CoreError;
use exel_workbook::Worksheet;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    #[error("Invalid table name '{0}': names cannot contain spaces or special characters")]
    InvalidTableName(String),
    #[error("Table with name '{0}' already exists")]
    DuplicateTableName(String),
    #[error("Table range must span at least 1 column and 1 row")]
    InvalidRange,
    #[error("Column '{0}' not found in table '{1}'")]
    ColumnNotFound(String, String),
    #[error("Invalid structured reference: '{0}'")]
    InvalidStructuredReference(String),
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// Aggregation functions supported in the structured total row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TotalFunction {
    None,
    Average,
    Count,
    CountNumbers,
    Max,
    Min,
    Sum,
    StdDev,
    Var,
}

impl TotalFunction {
    pub fn as_str(&self) -> &'static str {
        match self {
            TotalFunction::None => "None",
            TotalFunction::Average => "AVERAGE",
            TotalFunction::Count => "COUNT",
            TotalFunction::CountNumbers => "COUNTNUMBERS",
            TotalFunction::Max => "MAX",
            TotalFunction::Min => "MIN",
            TotalFunction::Sum => "SUM",
            TotalFunction::StdDev => "STDEV",
            TotalFunction::Var => "VAR",
        }
    }
}

/// Comparison operators for numeric column filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

/// Filter condition for a table column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnFilter {
    /// Excel checklist filter (only rows whose cell value matches one in the set).
    Values(Vec<CellValue>),
    /// Numeric comparison filter.
    NumericComparison { op: ComparisonOp, target: f64 },
    /// Text contains / starts with / ends with filter.
    TextContains {
        pattern: String,
        case_sensitive: bool,
    },
    /// Filter for blank cells only.
    Blank,
    /// Filter for non-blank cells only.
    NonBlank,
}

impl ColumnFilter {
    pub fn matches(&self, value: &CellValue) -> bool {
        match self {
            ColumnFilter::Values(candidates) => candidates.iter().any(|c| c == value),
            ColumnFilter::NumericComparison { op, target } => {
                if let Some(num) = value.as_number() {
                    match op {
                        ComparisonOp::Equal => (num - target).abs() < f64::EPSILON,
                        ComparisonOp::NotEqual => (num - target).abs() >= f64::EPSILON,
                        ComparisonOp::GreaterThan => num > *target,
                        ComparisonOp::GreaterThanOrEqual => num >= *target,
                        ComparisonOp::LessThan => num < *target,
                        ComparisonOp::LessThanOrEqual => num <= *target,
                    }
                } else {
                    false
                }
            }
            ColumnFilter::TextContains {
                pattern,
                case_sensitive,
            } => {
                let text = value.to_display_string();
                if *case_sensitive {
                    text.contains(pattern)
                } else {
                    text.to_lowercase().contains(&pattern.to_lowercase())
                }
            }
            ColumnFilter::Blank => value.is_empty(),
            ColumnFilter::NonBlank => !value.is_empty(),
        }
    }
}

/// A single column within a structured table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableColumn {
    pub id: u32,
    pub name: String,
    pub total_function: Option<TotalFunction>,
    pub total_formula: Option<String>,
    pub filter: Option<ColumnFilter>,
}

impl TableColumn {
    pub fn new<S: Into<String>>(id: u32, name: S) -> Self {
        Self {
            id,
            name: name.into(),
            total_function: None,
            total_formula: None,
            filter: None,
        }
    }

    pub fn with_total_function(mut self, func: TotalFunction) -> Self {
        self.total_function = Some(func);
        self
    }

    pub fn with_filter(mut self, filter: ColumnFilter) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// Structured Table (`ListObject`) definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub id: u32,
    pub name: String,
    pub sheet_id: SheetId,
    pub range: CellRange,
    pub has_header_row: bool,
    pub has_total_row: bool,
    pub banded_rows: bool,
    pub banded_cols: bool,
    pub style_name: String,
    pub columns: Vec<TableColumn>,
}

impl Table {
    pub fn new<S: Into<String>>(
        id: u32,
        name: S,
        sheet_id: SheetId,
        range: CellRange,
        column_names: Vec<String>,
    ) -> Result<Self, TableError> {
        let name_str = name.into();
        Self::validate_name(&name_str)?;

        let num_cols = range.col_count() as usize;
        let mut columns = Vec::with_capacity(num_cols);

        for (idx, col_num) in (0..num_cols).enumerate() {
            let col_name = column_names
                .get(idx)
                .cloned()
                .unwrap_or_else(|| format!("Column{}", idx + 1));
            columns.push(TableColumn::new((col_num + 1) as u32, col_name));
        }

        Ok(Self {
            id,
            name: name_str,
            sheet_id,
            range,
            has_header_row: true,
            has_total_row: false,
            banded_rows: true,
            banded_cols: false,
            style_name: "TableStyleMedium2".to_string(),
            columns,
        })
    }

    pub fn validate_name(name: &str) -> Result<(), TableError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TableError::InvalidTableName(name.to_string()));
        }
        let first = trimmed.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return Err(TableError::InvalidTableName(name.to_string()));
        }
        if trimmed.chars().any(|c| !c.is_alphanumeric() && c != '_') {
            return Err(TableError::InvalidTableName(name.to_string()));
        }
        Ok(())
    }

    /// Header row coordinate range.
    pub fn header_range(&self) -> Option<CellRange> {
        if !self.has_header_row {
            return None;
        }
        let start = self.range.start;
        let end = CellCoord::must_new(self.range.start.row, self.range.end.col);
        Some(CellRange::new(start, end))
    }

    /// Total row coordinate range.
    pub fn total_range(&self) -> Option<CellRange> {
        if !self.has_total_row {
            return None;
        }
        let start = CellCoord::must_new(self.range.end.row, self.range.start.col);
        let end = self.range.end;
        Some(CellRange::new(start, end))
    }

    /// Data rows coordinate range excluding header and total rows.
    pub fn data_range(&self) -> Option<CellRange> {
        let mut start_row = self.range.start.row;
        let mut end_row = self.range.end.row;

        if self.has_header_row {
            start_row += 1;
        }
        if self.has_total_row {
            if end_row <= start_row {
                return None;
            }
            end_row -= 1;
        }

        if start_row > end_row {
            return None;
        }

        Some(CellRange::new(
            CellCoord::must_new(start_row, self.range.start.col),
            CellCoord::must_new(end_row, self.range.end.col),
        ))
    }

    /// Get column definition by name (case-insensitive).
    pub fn column_by_name(&self, name: &str) -> Option<&TableColumn> {
        self.columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// 0-based column index by name.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Check if a given row is visible according to all active column filters.
    pub fn is_row_visible(&self, row: u32, sheet: &Worksheet) -> bool {
        for (idx, col) in self.columns.iter().enumerate() {
            if let Some(filter) = &col.filter {
                let col_num = self.range.start.col + idx as u32;
                if let Ok(coord) = CellCoord::new(row, col_num) {
                    let val = sheet.get_cell_value(coord);
                    if !filter.matches(&val) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Compute the total summary value for a column according to its TotalFunction.
    pub fn compute_total(&self, col_index: usize, sheet: &Worksheet) -> Option<CellValue> {
        let col = self.columns.get(col_index)?;
        let func = col.total_function?;
        let data_range = self.data_range()?;

        let col_num = self.range.start.col + col_index as u32;
        let mut numbers: Vec<f64> = Vec::new();
        let mut count = 0usize;

        for row in data_range.start.row..=data_range.end.row {
            // Apply filtering if row is hidden
            if !self.is_row_visible(row, sheet) {
                continue;
            }
            if let Ok(coord) = CellCoord::new(row, col_num) {
                let val = sheet.get_cell_value(coord);
                if !val.is_empty() {
                    count += 1;
                }
                if let Some(num) = val.as_number() {
                    numbers.push(num);
                }
            }
        }

        match func {
            TotalFunction::None => None,
            TotalFunction::Count => Some(CellValue::Number(count as f64)),
            TotalFunction::CountNumbers => Some(CellValue::Number(numbers.len() as f64)),
            TotalFunction::Sum => {
                let sum: f64 = numbers.iter().sum();
                Some(CellValue::Number(sum))
            }
            TotalFunction::Average => {
                if numbers.is_empty() {
                    Some(CellValue::Number(0.0))
                } else {
                    let avg = numbers.iter().sum::<f64>() / numbers.len() as f64;
                    Some(CellValue::Number(avg))
                }
            }
            TotalFunction::Max => {
                let max = numbers.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                Some(CellValue::Number(if max.is_infinite() { 0.0 } else { max }))
            }
            TotalFunction::Min => {
                let min = numbers.iter().cloned().fold(f64::INFINITY, f64::min);
                Some(CellValue::Number(if min.is_infinite() { 0.0 } else { min }))
            }
            TotalFunction::StdDev => {
                if numbers.len() < 2 {
                    Some(CellValue::Number(0.0))
                } else {
                    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
                    let var = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                        / (numbers.len() - 1) as f64;
                    Some(CellValue::Number(var.sqrt()))
                }
            }
            TotalFunction::Var => {
                if numbers.len() < 2 {
                    Some(CellValue::Number(0.0))
                } else {
                    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
                    let var = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                        / (numbers.len() - 1) as f64;
                    Some(CellValue::Number(var))
                }
            }
        }
    }

    /// Resolve structured reference syntax e.g. `[Column1]`, `[#Headers]`, `[@Column1]`.
    pub fn resolve_structured_ref(
        &self,
        ref_text: &str,
        current_row: Option<u32>,
    ) -> Result<CellRange, TableError> {
        let trimmed = ref_text.trim();
        let unbracketed = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);

        // Special items: #All, #Headers, #Data, #Totals
        match unbracketed {
            "#All" => Ok(self.range),
            "#Headers" => self.header_range().ok_or_else(|| {
                TableError::InvalidStructuredReference("Table has no header row".into())
            }),
            "#Totals" => self.total_range().ok_or_else(|| {
                TableError::InvalidStructuredReference("Table has no total row".into())
            }),
            "#Data" => self.data_range().ok_or_else(|| {
                TableError::InvalidStructuredReference("Table has no data rows".into())
            }),
            col_ref => {
                // Check if this-row operator '@' is present e.g. "@Amount"
                if let Some(col_name) = col_ref.strip_prefix('@') {
                    let col_idx = self.column_index(col_name).ok_or_else(|| {
                        TableError::ColumnNotFound(col_name.to_string(), self.name.clone())
                    })?;
                    let row = current_row.ok_or_else(|| {
                        TableError::InvalidStructuredReference(
                            "This-row reference '@' requires active cell row context".into(),
                        )
                    })?;
                    let col_num = self.range.start.col + col_idx as u32;
                    let coord = CellCoord::new(row, col_num)?;
                    Ok(CellRange::single_cell(coord))
                } else {
                    // Regular column data reference e.g. "Amount"
                    let col_idx = self.column_index(col_ref).ok_or_else(|| {
                        TableError::ColumnNotFound(col_ref.to_string(), self.name.clone())
                    })?;
                    let data_range = self.data_range().ok_or_else(|| {
                        TableError::InvalidStructuredReference("Table has no data rows".into())
                    })?;
                    let col_num = self.range.start.col + col_idx as u32;
                    Ok(CellRange::new(
                        CellCoord::must_new(data_range.start.row, col_num),
                        CellCoord::must_new(data_range.end.row, col_num),
                    ))
                }
            }
        }
    }
}

/// Collection manager for all structured tables across a workbook.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableManager {
    tables: Vec<Table>,
    next_id: u32,
}

impl TableManager {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_table(&mut self, mut table: Table) -> Result<u32, TableError> {
        Table::validate_name(&table.name)?;
        if self
            .tables
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case(&table.name))
        {
            return Err(TableError::DuplicateTableName(table.name));
        }

        if table.id == 0 {
            table.id = self.next_id;
            self.next_id += 1;
        } else {
            self.next_id = self.next_id.max(table.id + 1);
        }

        let id = table.id;
        self.tables.push(table);
        Ok(id)
    }

    pub fn remove_table(&mut self, name: &str) -> Option<Table> {
        let pos = self
            .tables
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(name))?;
        Some(self.tables.remove(pos))
    }

    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut Table> {
        self.tables
            .iter_mut()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    pub fn find_table_at(&self, sheet_id: &SheetId, coord: CellCoord) -> Option<&Table> {
        self.tables
            .iter()
            .find(|t| &t.sheet_id == sheet_id && t.range.contains(coord))
    }

    pub fn tables_on_sheet(&self, sheet_id: &SheetId) -> Vec<&Table> {
        self.tables
            .iter()
            .filter(|t| &t.sheet_id == sheet_id)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_creation_and_ranges() {
        let sheet_id = SheetId::new("Sheet1");
        let range = CellRange::from_a1("A1:C5").unwrap();
        let col_names = vec!["ID".into(), "Name".into(), "Score".into()];

        let mut table = Table::new(1, "ScoresTable", sheet_id.clone(), range, col_names).unwrap();
        assert_eq!(table.columns.len(), 3);
        assert_eq!(table.columns[0].name, "ID");
        assert_eq!(table.columns[2].name, "Score");

        let header_range = table.header_range().unwrap();
        assert_eq!(header_range.to_a1(), "A1:C1");

        let data_range = table.data_range().unwrap();
        assert_eq!(data_range.to_a1(), "A2:C5");

        // Enable total row
        table.has_total_row = true;
        let total_range = table.total_range().unwrap();
        assert_eq!(total_range.to_a1(), "A5:C5");

        let adjusted_data_range = table.data_range().unwrap();
        assert_eq!(adjusted_data_range.to_a1(), "A2:C4");
    }

    #[test]
    fn test_table_total_computation() {
        let sheet_id = SheetId::new("Sheet1");
        let mut sheet = Worksheet::new(sheet_id.clone(), "Sheet1");
        let range = CellRange::from_a1("A1:B4").unwrap();

        let mut table = Table::new(
            1,
            "Sales",
            sheet_id,
            range,
            vec!["Product".into(), "Amount".into()],
        )
        .unwrap();

        // Populate values
        sheet.set_cell_value(
            CellCoord::from_a1("A2").unwrap(),
            CellValue::Text("A".into()),
        );
        sheet.set_cell_value(CellCoord::from_a1("B2").unwrap(), CellValue::Number(100.0));
        sheet.set_cell_value(
            CellCoord::from_a1("A3").unwrap(),
            CellValue::Text("B".into()),
        );
        sheet.set_cell_value(CellCoord::from_a1("B3").unwrap(), CellValue::Number(200.0));
        sheet.set_cell_value(
            CellCoord::from_a1("A4").unwrap(),
            CellValue::Text("C".into()),
        );
        sheet.set_cell_value(CellCoord::from_a1("B4").unwrap(), CellValue::Number(300.0));

        table.columns[1].total_function = Some(TotalFunction::Sum);
        let sum_total = table.compute_total(1, &sheet).unwrap();
        assert_eq!(sum_total, CellValue::Number(600.0));

        table.columns[1].total_function = Some(TotalFunction::Average);
        let avg_total = table.compute_total(1, &sheet).unwrap();
        assert_eq!(avg_total, CellValue::Number(200.0));
    }

    #[test]
    fn test_structured_references() {
        let sheet_id = SheetId::new("Sheet1");
        let range = CellRange::from_a1("B2:D6").unwrap();
        let table = Table::new(
            1,
            "Orders",
            sheet_id,
            range,
            vec!["Item".into(), "Qty".into(), "Price".into()],
        )
        .unwrap();

        let qty_range = table.resolve_structured_ref("Qty", None).unwrap();
        assert_eq!(qty_range.to_a1(), "C3:C6");

        let header_range = table.resolve_structured_ref("#Headers", None).unwrap();
        assert_eq!(header_range.to_a1(), "B2:D2");

        let this_row_qty = table.resolve_structured_ref("@Qty", Some(4)).unwrap();
        assert_eq!(this_row_qty.to_a1(), "C4");
    }

    #[test]
    fn test_table_manager() {
        let mut mgr = TableManager::new();
        let sheet_id = SheetId::new("Sheet1");
        let range = CellRange::from_a1("A1:B10").unwrap();

        let table = Table::new(
            0,
            "Table1",
            sheet_id.clone(),
            range,
            vec!["ColA".into(), "ColB".into()],
        )
        .unwrap();
        let id = mgr.add_table(table).unwrap();
        assert_eq!(id, 1);
        assert_eq!(mgr.len(), 1);

        assert!(mgr.get_table("Table1").is_some());
        assert!(mgr
            .find_table_at(&sheet_id, CellCoord::from_a1("A5").unwrap())
            .is_some());
        assert!(mgr
            .find_table_at(&sheet_id, CellCoord::from_a1("C5").unwrap())
            .is_none());
    }
}
