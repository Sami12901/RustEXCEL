use crate::state::{SpreadsheetAction, SpreadsheetUiState, ViewMode};
use crate::theme::SpreadsheetTheme;
use egui::{vec2, Color32, Key, Layout, RichText, Slider, Ui};
use exel_cell::CellCoord;
use exel_workbook::Workbook;

/// Bottom Sheet Tabs Bar & Status Bar component.
pub struct SheetTabsBar<'a> {
    workbook: &'a Workbook,
    state: &'a mut SpreadsheetUiState,
}

impl<'a> SheetTabsBar<'a> {
    pub fn new(workbook: &'a Workbook, state: &'a mut SpreadsheetUiState) -> Self {
        Self { workbook, state }
    }

    pub fn show(self, ui: &mut Ui) -> Vec<SpreadsheetAction> {
        let mut actions = Vec::new();

        ui.vertical(|ui| {
            // 1. Sheet Tabs Row
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;

                // Tab navigation arrows
                let _ = ui.button("◀");
                let _ = ui.button("▶");
                ui.separator();

                // List of Sheets
                for (idx, sheet) in self.workbook.sheets.iter().enumerate() {
                    let is_active = idx == self.workbook.active_sheet_index;
                    let sheet_id = sheet.id.clone();

                    if self.state.renaming_sheet_id.as_ref() == Some(&sheet_id) {
                        // Inline rename input
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.state.rename_buffer)
                                .desired_width(70.0),
                        );
                        resp.request_focus();

                        if resp.lost_focus() || ui.input(|i| i.key_pressed(Key::Enter)) {
                            let new_name = self.state.rename_buffer.trim().to_string();
                            if !new_name.is_empty() {
                                actions.push(SpreadsheetAction::RenameSheet {
                                    id: sheet_id,
                                    new_name,
                                });
                            }
                            self.state.renaming_sheet_id = None;
                        } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                            self.state.renaming_sheet_id = None;
                        }
                    } else {
                        // Tab button
                        let text = if is_active {
                            RichText::new(&sheet.name)
                                .color(SpreadsheetTheme::EXCEL_GREEN)
                                .strong()
                        } else {
                            RichText::new(&sheet.name).color(SpreadsheetTheme::HEADER_TEXT)
                        };

                        let btn = egui::Button::new(text)
                            .fill(if is_active {
                                Color32::WHITE
                            } else {
                                SpreadsheetTheme::HEADER_BG
                            })
                            .min_size(vec2(70.0, 24.0));

                        let resp = ui.add(btn);

                        if resp.clicked() {
                            actions.push(SpreadsheetAction::SelectSheet(sheet_id.clone()));
                        }

                        if resp.double_clicked() {
                            self.state.renaming_sheet_id = Some(sheet_id.clone());
                            self.state.rename_buffer = sheet.name.clone();
                        }

                        // Right click context menu for sheet tab
                        resp.context_menu(|ui| {
                            if ui.button("Rename").clicked() {
                                self.state.renaming_sheet_id = Some(sheet_id.clone());
                                self.state.rename_buffer = sheet.name.clone();
                                ui.close();
                            }
                            if self.workbook.sheet_count() > 1 && ui.button("Delete").clicked() {
                                actions.push(SpreadsheetAction::DeleteSheet(sheet_id.clone()));
                                ui.close();
                            }
                        });
                    }
                }

                // Add Sheet (+) Button
                if ui
                    .button(
                        RichText::new(" ➕ ")
                            .strong()
                            .color(SpreadsheetTheme::EXCEL_GREEN),
                    )
                    .on_hover_text("Insert Worksheet")
                    .clicked()
                {
                    actions.push(SpreadsheetAction::AddSheet);
                }
            });

            ui.separator();

            // 2. Status Bar Row with Live Range Statistics and Zoom Slider
            ui.horizontal(|ui| {
                // Left: Ready status
                ui.label(RichText::new(&self.state.status_message).size(11.0));

                ui.separator();

                // Center-Left: Live Range Statistics
                let stats = self.calculate_selection_statistics();
                if let Some(stats) = stats {
                    ui.label(
                        RichText::new(format!(
                            "AVERAGE: {:.2}   COUNT: {}   SUM: {:.2}",
                            stats.average, stats.count, stats.sum
                        ))
                        .size(11.0)
                        .strong()
                        .color(SpreadsheetTheme::STATUS_BAR_TEXT),
                    );
                }

                // Right alignment for View modes and Zoom
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    // Zoom Percentage Label
                    ui.label(
                        RichText::new(format!("{:.0}%", self.state.zoom * 100.0))
                            .size(11.0)
                            .monospace(),
                    );

                    // Zoom In Button (+)
                    if ui.button("+").clicked() {
                        self.state.zoom = (self.state.zoom + 0.1).min(2.0);
                    }

                    // Zoom Slider (50% to 200%)
                    ui.add(
                        Slider::new(&mut self.state.zoom, 0.5..=2.0)
                            .show_value(false)
                            .logarithmic(false),
                    );

                    // Zoom Out Button (-)
                    if ui.button("-").clicked() {
                        self.state.zoom = (self.state.zoom - 0.1).max(0.5);
                    }

                    ui.separator();

                    // View Mode Quick Switch Icons
                    if ui
                        .selectable_label(self.state.view_mode == ViewMode::Normal, "🗖")
                        .on_hover_text("Normal View")
                        .clicked()
                    {
                        self.state.view_mode = ViewMode::Normal;
                    }
                    if ui
                        .selectable_label(self.state.view_mode == ViewMode::PageLayout, "📄")
                        .on_hover_text("Page Layout View")
                        .clicked()
                    {
                        self.state.view_mode = ViewMode::PageLayout;
                    }
                    if ui
                        .selectable_label(self.state.view_mode == ViewMode::PageBreakPreview, "✂")
                        .on_hover_text("Page Break Preview")
                        .clicked()
                    {
                        self.state.view_mode = ViewMode::PageBreakPreview;
                    }
                });
            });
        });

        actions
    }

    /// Computes live aggregate statistics (Average, Count, Sum) for all populated cells in current selection.
    fn calculate_selection_statistics(&self) -> Option<RangeStatistics> {
        let sel = self.state.selection;
        let sheet = self.workbook.active_sheet();

        let mut sum = 0.0;
        let mut count = 0usize;
        let mut num_count = 0usize;

        // Iterate over cells in the selection range (capped to reasonable limit if range is huge)
        let row_count = sel.row_count().min(10_000);
        let col_count = sel.col_count().min(256);

        for r_offset in 0..row_count {
            let row = sel.start.row + r_offset;
            for c_offset in 0..col_count {
                let col = sel.start.col + c_offset;
                if let Some(cell) = sheet.get_cell(CellCoord::must_new(row, col)) {
                    if !cell.is_empty() {
                        count += 1;
                        if let Some(n) = cell.value.as_number() {
                            sum += n;
                            num_count += 1;
                        }
                    }
                }
            }
        }

        if count > 0 {
            let average = if num_count > 0 {
                sum / (num_count as f64)
            } else {
                0.0
            };
            Some(RangeStatistics {
                sum,
                average,
                count,
                numerical_count: num_count,
            })
        } else {
            None
        }
    }
}

pub struct RangeStatistics {
    pub sum: f64,
    pub average: f64,
    pub count: usize,
    pub numerical_count: usize,
}
