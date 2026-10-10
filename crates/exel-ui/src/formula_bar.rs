use crate::state::{SpreadsheetAction, SpreadsheetUiState};
use crate::theme::SpreadsheetTheme;
use egui::{vec2, Color32, Key, RichText, Sense, Ui};
use exel_cell::{CellCoord, CellValue};
use exel_workbook::Worksheet;

/// Formula Bar & Name Box component.
pub struct FormulaBar<'a> {
    worksheet: &'a Worksheet,
    state: &'a mut SpreadsheetUiState,
}

impl<'a> FormulaBar<'a> {
    pub fn new(worksheet: &'a Worksheet, state: &'a mut SpreadsheetUiState) -> Self {
        Self { worksheet, state }
    }

    pub fn show(self, ui: &mut Ui) -> Vec<SpreadsheetAction> {
        let mut actions = Vec::new();

        ui.horizontal(|ui| {
            // 1. Name Box (Address / Jump to Cell)
            ui.add_sized([75.0, 22.0], |ui: &mut Ui| {
                let name_box_response = ui.add(
                    egui::TextEdit::singleline(&mut self.state.name_box_buffer)
                        .hint_text("A1")
                        .clip_text(true),
                );

                if name_box_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    if let Ok(coord) = CellCoord::from_a1(&self.state.name_box_buffer) {
                        let text = self
                            .worksheet
                            .get_cell(coord)
                            .and_then(|c| {
                                c.formula
                                    .clone()
                                    .or_else(|| Some(c.value.to_display_string()))
                            })
                            .unwrap_or_default();
                        self.state.select_cell(coord, text);

                        // Ensure jumped cell is visible by adjusting scroll if needed
                        let target_x = (coord.col.saturating_sub(1) as f32)
                            * (SpreadsheetTheme::DEFAULT_COL_WIDTH * self.state.zoom);
                        let target_y = (coord.row.saturating_sub(1) as f32)
                            * (SpreadsheetTheme::DEFAULT_ROW_HEIGHT * self.state.zoom);

                        self.state.scroll_x = target_x;
                        self.state.scroll_y = target_y;
                    } else {
                        // Reset to current active cell on invalid entry
                        self.state.name_box_buffer = self.state.active_cell.to_a1();
                    }
                }
                name_box_response
            });

            // Vertical divider
            ui.separator();

            // 2. Action buttons: Cancel (X) and Commit (Checkmark)
            let is_modified = self.state.formula_buffer != self.get_current_cell_formula_or_value();

            // Cancel Button
            let cancel_btn = ui.add_enabled(
                is_modified || self.state.is_editing,
                egui::Button::new(
                    RichText::new("✕")
                        .color(if is_modified {
                            Color32::from_rgb(220, 38, 38)
                        } else {
                            Color32::GRAY
                        })
                        .strong(),
                )
                .min_size(vec2(22.0, 22.0)),
            );
            if cancel_btn.clicked() {
                self.state.formula_buffer = self.get_current_cell_formula_or_value();
                self.state.cancel_editing();
            }

            // Commit Button
            let commit_btn = ui.add_enabled(
                is_modified || self.state.is_editing,
                egui::Button::new(
                    RichText::new("✓")
                        .color(if is_modified {
                            SpreadsheetTheme::EXCEL_GREEN
                        } else {
                            Color32::GRAY
                        })
                        .strong(),
                )
                .min_size(vec2(22.0, 22.0)),
            );
            if commit_btn.clicked() {
                let committed = self.state.formula_buffer.trim().to_string();
                self.commit_formula(committed, &mut actions);
                self.state.cancel_editing();
            }

            // Function symbol (fx)
            let fx_label = ui.add(
                egui::Label::new(
                    RichText::new("fx")
                        .italics()
                        .strong()
                        .color(Color32::from_rgb(100, 116, 139)),
                )
                .sense(Sense::click()),
            );
            if fx_label.clicked() && !self.state.formula_buffer.starts_with('=') {
                self.state.formula_buffer = format!("={}", self.state.formula_buffer);
            }

            ui.separator();

            // 3. Formula Editor
            let editor_height = if self.state.formula_expanded {
                60.0
            } else {
                22.0
            };

            let remaining_w = ui.available_width() - 30.0;
            let formula_edit_resp =
                ui.add_sized([remaining_w.max(100.0), editor_height], |ui: &mut Ui| {
                    if self.state.formula_expanded {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.state.formula_buffer)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(3),
                        )
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state.formula_buffer)
                                .font(egui::TextStyle::Monospace),
                        )
                    }
                });

            // Handle Enter key inside formula bar to commit
            if formula_edit_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                let text = self.state.formula_buffer.trim().to_string();
                self.commit_formula(text, &mut actions);
            }

            // 4. Multiline expansion toggle chevron
            let expand_icon = if self.state.formula_expanded {
                "▲"
            } else {
                "▼"
            };
            if ui
                .button(RichText::new(expand_icon).size(10.0))
                .on_hover_text("Expand Formula Bar")
                .clicked()
            {
                self.state.formula_expanded = !self.state.formula_expanded;
            }
        });

        actions
    }

    fn get_current_cell_formula_or_value(&self) -> String {
        self.worksheet
            .get_cell(self.state.active_cell)
            .and_then(|c| {
                c.formula
                    .clone()
                    .or_else(|| Some(c.value.to_display_string()))
            })
            .unwrap_or_default()
    }

    fn commit_formula(&self, text: String, actions: &mut Vec<SpreadsheetAction>) {
        let coord = self.state.active_cell;
        let trimmed = text.trim();

        if trimmed.starts_with('=') {
            actions.push(SpreadsheetAction::SetCellFormula {
                coord,
                formula: trimmed.to_string(),
            });
        } else if trimmed.is_empty() {
            actions.push(SpreadsheetAction::SetCellValue {
                coord,
                value: CellValue::Empty,
            });
        } else if let Ok(n) = trimmed.parse::<f64>() {
            actions.push(SpreadsheetAction::SetCellValue {
                coord,
                value: CellValue::Number(n),
            });
        } else if trimmed.eq_ignore_ascii_case("true") {
            actions.push(SpreadsheetAction::SetCellValue {
                coord,
                value: CellValue::Boolean(true),
            });
        } else if trimmed.eq_ignore_ascii_case("false") {
            actions.push(SpreadsheetAction::SetCellValue {
                coord,
                value: CellValue::Boolean(false),
            });
        } else {
            actions.push(SpreadsheetAction::SetCellValue {
                coord,
                value: CellValue::Text(text),
            });
        }
    }
}
