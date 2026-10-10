use crate::state::{CellFormatTab, SpreadsheetAction, SpreadsheetUiState};
use crate::theme::SpreadsheetTheme;
use egui::{
    vec2, Align2, Color32, ComboBox, Grid, Layout, RichText, Stroke, StrokeKind, Ui,
    Window,
};
use exel_formatting::{
    Border, BorderSide, BorderStyle, Color as FmtColor, Fill,
    HorizontalAlignment, NumberFormat, VerticalAlignment,
};

/// Format Cells Modal Dialog.
pub struct FormatCellsDialog<'a> {
    state: &'a mut SpreadsheetUiState,
}

impl<'a> FormatCellsDialog<'a> {
    pub fn new(state: &'a mut SpreadsheetUiState) -> Self {
        Self { state }
    }

    pub fn show(mut self, ctx: &egui::Context) -> Option<SpreadsheetAction> {
        let mut result_action = None;

        if !self.state.show_format_cells_dialog {
            return None;
        }

        let mut is_open = self.state.show_format_cells_dialog;

        Window::new("Format Cells")
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .default_size(vec2(520.0, 420.0))
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // 1. Tab Selector Bar
                    ui.horizontal(|ui| {
                        let tabs = [
                            (CellFormatTab::Number, "Number"),
                            (CellFormatTab::Alignment, "Alignment"),
                            (CellFormatTab::Font, "Font"),
                            (CellFormatTab::Border, "Border"),
                            (CellFormatTab::Fill, "Fill"),
                        ];

                        for (tab, label) in tabs {
                            let is_active = self.state.format_cells_active_tab == tab;
                            if ui
                                .selectable_label(
                                    is_active,
                                    RichText::new(label).strong(),
                                )
                                .clicked()
                            {
                                self.state.format_cells_active_tab = tab;
                            }
                        }
                    });

                    ui.separator();

                    // 2. Active Tab Content
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), 300.0),
                        Layout::top_down(egui::Align::LEFT),
                        |ui| match self.state.format_cells_active_tab {
                            CellFormatTab::Number => self.render_number_tab(ui),
                            CellFormatTab::Alignment => self.render_alignment_tab(ui),
                            CellFormatTab::Font => self.render_font_tab(ui),
                            CellFormatTab::Border => self.render_border_tab(ui),
                            CellFormatTab::Fill => self.render_fill_tab(ui),
                        },
                    );

                    ui.separator();

                    // 3. OK and Cancel Action Buttons
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            self.state.show_format_cells_dialog = false;
                        }

                        if ui
                            .button(RichText::new("OK").strong())
                            .clicked()
                        {
                            result_action = Some(SpreadsheetAction::ApplyStyle {
                                range: self.state.selection,
                                style: self.state.pending_cell_style.clone(),
                            });
                            self.state.show_format_cells_dialog = false;
                        }
                    });
                });
            });

        self.state.show_format_cells_dialog = is_open;
        result_action
    }

    fn render_number_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Category List Box on left
            ui.vertical(|ui| {
                ui.label(RichText::new("Category:").strong());
                let categories = [
                    "General",
                    "Number",
                    "Currency",
                    "Accounting",
                    "Date",
                    "Time",
                    "Percentage",
                    "Text",
                ];

                for cat in categories {
                    if ui.button(cat).clicked() {
                        match cat {
                            "General" => self.state.pending_cell_style.number_format = NumberFormat::General,
                            "Number" => self.state.pending_cell_style.number_format = NumberFormat::number(2, false),
                            "Currency" => self.state.pending_cell_style.number_format = NumberFormat::currency("$", 2),
                            "Accounting" => self.state.pending_cell_style.number_format = NumberFormat::currency("$", 2),
                            "Date" => self.state.pending_cell_style.number_format = NumberFormat::Date(exel_formatting::DateFormat::default()),
                            "Time" => self.state.pending_cell_style.number_format = NumberFormat::Time(exel_formatting::TimeFormat::default()),
                            "Percentage" => self.state.pending_cell_style.number_format = NumberFormat::percentage(2),
                            "Text" => self.state.pending_cell_style.number_format = NumberFormat::Text,
                            _ => {}
                        }
                    }
                }
            });

            ui.separator();

            // Format Options & Live Preview on right
            ui.vertical(|ui| {
                ui.label(RichText::new("Sample Preview:").strong());
                let sample_val = 1234.5678;
                let preview = self
                    .state
                    .pending_cell_style
                    .number_format
                    .format_number(sample_val);

                ui.group(|ui| {
                    ui.label(RichText::new(preview).heading().color(SpreadsheetTheme::EXCEL_GREEN));
                });

                ui.add_space(10.0);
                ui.label("Decimal places: 2");
                ui.label("Currency Symbol: $");
            });
        });
    }

    fn render_alignment_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(RichText::new("Text Alignment").strong());
            ui.add_space(8.0);

            Grid::new("alignment_grid").num_columns(2).spacing([20.0, 10.0]).show(ui, |ui| {
                ui.label("Horizontal:");
                ComboBox::from_id_salt("horiz_align")
                    .selected_text(match self.state.pending_cell_style.alignment.horizontal {
                        HorizontalAlignment::General => "General",
                        HorizontalAlignment::Left => "Left (Indent)",
                        HorizontalAlignment::Center => "Center",
                        HorizontalAlignment::Right => "Right",
                        HorizontalAlignment::Fill => "Fill",
                        HorizontalAlignment::Justify => "Justify",
                        HorizontalAlignment::CenterContinuous => "Center Across Selection",
                        HorizontalAlignment::Distributed => "Distributed",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(false, "General").clicked() {
                            self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::General;
                        }
                        if ui.selectable_label(false, "Left").clicked() {
                            self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::Left;
                        }
                        if ui.selectable_label(false, "Center").clicked() {
                            self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::Center;
                        }
                        if ui.selectable_label(false, "Right").clicked() {
                            self.state.pending_cell_style.alignment.horizontal = HorizontalAlignment::Right;
                        }
                    });
                ui.end_row();

                ui.label("Vertical:");
                ComboBox::from_id_salt("vert_align")
                    .selected_text(match self.state.pending_cell_style.alignment.vertical {
                        VerticalAlignment::Top => "Top",
                        VerticalAlignment::Center => "Center",
                        VerticalAlignment::Bottom => "Bottom",
                        VerticalAlignment::Justify => "Justify",
                        VerticalAlignment::Distributed => "Distributed",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(false, "Top").clicked() {
                            self.state.pending_cell_style.alignment.vertical = VerticalAlignment::Top;
                        }
                        if ui.selectable_label(false, "Center").clicked() {
                            self.state.pending_cell_style.alignment.vertical = VerticalAlignment::Center;
                        }
                        if ui.selectable_label(false, "Bottom").clicked() {
                            self.state.pending_cell_style.alignment.vertical = VerticalAlignment::Bottom;
                        }
                    });
                ui.end_row();
            });

            ui.add_space(15.0);
            ui.label(RichText::new("Text Control").strong());
            ui.checkbox(
                &mut self.state.pending_cell_style.alignment.wrap_text,
                "Wrap text",
            );
        });
    }

    fn render_font_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                // Font Family
                ui.vertical(|ui| {
                    ui.label(RichText::new("Font:").strong());
                    for font_name in ["Calibri", "Segoe UI", "Arial", "Times New Roman", "Consolas"] {
                        if ui
                            .selectable_label(
                                self.state.pending_cell_style.font.name == font_name,
                                font_name,
                            )
                            .clicked()
                        {
                            self.state.pending_cell_style.font.name = font_name.to_string();
                        }
                    }
                });

                ui.separator();

                // Font Style (Bold / Italic)
                ui.vertical(|ui| {
                    ui.label(RichText::new("Font style:").strong());
                    let is_regular = !self.state.pending_cell_style.font.bold && !self.state.pending_cell_style.font.italic;
                    let is_italic = !self.state.pending_cell_style.font.bold && self.state.pending_cell_style.font.italic;
                    let is_bold = self.state.pending_cell_style.font.bold && !self.state.pending_cell_style.font.italic;
                    let is_both = self.state.pending_cell_style.font.bold && self.state.pending_cell_style.font.italic;

                    if ui.selectable_label(is_regular, "Regular").clicked() {
                        self.state.pending_cell_style.font.bold = false;
                        self.state.pending_cell_style.font.italic = false;
                    }
                    if ui.selectable_label(is_italic, "Italic").clicked() {
                        self.state.pending_cell_style.font.bold = false;
                        self.state.pending_cell_style.font.italic = true;
                    }
                    if ui.selectable_label(is_bold, "Bold").clicked() {
                        self.state.pending_cell_style.font.bold = true;
                        self.state.pending_cell_style.font.italic = false;
                    }
                    if ui.selectable_label(is_both, "Bold Italic").clicked() {
                        self.state.pending_cell_style.font.bold = true;
                        self.state.pending_cell_style.font.italic = true;
                    }
                });

                ui.separator();

                // Font Size
                ui.vertical(|ui| {
                    ui.label(RichText::new("Size:").strong());
                    let current_size = self.state.pending_cell_style.font.size as i32;
                    for size in [9, 10, 11, 12, 14, 16, 18, 20, 24, 28] {
                        if ui.selectable_label(current_size == size, format!("{}", size)).clicked() {
                            self.state.pending_cell_style.font.size = size as f32;
                        }
                    }
                });
            });

            ui.add_space(10.0);
            ui.checkbox(
                &mut self.state.pending_cell_style.font.underline,
                "Underline",
            );
        });
    }

    fn render_border_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(RichText::new("Presets:").strong());
            ui.horizontal(|ui| {
                if ui.button("None").clicked() {
                    self.state.pending_cell_style.border = Border::none();
                }
                if ui.button("Outline").clicked() {
                    self.state.pending_cell_style.border = Border::all(BorderStyle::Thin, FmtColor::BLACK);
                }
                if ui.button("Bottom Double").clicked() {
                    let mut b = Border::none();
                    b.bottom = BorderSide::new(BorderStyle::Double, FmtColor::BLACK);
                    self.state.pending_cell_style.border = b;
                }
            });

            ui.add_space(15.0);
            ui.label(RichText::new("Border Sides:").strong());
            ui.horizontal(|ui| {
                let has_top = self.state.pending_cell_style.border.top.style != BorderStyle::None;
                if ui.selectable_label(has_top, "Top").clicked() {
                    if has_top {
                        self.state.pending_cell_style.border.top = BorderSide::none();
                    } else {
                        self.state.pending_cell_style.border.top = BorderSide::new(BorderStyle::Thin, FmtColor::BLACK);
                    }
                }

                let has_bottom = self.state.pending_cell_style.border.bottom.style != BorderStyle::None;
                if ui.selectable_label(has_bottom, "Bottom").clicked() {
                    if has_bottom {
                        self.state.pending_cell_style.border.bottom = BorderSide::none();
                    } else {
                        self.state.pending_cell_style.border.bottom = BorderSide::new(BorderStyle::Thin, FmtColor::BLACK);
                    }
                }

                let has_left = self.state.pending_cell_style.border.left.style != BorderStyle::None;
                if ui.selectable_label(has_left, "Left").clicked() {
                    if has_left {
                        self.state.pending_cell_style.border.left = BorderSide::none();
                    } else {
                        self.state.pending_cell_style.border.left = BorderSide::new(BorderStyle::Thin, FmtColor::BLACK);
                    }
                }

                let has_right = self.state.pending_cell_style.border.right.style != BorderStyle::None;
                if ui.selectable_label(has_right, "Right").clicked() {
                    if has_right {
                        self.state.pending_cell_style.border.right = BorderSide::none();
                    } else {
                        self.state.pending_cell_style.border.right = BorderSide::new(BorderStyle::Thin, FmtColor::BLACK);
                    }
                }
            });
        });
    }

    fn render_fill_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(RichText::new("Background Color:").strong());
            ui.add_space(10.0);

            // Palette of standard spreadsheet fill colors
            let palette = [
                ("No Color", None),
                ("Light Yellow", Some(FmtColor::rgb(255, 255, 180))),
                ("Light Green", Some(FmtColor::rgb(198, 239, 206))),
                ("Light Red", Some(FmtColor::rgb(255, 199, 206))),
                ("Light Blue", Some(FmtColor::rgb(217, 225, 242))),
                ("Light Gray", Some(FmtColor::rgb(242, 242, 242))),
                ("Accent Blue", Some(FmtColor::rgb(180, 210, 255))),
            ];

            for (name, color_opt) in palette {
                if ui.button(name).clicked() {
                    if let Some(col) = color_opt {
                        self.state.pending_cell_style.fill = Fill::solid(col);
                    } else {
                        self.state.pending_cell_style.fill = Fill::none();
                    }
                }
            }

            ui.add_space(15.0);
            ui.label(RichText::new("Sample Preview").strong());
            let preview_rect = ui.available_rect_before_wrap();
            let sample_rect = egui::Rect::from_min_size(preview_rect.min, vec2(150.0, 30.0));

            let bg = self.state.pending_cell_style.fill.fg_color.or(self.state.pending_cell_style.fill.bg_color).map(|c| {
                Color32::from_rgb(c.r, c.g, c.b)
            }).unwrap_or(Color32::WHITE);

            ui.painter().rect_filled(sample_rect, 0.0, bg);
            ui.painter().rect_stroke(sample_rect, 0.0, Stroke::new(1.0, Color32::GRAY), StrokeKind::Inside);
        });
    }
}
