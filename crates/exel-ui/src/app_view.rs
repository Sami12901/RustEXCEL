use crate::format_dialog::FormatCellsDialog;
use crate::formula_bar::FormulaBar;
use crate::grid::SpreadsheetGrid;
use crate::ribbon::Ribbon;
use crate::sheet_tabs::SheetTabsBar;
use crate::state::{SpreadsheetAction, SpreadsheetUiState};
use egui::{CentralPanel, Panel, Ui};
use exel_shortcuts::{ShortcutAction, ShortcutRegistry};
use exel_workbook::Workbook;

/// Top-level integrated spreadsheet UI view component.
pub struct SpreadsheetAppView<'a> {
    workbook: &'a mut Workbook,
    state: &'a mut SpreadsheetUiState,
    shortcuts: &'a ShortcutRegistry,
}

impl<'a> SpreadsheetAppView<'a> {
    pub fn new(
        workbook: &'a mut Workbook,
        state: &'a mut SpreadsheetUiState,
        shortcuts: &'a ShortcutRegistry,
    ) -> Self {
        Self {
            workbook,
            state,
            shortcuts,
        }
    }

    /// Renders the complete spreadsheet application interface (Ribbon, Formula Bar, Grid, Tabs, Status Bar).
    pub fn show(self, ui: &mut Ui) -> Vec<SpreadsheetAction> {
        let mut actions = Vec::new();
        let ctx = ui.ctx().clone();

        // 1. Process global keyboard shortcuts
        ctx.input(|i| {
            if let Some(action) = self.shortcuts.matches_input(i) {
                match action {
                    ShortcutAction::NewWorkbook => actions.push(SpreadsheetAction::NewFile),
                    ShortcutAction::OpenWorkbook => actions.push(SpreadsheetAction::OpenFile),
                    ShortcutAction::SaveWorkbook => actions.push(SpreadsheetAction::SaveFile),
                    ShortcutAction::SaveAsWorkbook => actions.push(SpreadsheetAction::SaveAsFile),
                    ShortcutAction::Undo => actions.push(SpreadsheetAction::Undo),
                    ShortcutAction::Redo => actions.push(SpreadsheetAction::Redo),
                    ShortcutAction::Cut => actions.push(SpreadsheetAction::Cut),
                    ShortcutAction::Copy => actions.push(SpreadsheetAction::Copy),
                    ShortcutAction::Paste => actions.push(SpreadsheetAction::Paste),
                    ShortcutAction::FormatCells => self.state.show_format_cells_dialog = true,
                    ShortcutAction::Recalculate => actions.push(SpreadsheetAction::Recalculate),
                    _ => {}
                }
            }
        });

        // 2. Format Cells modal dialog
        if let Some(dialog_action) = FormatCellsDialog::new(self.state).show(&ctx) {
            actions.push(dialog_action);
        }

        // 3. Ribbon Panel (Top)
        Panel::top("ribbon_panel").resizable(false).show(ui, |ui| {
            let ribbon_actions = Ribbon::new(self.state).show(ui);
            actions.extend(ribbon_actions);
        });

        // 4. Formula Bar Panel (Below Ribbon)
        if self.state.show_formula_bar {
            Panel::top("formula_bar_panel")
                .resizable(false)
                .show(ui, |ui| {
                    let sheet = self.workbook.active_sheet();
                    let formula_actions = FormulaBar::new(sheet, self.state).show(ui);
                    actions.extend(formula_actions);
                });
        }

        // 5. Sheet Tabs and Status Bar (Bottom)
        Panel::bottom("bottom_bar_panel")
            .resizable(false)
            .show(ui, |ui| {
                let tab_actions = SheetTabsBar::new(self.workbook, self.state).show(ui);
                actions.extend(tab_actions);
            });

        // 6. Virtualized Spreadsheet Grid (Center)
        CentralPanel::default().show(ui, |ui| {
            let sheet = self.workbook.active_sheet();
            let (_resp, grid_actions) = SpreadsheetGrid::new(sheet, self.state).show(ui);
            actions.extend(grid_actions);
        });

        actions
    }
}
