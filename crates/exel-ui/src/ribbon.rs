use crate::state::{AutoSumFunction, RibbonTab, SpreadsheetAction, SpreadsheetUiState, ViewMode};
use crate::theme::SpreadsheetTheme;
use egui::{Color32, ComboBox, Response, RichText, Ui};
use exel_formatting::{
    Color as FmtColor, Fill, HorizontalAlignment, NumberFormat,
};

/// Professional Excel Ribbon Interface.
pub struct Ribbon<'a> {
    state: &'a mut SpreadsheetUiState,
}

impl<'a> Ribbon<'a> {
    pub fn new(state: &'a mut SpreadsheetUiState) -> Self {
        Self { state }
    }

    pub fn show(mut self, ui: &mut Ui) -> Vec<SpreadsheetAction> {
        let mut actions = Vec::new();

        ui.vertical(|ui| {
            // 1. Ribbon Tab Headers
            self.render_tab_headers(ui);

            ui.separator();

            // 2. Ribbon Content Panel for Active Tab
            ui.add_sized([ui.available_width(), 90.0], |ui: &mut Ui| {
                match self.state.active_ribbon_tab {
                    RibbonTab::File => self.render_file_tab(ui, &mut actions),
                    RibbonTab::Home => self.render_home_tab(ui, &mut actions),
                    RibbonTab::Insert => self.render_insert_tab(ui, &mut actions),
                    RibbonTab::PageLayout => self.render_page_layout_tab(ui, &mut actions),
                    RibbonTab::Formulas => self.render_formulas_tab(ui, &mut actions),
                    RibbonTab::Data => self.render_data_tab(ui, &mut actions),
                    RibbonTab::Review => self.render_review_tab(ui, &mut actions),
                    RibbonTab::View => self.render_view_tab(ui, &mut actions),
                    RibbonTab::Developer => self.render_developer_tab(ui, &mut actions),
                    RibbonTab::Help => self.render_help_tab(ui, &mut actions),
                }
            });

            ui.separator();
        });

        actions
    }

