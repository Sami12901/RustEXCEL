//! Print configuration, page setup, and print preview generation for Rust EXCEL.
//!
//! Provides `PageSetup` (margins, orientation, paper size, scaling, headers/footers),
//! pagination calculation, and text-based print preview generation.

use exel_cell::{CellCoord, CellRange};
use exel_core::col_to_name;
use exel_workbook::Worksheet;
use serde::{Deserialize, Serialize};

/// Document page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

/// Standard paper sizes supported by spreadsheet engines.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PaperSize {
    Letter,
    Legal,
    A4,
    A3,
    Tabloid,
    Custom {
        width_inches: f32,
        height_inches: f32,
    },
}

impl PaperSize {
    /// Returns page dimensions (width, height) in inches taking orientation into account.
    pub fn dimensions_inches(&self, orientation: PageOrientation) -> (f32, f32) {
        let (w, h) = match self {
            Self::Letter => (8.5, 11.0),
            Self::Legal => (8.5, 14.0),
            Self::A4 => (8.27, 11.69),
            Self::A3 => (11.69, 16.54),
            Self::Tabloid => (11.0, 17.0),
            Self::Custom {
                width_inches,
                height_inches,
            } => (*width_inches, *height_inches),
        };

        match orientation {
            PageOrientation::Portrait => (w, h),
            PageOrientation::Landscape => (h, w),
        }
    }
}

/// Page margins specified in inches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageMargins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
    pub header: f32,
    pub footer: f32,
}

impl PageMargins {
    pub fn normal() -> Self {
        Self {
            top: 0.75,
            bottom: 0.75,
            left: 0.7,
            right: 0.7,
            header: 0.3,
            footer: 0.3,
        }
    }

    pub fn wide() -> Self {
        Self {
            top: 1.0,
            bottom: 1.0,
            left: 1.0,
            right: 1.0,
            header: 0.5,
            footer: 0.5,
        }
    }

    pub fn narrow() -> Self {
        Self {
            top: 0.5,
            bottom: 0.5,
            left: 0.25,
            right: 0.25,
            header: 0.3,
            footer: 0.3,
        }
    }
}

/// Scaling behavior for print layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scaling {
    /// Fit content onto an exact number of pages.
    FitToPages { wide: u32, tall: u32 },
    /// Percentage zoom scaling (e.g. 100% = 100).
    Percent(u32),
}

/// Print rendering options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintOptions {
    pub print_gridlines: bool,
    pub print_headings: bool,
    pub black_and_white: bool,
    pub center_horizontally: bool,
    pub center_vertically: bool,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            print_gridlines: true,
            print_headings: true,
            black_and_white: false,
            center_horizontally: false,
            center_vertically: false,
        }
    }
}

/// Complete page setup configuration for printing worksheets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageSetup {
    pub orientation: PageOrientation,
    pub paper_size: PaperSize,
    pub margins: PageMargins,
    pub scaling: Scaling,
    pub print_options: PrintOptions,
    pub print_area: Option<CellRange>,
    pub header_center: Option<String>,
    pub footer_center: Option<String>,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            orientation: PageOrientation::Portrait,
            paper_size: PaperSize::Letter,
            margins: PageMargins::normal(),
            scaling: Scaling::Percent(100),
            print_options: PrintOptions::default(),
            print_area: None,
            header_center: None,
            footer_center: Some("Page &[Page]".into()),
        }
    }
}

impl PageSetup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_orientation(mut self, orientation: PageOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_paper_size(mut self, size: PaperSize) -> Self {
        self.paper_size = size;
        self
    }

    pub fn with_margins(mut self, margins: PageMargins) -> Self {
        self.margins = margins;
        self
    }

    pub fn with_scaling(mut self, scaling: Scaling) -> Self {
        self.scaling = scaling;
        self
    }

    pub fn with_header<S: Into<String>>(mut self, header: S) -> Self {
        self.header_center = Some(header.into());
        self
    }

    pub fn with_footer<S: Into<String>>(mut self, footer: S) -> Self {
        self.footer_center = Some(footer.into());
        self
    }

    pub fn with_print_area(mut self, area: CellRange) -> Self {
        self.print_area = Some(area);
        self
    }
}

