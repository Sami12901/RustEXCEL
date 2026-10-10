use egui::{Color32, Stroke};

/// Design system constants and color palette matching professional spreadsheet styling.
pub struct SpreadsheetTheme;

impl SpreadsheetTheme {
    // Brand Colors
    pub const EXCEL_GREEN: Color32 = Color32::from_rgb(16, 124, 65);
    pub const EXCEL_GREEN_DARK: Color32 = Color32::from_rgb(12, 90, 47);
    pub const EXCEL_GREEN_LIGHT: Color32 = Color32::from_rgb(22, 163, 74);
    pub const SELECTION_FILL: Color32 = Color32::from_rgba_premultiplied(16, 124, 65, 32);
    pub const AUTOFILL_FILL: Color32 = Color32::from_rgba_premultiplied(16, 124, 65, 20);

    // Grid & Header Colors
    pub const GRID_LINE: Color32 = Color32::from_rgb(218, 220, 224);
    pub const HEADER_BG: Color32 = Color32::from_rgb(243, 244, 246);
    pub const HEADER_BG_ACTIVE: Color32 = Color32::from_rgb(220, 237, 225);
    pub const HEADER_TEXT: Color32 = Color32::from_rgb(70, 75, 80);
    pub const HEADER_TEXT_ACTIVE: Color32 = Color32::from_rgb(16, 124, 65);
    pub const HEADER_BORDER: Color32 = Color32::from_rgb(209, 213, 219);
    pub const CELL_BG: Color32 = Color32::from_rgb(255, 255, 255);
    pub const CELL_TEXT: Color32 = Color32::from_rgb(30, 41, 59);

    // Ribbon & UI Chrome
    pub const RIBBON_BG: Color32 = Color32::from_rgb(248, 250, 252);
    pub const RIBBON_TAB_ACTIVE_BG: Color32 = Color32::from_rgb(255, 255, 255);
    pub const RIBBON_SEPARATOR: Color32 = Color32::from_rgb(226, 232, 240);
    pub const STATUS_BAR_BG: Color32 = Color32::from_rgb(241, 245, 249);
    pub const STATUS_BAR_TEXT: Color32 = Color32::from_rgb(71, 85, 105);

    // Dimensions
    pub const DEFAULT_COL_WIDTH: f32 = 80.0;
    pub const DEFAULT_ROW_HEIGHT: f32 = 22.0;
    pub const ROW_HEADER_WIDTH: f32 = 50.0;
    pub const COL_HEADER_HEIGHT: f32 = 24.0;
    pub const FILL_HANDLE_SIZE: f32 = 6.0;
    pub const RESIZE_HANDLE_SLOP: f32 = 4.0;

    // Strokes
    pub fn grid_stroke() -> Stroke {
        Stroke::new(1.0, Self::GRID_LINE)
    }

    pub fn header_border_stroke() -> Stroke {
        Stroke::new(1.0, Self::HEADER_BORDER)
    }

    pub fn selection_border_stroke() -> Stroke {
        Stroke::new(1.5, Self::EXCEL_GREEN)
    }

    pub fn active_cell_stroke() -> Stroke {
        Stroke::new(2.0, Self::EXCEL_GREEN)
    }

    pub fn autofill_dashed_stroke() -> Stroke {
        Stroke::new(1.5, Self::EXCEL_GREEN)
    }
}