    fn render_tab_headers(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(RichText::new("📊 Rust EXEL").strong().color(Color32::from_rgb(16, 124, 65)).size(13.0));
            ui.separator();
            ui.spacing_mut().item_spacing.x = 2.0;

            let tabs = [
                (RibbonTab::File, "File"),
                (RibbonTab::Home, "Home"),
                (RibbonTab::Insert, "Insert"),
                (RibbonTab::PageLayout, "Page Layout"),
                (RibbonTab::Formulas, "Formulas"),
                (RibbonTab::Data, "Data"),
                (RibbonTab::Review, "Review"),
                (RibbonTab::View, "View"),
                (RibbonTab::Developer, "Developer"),
                (RibbonTab::Help, "Help"),
            ];

            for (tab, title) in tabs {
                let is_active = self.state.active_ribbon_tab == tab;
                let is_file = tab == RibbonTab::File;

                let text = if is_file {
                    RichText::new(title)
                        .color(Color32::WHITE)
                        .strong()
                } else if is_active {
                    RichText::new(title)
                        .color(SpreadsheetTheme::EXCEL_GREEN)
                        .strong()
                } else {
                    RichText::new(title)
                        .color(SpreadsheetTheme::HEADER_TEXT)
                };

                let button = if is_file {
                    egui::Button::new(text).fill(SpreadsheetTheme::EXCEL_GREEN)
                } else if is_active {
                    egui::Button::new(text).fill(SpreadsheetTheme::RIBBON_TAB_ACTIVE_BG)
                } else {
                    egui::Button::new(text).frame(false)
                };

                if ui.add(button).clicked() {
                    self.state.active_ribbon_tab = tab;
                }
            }
        });
    }

    fn render_home_tab(&mut self, ui: &mut Ui, actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            // Group 1: Clipboard
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("📋 Paste").on_hover_text("Paste (Ctrl+V)").clicked() {
                        actions.push(SpreadsheetAction::Paste);
                    }
                    ui.vertical(|ui| {
                        if ui.button("✂ Cut").on_hover_text("Cut (Ctrl+X)").clicked() {
                            actions.push(SpreadsheetAction::Cut);
                        }
                        if ui.button("📄 Copy").on_hover_text("Copy (Ctrl+C)").clicked() {
                            actions.push(SpreadsheetAction::Copy);
                        }
                    });
                });
                ui.label(RichText::new("Clipboard").size(10.0).weak());
            });

            ui.separator();

            // Group 2: Font
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    // Font Family Dropdown
                    ComboBox::from_id_salt("font_family")
                        .width(90.0)
                        .selected_text(&self.state.pending_cell_style.font.name)
                        .show_ui(ui, |ui| {
                            for font_name in ["Calibri", "Segoe UI", "Arial", "Times New Roman", "Consolas"] {
                                if ui.selectable_label(self.state.pending_cell_style.font.name == font_name, font_name).clicked() {
                                    self.state.pending_cell_style.font.name = font_name.to_string();
                                    actions.push(SpreadsheetAction::ApplyStyle {
                                        range: self.state.selection,
                                        style: self.state.pending_cell_style.clone(),
                                    });
                                }
                            }
                        });

                    // Font Size Dropdown
                    let current_size = self.state.pending_cell_style.font.size as u32;
                    ComboBox::from_id_salt("font_size")
                        .width(45.0)
                        .selected_text(format!("{}", current_size))
                        .show_ui(ui, |ui| {
                            for size in [8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 36, 72] {
                                if ui.selectable_label(current_size == size, format!("{}", size)).clicked() {
                                    self.state.pending_cell_style.font.size = size as f32;
                                    actions.push(SpreadsheetAction::ApplyStyle {
                                        range: self.state.selection,
                                        style: self.state.pending_cell_style.clone(),
                                    });
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    // Bold Toggle
                    let bold = self.state.pending_cell_style.font.bold;
                    if ui.selectable_label(bold, RichText::new("B").strong()).clicked() {
                        self.state.pending_cell_style.font.bold = !bold;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }

                    // Italic Toggle
                    let italic = self.state.pending_cell_style.font.italic;
                    if ui.selectable_label(italic, RichText::new("I").italics()).clicked() {
                        self.state.pending_cell_style.font.italic = !italic;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }

                    // Underline Toggle
                    let underline = self.state.pending_cell_style.font.underline;
                    if ui.selectable_label(underline, RichText::new("U").underline()).clicked() {
                        self.state.pending_cell_style.font.underline = !underline;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }

                    // Fill Color Quick Button (Yellow / None)
                    if ui.button("🎨 Fill").on_hover_text("Yellow Fill").clicked() {
                        self.state.pending_cell_style.fill = Fill::solid(FmtColor::rgb(255, 255, 150));
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }

                    // Text Color Button (Red / Black)
                    if ui.button("A Text").on_hover_text("Red Text Color").clicked() {
                        self.state.pending_cell_style.font.color = FmtColor::rgb(220, 38, 38);
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                });

                ui.label(RichText::new("Font").size(10.0).weak());
            });

            ui.separator();

            // Group 3: Alignment
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("⫷ Left").clicked() {
                        self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::Left;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                    if ui.button("⫸ Center").clicked() {
                        self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::Center;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                    if ui.button("⫸ Right").clicked() {
                        self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::Right;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                });

                ui.horizontal(|ui| {
                    let wrap = self.state.pending_cell_style.alignment.wrap_text;
                    if ui.selectable_label(wrap, "Wrap Text").clicked() {
                        self.state.pending_cell_style.alignment.wrap_text = !wrap;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }

                    if ui.button("Merge & Center").clicked() {
                        // Merge active range
                        self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::Center;
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                });

                ui.label(RichText::new("Alignment").size(10.0).weak());
            });

            ui.separator();

            // Group 4: Number Format
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ComboBox::from_id_salt("number_format_ribbon")
                        .width(110.0)
                        .selected_text(self.get_current_number_format_name())
                        .show_ui(ui, |ui| {
                            let formats = [
                                ("General", NumberFormat::General),
                                ("Number", NumberFormat::number(2, false)),
                                ("Currency", NumberFormat::currency("$", 2)),
                                ("Percentage", NumberFormat::percentage(2)),
                                ("Date", NumberFormat::Date(exel_formatting::DateFormat::default())),
                                ("Time", NumberFormat::Time(exel_formatting::TimeFormat::default())),
                                ("Text", NumberFormat::Text),
                            ];

                            for (label, fmt) in formats {
                                if ui.selectable_label(false, label).clicked() {
                                    self.state.pending_cell_style.number_format = fmt;
                                    actions.push(SpreadsheetAction::ApplyStyle {
                                        range: self.state.selection,
                                        style: self.state.pending_cell_style.clone(),
                                    });
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    if ui.button("$").on_hover_text("Currency Format").clicked() {
                        self.state.pending_cell_style.number_format = NumberFormat::currency("$", 2);
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                    if ui.button("%").on_hover_text("Percent Format").clicked() {
                        self.state.pending_cell_style.number_format = NumberFormat::percentage(2);
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                    if ui.button(".00→.0").on_hover_text("Decrease Decimals").clicked() {
                        // Decrease decimal places
                        self.state.pending_cell_style.number_format = NumberFormat::number(1, false);
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                    if ui.button(".0→.00").on_hover_text("Increase Decimals").clicked() {
                        // Increase decimal places
                        self.state.pending_cell_style.number_format = NumberFormat::number(3, false);
                        actions.push(SpreadsheetAction::ApplyStyle {
                            range: self.state.selection,
                            style: self.state.pending_cell_style.clone(),
                        });
                    }
                });

                ui.label(RichText::new("Number").size(10.0).weak());
            });

            ui.separator();

            // Group 5: Cells & Format Cells
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("➕ Insert").clicked() {
                        // Insert action
                    }
                    if ui.button("➖ Delete").clicked() {
                        actions.push(SpreadsheetAction::ClearCells {
                            range: self.state.selection,
                        });
                    }
                });

                if ui.button("⚙ Format Cells...").on_hover_text("Format Cells Dialog (Ctrl+1)").clicked() {
                    self.state.show_format_cells_dialog = true;
                }

                ui.label(RichText::new("Cells").size(10.0).weak());
            });

            ui.separator();

            // Group 6: Editing (AutoSum, Clear)
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ComboBox::from_id_salt("autosum_dropdown")
                        .width(90.0)
                        .selected_text("Σ AutoSum")
                        .show_ui(ui, |ui| {
                            let funcs = [
                                (AutoSumFunction::Sum, "Sum"),
                                (AutoSumFunction::Average, "Average"),
                                (AutoSumFunction::Count, "Count"),
                                (AutoSumFunction::Max, "Max"),
                                (AutoSumFunction::Min, "Min"),
                            ];
                            for (func, label) in funcs {
                                if ui.button(label).clicked() {
                                    actions.push(SpreadsheetAction::AutoSum {
                                        range: self.state.selection,
                                        func,
                                    });
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    if ui.button("🧹 Clear All").clicked() {
                        actions.push(SpreadsheetAction::ClearCells {
                            range: self.state.selection,
                        });
                    }
                    if ui.button("🔄 Recalc").on_hover_text("Recalculate Formulas (F9)").clicked() {
                        actions.push(SpreadsheetAction::Recalculate);
                    }
                });

                ui.label(RichText::new("Editing").size(10.0).weak());
            });
        }).response
    }

    fn render_file_tab(&mut self, ui: &mut Ui, actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            if ui.button("📄 New Workbook").clicked() {
                actions.push(SpreadsheetAction::NewFile);
            }
            if ui.button("📂 Open...").clicked() {
                actions.push(SpreadsheetAction::OpenFile);
            }
            if ui.button("💾 Save").clicked() {
                actions.push(SpreadsheetAction::SaveFile);
            }
            if ui.button("💾 Save As...").clicked() {
                actions.push(SpreadsheetAction::SaveAsFile);
            }
            ui.separator();
            if ui.button("📊 Export XLSX").clicked() {
                actions.push(SpreadsheetAction::ExportXlsx);
            }
            if ui.button("📄 Export CSV").clicked() {
                actions.push(SpreadsheetAction::ExportCsv);
            }
        }).response
    }

    fn render_insert_tab(&mut self, ui: &mut Ui, _actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            let _ = ui.button("📊 Column Chart");
            let _ = ui.button("📈 Line Chart");
            let _ = ui.button("🥧 Pie Chart");
            let _ = ui.button("📉 Bar Chart");
            ui.separator();
            let _ = ui.button("📋 Pivot Table");
            let _ = ui.button("📑 Data Table");
            ui.separator();
            let _ = ui.button("🔗 Hyperlink");
            let _ = ui.button("🔤 Text Box");
        }).response
    }

    fn render_page_layout_tab(&mut self, ui: &mut Ui, _actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            let _ = ui.button("📐 Margins");
            let _ = ui.button("📄 Orientation");
            let _ = ui.button("📏 Page Size");
            let _ = ui.button("🖨 Print Area");
            ui.separator();
            ui.checkbox(&mut self.state.show_gridlines, "View Gridlines");
            ui.checkbox(&mut self.state.show_headings, "View Headings");
        }).response
    }

    fn render_formulas_tab(&mut self, ui: &mut Ui, actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            if ui.button("fx Insert Function").clicked() {
                if !self.state.formula_buffer.starts_with('=') {
                    self.state.formula_buffer = format!("={}", self.state.formula_buffer);
                }
            }
            ui.separator();
            if ui.button("Σ AutoSum").clicked() {
                actions.push(SpreadsheetAction::AutoSum {
                    range: self.state.selection,
                    func: AutoSumFunction::Sum,
                });
            }
            let _ = ui.button("💲 Financial");
            let _ = ui.button("⚖ Logical");
            let _ = ui.button("🔤 Text");
            let _ = ui.button("📅 Date & Time");
            let _ = ui.button("📐 Math & Trig");
            ui.separator();
            if ui.button("⚡ Calculate Now (F9)").clicked() {
                actions.push(SpreadsheetAction::Recalculate);
            }
        }).response
    }

    fn render_data_tab(&mut self, ui: &mut Ui, _actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            let _ = ui.button("📶 Get Data");
            let _ = ui.button("🔄 Refresh All");
            ui.separator();
            let _ = ui.button("AZ↓ Sort A-Z");
            let _ = ui.button("ZA↓ Sort Z-A");
            let _ = ui.button("🔍 Filter");
            ui.separator();
            let _ = ui.button("🛡 Data Validation");
            let _ = ui.button("📊 Consolidate");
        }).response
    }

    fn render_review_tab(&mut self, ui: &mut Ui, _actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            let _ = ui.button("✓ Spelling");
            let _ = ui.button("💬 New Comment");
            ui.separator();
            let _ = ui.button("🔒 Protect Sheet");
            let _ = ui.button("🛡 Protect Workbook");
        }).response
    }

    fn render_view_tab(&mut self, ui: &mut Ui, _actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            if ui.selectable_label(self.state.view_mode == ViewMode::Normal, "Normal").clicked() {
                self.state.view_mode = ViewMode::Normal;
            }
            if ui.selectable_label(self.state.view_mode == ViewMode::PageBreakPreview, "Page Break").clicked() {
                self.state.view_mode = ViewMode::PageBreakPreview;
            }
            if ui.selectable_label(self.state.view_mode == ViewMode::PageLayout, "Page Layout").clicked() {
                self.state.view_mode = ViewMode::PageLayout;
            }
            ui.separator();
            ui.checkbox(&mut self.state.show_gridlines, "Gridlines");
            ui.checkbox(&mut self.state.show_headings, "Headings");
            ui.checkbox(&mut self.state.show_formula_bar, "Formula Bar");
            ui.separator();
            if ui.button("🔍 100% Zoom").clicked() {
                self.state.zoom = 1.0;
            }
            if ui.button("❄ Freeze Panes").clicked() {
                // Freeze pane action
            }
        }).response
    }

    fn render_developer_tab(&mut self, ui: &mut Ui, _actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            let _ = ui.button("💻 Visual Basic / Scripts");
            let _ = ui.button("▶ Macros");
            let _ = ui.button("⏺ Record Macro");
            let _ = ui.button("🧩 Add-ins");
        }).response
    }

    fn render_help_tab(&mut self, ui: &mut Ui, _actions: &mut Vec<SpreadsheetAction>) -> Response {
        ui.horizontal(|ui| {
            let _ = ui.button("❓ Help");
            let _ = ui.button("🌐 Community");
            let _ = ui.button("✉ Feedback");
            ui.label(RichText::new("Rust EXCEL v0.1.0").weak());
        }).response
    }

    fn get_current_number_format_name(&self) -> &'static str {
        match &self.state.pending_cell_style.number_format {
            NumberFormat::General => "General",
            NumberFormat::Number { .. } => "Number",
            NumberFormat::Currency { .. } => "Currency",
            NumberFormat::Percentage { .. } => "Percentage",
            NumberFormat::Date(_) => "Date",
            NumberFormat::Time(_) => "Time",
            NumberFormat::Text => "Text",
            _ => "Custom",
        }
    }
}