/// Generates a structured ASCII print preview of the worksheet conforming to `PageSetup`.
pub fn generate_print_preview_text(sheet: &Worksheet, setup: &PageSetup) -> String {
    let range = setup
        .print_area
        .or_else(|| sheet.cells.used_range())
        .unwrap_or_else(|| CellRange::single_cell(CellCoord::must_new(1, 1)));

    let (pw, ph) = setup.paper_size.dimensions_inches(setup.orientation);
    let orient_str = match setup.orientation {
        PageOrientation::Portrait => "Portrait",
        PageOrientation::Landscape => "Landscape",
    };

    let mut out = String::new();

    // Banner / Metadata Header
    out.push_str(
        "================================================================================\n",
    );
    out.push_str(&format!(
        " PRINT PREVIEW: {} (Sheet: '{}')\n",
        setup.header_center.as_deref().unwrap_or("Untitled"),
        sheet.name
    ));
    out.push_str(&format!(
        " Paper: {:?} ({:.1}\" x {:.1}\") | Orientation: {} | Area: {}\n",
        setup.paper_size,
        pw,
        ph,
        orient_str,
        range.to_a1()
    ));
    out.push_str(
        "================================================================================\n",
    );

    // Column widths
    let mut col_widths: Vec<usize> = Vec::new();
    for c in range.start.col..=range.end.col {
        let col_name = col_to_name(c);
        let mut max_len = col_name.len();
        for r in range.start.row..=range.end.row {
            if let Ok(coord) = CellCoord::new(r, c) {
                let text = sheet.get_cell_value(coord).to_display_string();
                max_len = max_len.max(text.len());
            }
        }
        col_widths.push(max_len.max(8)); // minimum 8 chars per column
    }

    let row_header_width = 6;

    // Header row with column letters
    if setup.print_options.print_headings {
        out.push_str(&format!("{:>width$} |", "Row", width = row_header_width));
        for (idx, c) in (range.start.col..=range.end.col).enumerate() {
            let col_name = col_to_name(c);
            let width = col_widths[idx];
            out.push_str(&format!(" {:^width$} |", col_name, width = width));
        }
        out.push('\n');

        // Divider
        out.push_str(&"-".repeat(row_header_width + 1));
        out.push('+');
        for &w in &col_widths {
            out.push_str(&"-".repeat(w + 2));
            out.push('+');
        }
        out.push('\n');
    }

    // Data rows
    for r in range.start.row..=range.end.row {
        if setup.print_options.print_headings {
            out.push_str(&format!("{:>width$} |", r, width = row_header_width));
        }
        for (idx, c) in (range.start.col..=range.end.col).enumerate() {
            let width = col_widths[idx];
            let cell_text = if let Ok(coord) = CellCoord::new(r, c) {
                sheet.get_cell_value(coord).to_display_string()
            } else {
                String::new()
            };

            if setup.print_options.print_gridlines {
                out.push_str(&format!(" {:<width$} |", cell_text, width = width));
            } else {
                out.push_str(&format!(" {:<width$}  ", cell_text, width = width));
            }
        }
        out.push('\n');
    }

    // Footer
    out.push_str(
        "--------------------------------------------------------------------------------\n",
    );
    if let Some(footer) = &setup.footer_center {
        out.push_str(&format!(" Footer: {}\n", footer));
    }
    out.push_str(
        "================================================================================\n",
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::{CellValue, SheetId};

    #[test]
    fn test_page_setup_defaults_and_dimensions() {
        let setup = PageSetup::default();
        assert_eq!(setup.orientation, PageOrientation::Portrait);
        let (w, h) = setup.paper_size.dimensions_inches(setup.orientation);
        assert_eq!(w, 8.5);
        assert_eq!(h, 11.0);

        let landscape_setup = setup.with_orientation(PageOrientation::Landscape);
        let (lw, lh) = landscape_setup
            .paper_size
            .dimensions_inches(landscape_setup.orientation);
        assert_eq!(lw, 11.0);
        assert_eq!(lh, 8.5);
    }

    #[test]
    fn test_print_preview_generation() {
        let mut sheet = Worksheet::new(SheetId::new("s_print"), "Report");
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Item".into()));
        sheet.set_cell_value(CellCoord::must_new(1, 2), CellValue::Text("Price".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("Apples".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 2), CellValue::Number(2.50));

        let setup = PageSetup::new()
            .with_header("Weekly Grocery Summary")
            .with_footer("Page 1 of 1");

        let preview = generate_print_preview_text(&sheet, &setup);

        assert!(preview.contains("Weekly Grocery Summary"));
        assert!(preview.contains("Report"));
        assert!(preview.contains("Apples"));
        assert!(preview.contains("2.5"));
        assert!(preview.contains("Footer: Page 1 of 1"));
    }
}
