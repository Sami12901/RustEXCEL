//! Chart models and data extraction engine for Rust EXCEL.
//!
//! Supports Bar, Line, Pie, Area, and Scatter chart models, series extraction from
//! worksheet cell ranges, category labeling, titles, and legend configurations.

use exel_cell::{CellCoord, CellRange, CellValue};
use exel_workbook::Worksheet;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ChartError {
    #[error("Selected range is empty")]
    EmptyRange,
    #[error("No numeric series could be extracted from range {0}")]
    NoNumericData(String),
    #[error("Invalid coordinate: {0}")]
    InvalidCoordinate(String),
}

/// The visual presentation type of the chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Area,
    Scatter,
}

impl std::fmt::Display for ChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bar => write!(f, "Bar"),
            Self::Line => write!(f, "Line"),
            Self::Pie => write!(f, "Pie"),
            Self::Area => write!(f, "Area"),
            Self::Scatter => write!(f, "Scatter"),
        }
    }
}

/// Legend alignment on the chart viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegendPosition {
    Top,
    Bottom,
    Left,
    Right,
    None,
}

/// A single series of numeric data points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Series {
    pub name: String,
    pub values: Vec<f64>,
    pub color: Option<String>,
}

impl Series {
    pub fn new<S: Into<String>>(name: S, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
            color: None,
        }
    }

    pub fn with_color<C: Into<String>>(mut self, color: C) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// Complete chart configuration and data model ready for rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartModel {
    pub id: String,
    pub title: String,
    pub chart_type: ChartType,
    pub categories: Vec<String>,
    pub series: Vec<Series>,
    pub source_range: Option<CellRange>,
    pub legend_position: LegendPosition,
    pub show_gridlines: bool,
    pub show_data_labels: bool,
}

impl ChartModel {
    pub fn new<S: Into<String>>(title: S, chart_type: ChartType) -> Self {
        Self {
            id: uuid_placeholder(),
            title: title.into(),
            chart_type,
            categories: Vec::new(),
            series: Vec::new(),
            source_range: None,
            legend_position: LegendPosition::Right,
            show_gridlines: true,
            show_data_labels: false,
        }
    }

    /// Sets the category labels (X-axis).
    pub fn with_categories(mut self, categories: Vec<String>) -> Self {
        self.categories = categories;
        self
    }

    /// Adds a data series.
    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Total number of data categories.
    pub fn category_count(&self) -> usize {
        self.categories.len()
    }

    /// Total number of series.
    pub fn series_count(&self) -> usize {
        self.series.len()
    }

    /// Computes minimum and maximum values across all series.
    pub fn value_range(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for s in &self.series {
            for &val in &s.values {
                if val.is_finite() {
                    min = min.min(val);
                    max = max.max(val);
                }
            }
        }

        if min.is_infinite() {
            (0.0, 100.0)
        } else {
            (min, max)
        }
    }
}

fn uuid_placeholder() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("chart_{:x}", nanos)
}

