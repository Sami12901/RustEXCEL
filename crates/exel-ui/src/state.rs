use exel_cell::{CellCoord, CellRange, CellValue, SheetId};
use exel_formatting::{CellStyle, StyleTable};
use serde::{Deserialize, Serialize};

/// Active Ribbon category tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RibbonTab {
    File,
    Home,
    Insert,
    PageLayout,
    Formulas,
    Data,
    Review,
    View,
    Developer,
    Help,
}

/// View display modes in spreadsheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewMode {
    Normal,
    PageBreakPreview,
    PageLayout,
}

/// Standard aggregate function types for AutoSum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AutoSumFunction {
    Sum,
    Average,
    Count,
    Max,
    Min,
}

impl AutoSumFunction {
    pub fn formula_name(&self) -> &'static str {
        match self {
            AutoSumFunction::Sum => "SUM",
            AutoSumFunction::Average => "AVERAGE",
            AutoSumFunction::Count => "COUNT",
            AutoSumFunction::Max => "MAX",
            AutoSumFunction::Min => "MIN",
        }
    }
}

/// Tab options in the Format Cells modal dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellFormatTab {
    Number,
    Alignment,
    Font,
    Border,
    Fill,
}

/// Actions produced by the UI components for the application controller.
#[derive(Debug, Clone, PartialEq)]
pub enum SpreadsheetAction {
    SetCellValue {
        coord: CellCoord,
        value: CellValue,
    },
    SetCellFormula {
        coord: CellCoord,
        formula: String,
    },
    ClearCells {
        range: CellRange,
    },
    ApplyStyle {
        range: CellRange,
        style: CellStyle,
    },
    AutoSum {
        range: CellRange,
        func: AutoSumFunction,
    },
    AutoFill {
        source: CellRange,
        target: CellRange,
    },
    SelectSheet(SheetId),
    AddSheet,
    RenameSheet {
        id: SheetId,
        new_name: String,
    },
    DeleteSheet(SheetId),
    ResizeColumn {
        col: u32,
        width: f32,
    },
    ResizeRow {
        row: u32,
        height: f32,
    },
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Recalculate,
    NewFile,
    OpenFile,
    SaveFile,
    SaveAsFile,
    ExportCsv,
    ExportXlsx,
}

/// Central state tracking for the UI session.
#[derive(Debug, Clone)]
pub struct SpreadsheetUiState {
    // Selection and Navigation
    pub active_cell: CellCoord,
    pub selection: CellRange,
    pub selection_anchor: CellCoord,
    pub is_dragging_selection: bool,

    // Inline Editing
    pub is_editing: bool,
    pub edit_buffer: String,
    pub edit_focus_requested: bool,

    // Formula Bar & Name Box
    pub formula_buffer: String,
    pub name_box_buffer: String,
    pub formula_expanded: bool,

    // Viewport & Scrolling
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub zoom: f32,
    pub view_mode: ViewMode,
    pub show_gridlines: bool,
    pub show_headings: bool,
    pub show_formula_bar: bool,

    // Ribbon
    pub active_ribbon_tab: RibbonTab,

    // Resizing
    pub col_resizing: Option<(u32, f32, f32)>, // (col, initial_width, start_mouse_x)
    pub row_resizing: Option<(u32, f32, f32)>, // (row, initial_height, start_mouse_y)

    // AutoFill Handle
    pub is_dragging_fill_handle: bool,
    pub fill_handle_target_range: Option<CellRange>,

    // Sheet Management
    pub renaming_sheet_id: Option<SheetId>,
    pub rename_buffer: String,

    // Dialogs
    pub show_format_cells_dialog: bool,
    pub format_cells_active_tab: CellFormatTab,
    pub pending_cell_style: CellStyle,

    // Style System
    pub style_table: StyleTable,

    // Status Message
    pub status_message: String,
}

impl Default for SpreadsheetUiState {
    fn default() -> Self {
        let origin = CellCoord::must_new(1, 1);
        Self {
            active_cell: origin,
            selection: CellRange::single_cell(origin),
            selection_anchor: origin,
            is_dragging_selection: false,

            is_editing: false,
            edit_buffer: String::new(),
            edit_focus_requested: false,

            formula_buffer: String::new(),
            name_box_buffer: origin.to_a1(),
            formula_expanded: false,

            scroll_x: 0.0,
            scroll_y: 0.0,
            zoom: 1.0,
            view_mode: ViewMode::Normal,
            show_gridlines: true,
            show_headings: true,
            show_formula_bar: true,

            active_ribbon_tab: RibbonTab::Home,

            col_resizing: None,
            row_resizing: None,

            is_dragging_fill_handle: false,
            fill_handle_target_range: None,

            renaming_sheet_id: None,
            rename_buffer: String::new(),

            show_format_cells_dialog: false,
            format_cells_active_tab: CellFormatTab::Number,
            pending_cell_style: CellStyle::default(),

            style_table: StyleTable::new(),

            status_message: "Ready".to_string(),
        }
    }
}

impl SpreadsheetUiState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects a single cell and updates the Name Box and formula buffer.
    pub fn select_cell(&mut self, coord: CellCoord, formula_text: String) {
        self.active_cell = coord;
        self.selection = CellRange::single_cell(coord);
        self.selection_anchor = coord;
        self.name_box_buffer = coord.to_a1();
        self.formula_buffer = formula_text;
    }

    /// Extends selection marquee from current anchor to target coordinate.
    pub fn extend_selection(&mut self, target: CellCoord) {
        self.selection = CellRange::new(self.selection_anchor, target);
        self.name_box_buffer = if self.selection.start == self.selection.end {
            self.selection.start.to_a1()
        } else {
            self.selection.to_a1()
        };
    }

    /// Starts inline cell editing with initial string content.
    pub fn start_editing(&mut self, initial_text: String) {
        self.is_editing = true;
        self.edit_buffer = initial_text.clone();
        self.formula_buffer = initial_text;
        self.edit_focus_requested = true;
        self.status_message = "Edit".to_string();
    }

    /// Cancels active editing session without committing.
    pub fn cancel_editing(&mut self) {
        self.is_editing = false;
        self.edit_buffer.clear();
        self.status_message = "Ready".to_string();
    }

    /// Commits active edit buffer, returning the committed text.
    pub fn commit_editing(&mut self) -> String {
        let result = self.edit_buffer.clone();
        self.is_editing = false;
        self.edit_buffer.clear();
        self.status_message = "Ready".to_string();
        result
    }
}