/// Extracts a complete `ChartModel` from a rectangular worksheet cell range.
///
/// Convention:
/// - If first row contains text labels, they are treated as Series names (headers).
/// - If first column contains non-numeric strings, it is treated as category labels (X-axis).
/// - Remaining rectangular cells are parsed into numerical series.
pub fn extract_chart_from_range(
    sheet: &Worksheet,
    range: CellRange,
    chart_type: ChartType,
    title: Option<&str>,
) -> Result<ChartModel, ChartError> {
    if range.start.row > range.end.row || range.start.col > range.end.col {
        return Err(ChartError::EmptyRange);
    }

    let num_rows = (range.end.row - range.start.row + 1) as usize;
    let num_cols = (range.end.col - range.start.col + 1) as usize;

    if num_rows == 0 || num_cols == 0 {
        return Err(ChartError::EmptyRange);
    }

    let chart_title = title.unwrap_or("Chart").to_string();
    let mut model = ChartModel::new(chart_title, chart_type);
    model.source_range = Some(range);

    // Detect if top row is headers
    let has_header_row = num_rows > 1
        && (range.start.col..=range.end.col).any(|c| {
            let coord = CellCoord::must_new(range.start.row, c);
            matches!(sheet.get_cell_value(coord), CellValue::Text(_))
        });

    // Detect if first column is category labels
    let data_start_row = if has_header_row {
        range.start.row + 1
    } else {
        range.start.row
    };

    let has_category_col = num_cols > 1
        && (data_start_row..=range.end.row).any(|r| {
            let coord = CellCoord::must_new(r, range.start.col);
            matches!(sheet.get_cell_value(coord), CellValue::Text(_))
        });

    let data_start_col = if has_category_col {
        range.start.col + 1
    } else {
        range.start.col
    };

    // Extract categories
    let mut categories = Vec::new();
    for r in data_start_row..=range.end.row {
        if has_category_col {
            let coord = CellCoord::must_new(r, range.start.col);
            categories.push(sheet.get_cell_value(coord).to_display_string());
        } else {
            categories.push(format!("Row {}", r));
        }
    }
    model.categories = categories;

    // Extract numeric series for each data column
    for c in data_start_col..=range.end.col {
        let series_name = if has_header_row {
            let coord = CellCoord::must_new(range.start.row, c);
            sheet.get_cell_value(coord).to_display_string()
        } else {
            format!("Series {}", c - data_start_col + 1)
        };

        let mut values = Vec::new();
        for r in data_start_row..=range.end.row {
            let coord = CellCoord::must_new(r, c);
            let val = sheet.get_cell_value(coord).as_number().unwrap_or(0.0);
            values.push(val);
        }

        model.add_series(Series::new(series_name, values));
    }

    if model.series.is_empty() {
        return Err(ChartError::NoNumericData(range.to_a1()));
    }

    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::SheetId;

    #[test]
    fn test_extract_bar_chart_from_table() {
        let mut sheet = Worksheet::new(SheetId::new("s1"), "Data");

        // Headers: Month | Revenue | Expenses
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Month".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Text("Revenue".into()));
        sheet.set_cell_value(
            CellCoord::must_new(1, 3),
            CellValue::Text("Expenses".into()),
        );

        // Data rows
        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Jan".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 2), CellValue::Number(100.0));
        sheet.set_cell_value(CellCoord::must_new(2, 3), CellValue::Number(60.0));

        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Text("Feb".into()));
        sheet.set_cell_value(CellCoord::must_new(3, 2), CellValue::Number(150.0));
        sheet.set_cell_value(CellCoord::must_new(3, 3), CellValue::Number(80.0));

        sheet.set_cell_value(CellCoord::must_new(4, 1), CellValue::Text("Mar".into()));
        sheet.set_cell_value(CellCoord::must_new(4, 2), CellValue::Number(200.0));
        sheet.set_cell_value(CellCoord::must_new(4, 3), CellValue::Number(110.0));

        let range = CellRange::new(CellCoord::must_new(1, 1), CellCoord::must_new(4, 3));
        let chart =
            extract_chart_from_range(&sheet, range, ChartType::Bar, Some("Financial Overview"))
                .unwrap();

        assert_eq!(chart.title, "Financial Overview");
        assert_eq!(chart.chart_type, ChartType::Bar);
        assert_eq!(chart.categories, vec!["Jan", "Feb", "Mar"]);
        assert_eq!(chart.series.len(), 2);

        assert_eq!(chart.series[0].name, "Revenue");
        assert_eq!(chart.series[0].values, vec![100.0, 150.0, 200.0]);

        assert_eq!(chart.series[1].name, "Expenses");
        assert_eq!(chart.series[1].values, vec![60.0, 80.0, 110.0]);

        let (min, max) = chart.value_range();
        assert_eq!(min, 60.0);
        assert_eq!(max, 200.0);
    }

    #[test]
    fn test_extract_pie_chart_single_series() {
        let mut sheet = Worksheet::new(SheetId::new("s2"), "Share");

        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Segment".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Text("Share".into()));

        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Direct".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 2), CellValue::Number(45.0));

        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Text("Organic".into()));
        sheet.set_cell_value(CellCoord::must_new(3, 2), CellValue::Number(35.0));

        sheet.set_cell_value(
            CellCoord::must_new(4, 1),
            CellValue::Text("Referral".into()),
        );
        sheet.set_cell_value(CellCoord::must_new(4, 2), CellValue::Number(20.0));

        let range = CellRange::new(CellCoord::must_new(1, 1), CellCoord::must_new(4, 2));
        let chart =
            extract_chart_from_range(&sheet, range, ChartType::Pie, Some("Traffic Distribution"))
                .unwrap();

        assert_eq!(chart.chart_type, ChartType::Pie);
        assert_eq!(chart.categories, vec!["Direct", "Organic", "Referral"]);
        assert_eq!(chart.series.len(), 1);
        assert_eq!(chart.series[0].values, vec![45.0, 35.0, 20.0]);
    }
}
