use crate::state::{AutoSumFunction, SpreadsheetAction, SpreadsheetUiState};
use crate::theme::SpreadsheetTheme;
use egui::{
    vec2, Align2, Color32, CornerRadius, CursorIcon, Event, Id, Key, Painter, Pos2, Rect,
    Response, Sense, Stroke, StrokeKind, Ui, UiBuilder,
};
use exel_cell::{CellCoord, CellRange, CellValue};
use exel_core::{col_to_name, MAX_COLS, MAX_ROWS};
use exel_workbook::Worksheet;

/// Virtualized Spreadsheet Grid Widget in egui using Painter.
pub struct SpreadsheetGrid<'a> {
    worksheet: &'a Worksheet,
    state: &'a mut SpreadsheetUiState,
}

impl<'a> SpreadsheetGrid<'a> {
    pub fn new(worksheet: &'a Worksheet, state: &'a mut SpreadsheetUiState) -> Self {
        Self { worksheet, state }
    }

    /// Renders the complete virtualized spreadsheet grid and handles user interactions.
    pub fn show(mut self, ui: &mut Ui) -> (Response, Vec<SpreadsheetAction>) {
        let mut actions = Vec::new();
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        let zoom = self.state.zoom;
        let row_header_w = SpreadsheetTheme::ROW_HEADER_WIDTH * zoom;
        let col_header_h = SpreadsheetTheme::COL_HEADER_HEIGHT * zoom;

        // Viewport rect for the actual cell contents (excluding sticky headers)
        let grid_content_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + row_header_w, rect.min.y + col_header_h),
            rect.max,
        );

        // 1. Handle keyboard navigation & shortcuts when grid is focused
        if response.has_focus() || !self.state.is_editing {
            self.handle_grid_keys(ui, &mut actions);
        }

        // 2. Handle mouse wheel scrolling
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        if response.hovered() && (scroll_delta.x != 0.0 || scroll_delta.y != 0.0) {
            let shift = ui.input(|i| i.modifiers.shift);
            if shift {
                self.state.scroll_x = (self.state.scroll_x - scroll_delta.y).max(0.0);
            } else {
                self.state.scroll_x = (self.state.scroll_x - scroll_delta.x).max(0.0);
                self.state.scroll_y = (self.state.scroll_y - scroll_delta.y).max(0.0);
            }
        }

        // 3. Compute virtualized visible columns
        let (visible_cols, col_positions) = self.compute_visible_columns(
            grid_content_rect.width(),
            self.state.scroll_x,
            zoom,
        );

        // 4. Compute virtualized visible rows
        let (visible_rows, row_positions) = self.compute_visible_rows(
            grid_content_rect.height(),
            self.state.scroll_y,
            zoom,
        );

        // 5. Handle mouse interactions (clicks, drags, header resizing)
        self.handle_mouse_interactions(
            ui,
            &response,
            rect,
            grid_content_rect,
            row_header_w,
            col_header_h,
            &visible_cols,
            &col_positions,
            &visible_rows,
            &row_positions,
            zoom,
            &mut actions,
        );

        // 6. Draw grid background & cell contents with clipping
        painter.rect_filled(grid_content_rect, 0.0, SpreadsheetTheme::CELL_BG);
        let cell_painter = painter.with_clip_rect(grid_content_rect);

        // Draw cells and cell fills
        self.draw_cells(
            &cell_painter,
            grid_content_rect,
            &visible_cols,
            &col_positions,
            &visible_rows,
            &row_positions,
            zoom,
        );

        // Draw gridlines
        if self.state.show_gridlines {
            self.draw_gridlines(
                &cell_painter,
                grid_content_rect,
                &visible_cols,
                &col_positions,
                &visible_rows,
                &row_positions,
            );
        }

        // Draw selection marquee & active cell indicator
        self.draw_selection_and_active_cell(
            &cell_painter,
            grid_content_rect,
            &visible_cols,
            &col_positions,
            &visible_rows,
            &row_positions,
            zoom,
        );

        // Draw Autofill drag preview if active
        if self.state.is_dragging_fill_handle {
            if let Some(target) = self.state.fill_handle_target_range {
                self.draw_autofill_preview(
                    &cell_painter,
                    grid_content_rect,
                    &visible_cols,
                    &col_positions,
                    &visible_rows,
                    &row_positions,
                    target,
                    zoom,
                );
            }
        }

        // 7. Draw sticky column headers
        if self.state.show_headings {
            let col_header_rect = Rect::from_min_max(
                Pos2::new(grid_content_rect.min.x, rect.min.y),
                Pos2::new(rect.max.x, rect.min.y + col_header_h),
            );
            let col_painter = painter.with_clip_rect(col_header_rect);
            self.draw_col_headers(
                &col_painter,
                col_header_rect,
                &visible_cols,
                &col_positions,
                zoom,
            );

            // 8. Draw sticky row headers
            let row_header_rect = Rect::from_min_max(
                Pos2::new(rect.min.x, grid_content_rect.min.y),
                Pos2::new(rect.min.x + row_header_w, rect.max.y),
            );
            let row_painter = painter.with_clip_rect(row_header_rect);
            self.draw_row_headers(
                &row_painter,
                row_header_rect,
                &visible_rows,
                &row_positions,
                zoom,
            );

            // 9. Draw top-left corner cell (Select All)
            let corner_rect = Rect::from_min_max(
                rect.min,
                Pos2::new(rect.min.x + row_header_w, rect.min.y + col_header_h),
            );
            self.draw_corner_cell(&painter, corner_rect);
        }

        // 10. Draw Inline Cell Editor if active
        if self.state.is_editing {
            self.render_inline_editor(
                ui,
                grid_content_rect,
                &visible_cols,
                &col_positions,
                &visible_rows,
                &row_positions,
                zoom,
                &mut actions,
            );
        }

        // 11. Context Menu (right click)
        self.render_context_menu(&response, &mut actions);

        (response, actions)
    }

    /// Computes the list of columns currently visible in the viewport and their horizontal pixel ranges.
    fn compute_visible_columns(
        &self,
        viewport_width: f32,
        scroll_x: f32,
        zoom: f32,
    ) -> (Vec<u32>, Vec<(f32, f32)>) {
        let mut visible_cols = Vec::new();
        let mut positions = Vec::new();

        let mut current_x = 0.0;
        let max_x = scroll_x + viewport_width;

        for col in 1..=MAX_COLS {
            let width = self.worksheet.col_width(col) * zoom;
            let col_end = current_x + width;

            if col_end >= scroll_x && current_x <= max_x {
                visible_cols.push(col);
                positions.push((current_x - scroll_x, col_end - scroll_x));
            }

            current_x = col_end;
            if current_x > max_x {
                break;
            }
        }

        (visible_cols, positions)
    }

    /// Computes the list of rows currently visible in the viewport and their vertical pixel ranges.
    fn compute_visible_rows(
        &self,
        viewport_height: f32,
        scroll_y: f32,
        zoom: f32,
    ) -> (Vec<u32>, Vec<(f32, f32)>) {
        let mut visible_rows = Vec::new();
        let mut positions = Vec::new();

        let mut current_y = 0.0;
        let max_y = scroll_y + viewport_height;

        for row in 1..=MAX_ROWS {
            let height = self.worksheet.row_height(row) * zoom;
            let row_end = current_y + height;

            if row_end >= scroll_y && current_y <= max_y {
                visible_rows.push(row);
                positions.push((current_y - scroll_y, row_end - scroll_y));
            }

            current_y = row_end;
            if current_y > max_y {
                break;
            }
        }

        (visible_rows, positions)
    }

    fn draw_cells(
        &self,
        painter: &Painter,
        grid_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        zoom: f32,
    ) {
        let font_id = egui::FontId::proportional(11.0 * zoom);

        for (col_idx, &col) in cols.iter().enumerate() {
            let (col_x1, col_x2) = col_pos[col_idx];
            let cell_left = grid_rect.min.x + col_x1;
            let cell_right = grid_rect.min.x + col_x2;

            for (row_idx, &row) in rows.iter().enumerate() {
                let (row_y1, row_y2) = row_pos[row_idx];
                let cell_top = grid_rect.min.y + row_y1;
                let cell_bottom = grid_rect.min.y + row_y2;

                let cell_rect = Rect::from_min_max(
                    Pos2::new(cell_left, cell_top),
                    Pos2::new(cell_right, cell_bottom),
                );

                let coord = CellCoord::must_new(row, col);
                if let Some(cell_state) = self.worksheet.get_cell(coord) {
                    if cell_state.style_id != 0 {
                        if let Some(style) = self.state.style_table.get(cell_state.style_id) {
                            if let Some(bg_color) = style.fill.fg_color.or(style.fill.bg_color) {
                                let egui_color = Color32::from_rgba_premultiplied(
                                    bg_color.r, bg_color.g, bg_color.b, bg_color.a,
                                );
                                painter.rect_filled(cell_rect, 0.0, egui_color);
                            }
                        }
                    }

                    let display_text = if cell_state.style_id != 0 {
                        if let Some(style) = self.state.style_table.get(cell_state.style_id) {
                            match &cell_state.value {
                                CellValue::Number(n) => style.number_format.format_number(*n),
                                other => other.to_display_string(),
                            }
                        } else {
                            cell_state.value.to_display_string()
                        }
                    } else {
                        cell_state.value.to_display_string()
                    };

                    if !display_text.is_empty() {
                        let text_color = SpreadsheetTheme::CELL_TEXT;

                        let (align, text_pos) = match &cell_state.value {
                            CellValue::Number(_) => (
                                Align2::RIGHT_CENTER,
                                Pos2::new(cell_rect.max.x - 4.0 * zoom, cell_rect.center().y),
                            ),
                            CellValue::Boolean(_) | CellValue::Error(_) => (
                                Align2::CENTER_CENTER,
                                Pos2::new(cell_rect.center().x, cell_rect.center().y),
                            ),
                            _ => (
                                Align2::LEFT_CENTER,
                                Pos2::new(cell_rect.min.x + 4.0 * zoom, cell_rect.center().y),
                            ),
                        };

                        painter.text(text_pos, align, display_text, font_id.clone(), text_color);
                    }
                }
            }
        }
    }

    fn draw_gridlines(
        &self,
        painter: &Painter,
        grid_rect: Rect,
        _cols: &[u32],
        col_pos: &[(f32, f32)],
        _rows: &[u32],
        row_pos: &[(f32, f32)],
    ) {
        let stroke = SpreadsheetTheme::grid_stroke();

        for &(_, x2) in col_pos {
            let x = grid_rect.min.x + x2;
            painter.line_segment(
                [Pos2::new(x, grid_rect.min.y), Pos2::new(x, grid_rect.max.y)],
                stroke,
            );
        }

        for &(_, y2) in row_pos {
            let y = grid_rect.min.y + y2;
            painter.line_segment(
                [Pos2::new(grid_rect.min.x, y), Pos2::new(grid_rect.max.x, y)],
                stroke,
            );
        }
    }

    fn draw_selection_and_active_cell(
        &self,
        painter: &Painter,
        grid_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        zoom: f32,
    ) {
        let sel = self.state.selection;

        if let Some(sel_rect) = self.get_range_screen_rect(grid_rect, cols, col_pos, rows, row_pos, sel) {
            if sel.cell_count() > 1 {
                painter.rect_filled(sel_rect, 0.0, SpreadsheetTheme::SELECTION_FILL);
                painter.rect_stroke(
                    sel_rect,
                    0.0,
                    SpreadsheetTheme::selection_border_stroke(),
                    StrokeKind::Inside,
                );
            }

            let active_coord = self.state.active_cell;
            if let Some(active_rect) = self.get_cell_screen_rect(
                grid_rect,
                cols,
                col_pos,
                rows,
                row_pos,
                active_coord.col,
                active_coord.row,
            ) {
                painter.rect_stroke(
                    active_rect,
                    0.0,
                    SpreadsheetTheme::active_cell_stroke(),
                    StrokeKind::Inside,
                );
            }

            let handle_size = SpreadsheetTheme::FILL_HANDLE_SIZE * zoom.clamp(0.8, 1.5);
            let handle_rect = Rect::from_center_size(
                sel_rect.max,
                vec2(handle_size, handle_size),
            );
            painter.rect_filled(
                handle_rect,
                CornerRadius::same(1),
                SpreadsheetTheme::EXCEL_GREEN,
            );
            painter.rect_stroke(
                handle_rect,
                CornerRadius::same(1),
                Stroke::new(1.0, Color32::WHITE),
                StrokeKind::Outside,
            );
        }
    }

    fn draw_autofill_preview(
        &self,
        painter: &Painter,
        grid_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        target: CellRange,
        _zoom: f32,
    ) {
        if let Some(target_rect) = self.get_range_screen_rect(grid_rect, cols, col_pos, rows, row_pos, target) {
            painter.rect_filled(target_rect, 0.0, SpreadsheetTheme::AUTOFILL_FILL);
            painter.rect_stroke(
                target_rect,
                0.0,
                SpreadsheetTheme::autofill_dashed_stroke(),
                StrokeKind::Inside,
            );
        }
    }

    fn draw_col_headers(
        &self,
        painter: &Painter,
        header_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        zoom: f32,
    ) {
        let font_id = egui::FontId::proportional(11.0 * zoom);
        let border_stroke = SpreadsheetTheme::header_border_stroke();

        painter.rect_filled(header_rect, 0.0, SpreadsheetTheme::HEADER_BG);
        painter.line_segment(
            [
                Pos2::new(header_rect.min.x, header_rect.max.y),
                Pos2::new(header_rect.max.x, header_rect.max.y),
            ],
            border_stroke,
        );

        let active_col = self.state.active_cell.col;
        let sel = self.state.selection;

        for (idx, &col) in cols.iter().enumerate() {
            let (x1, x2) = col_pos[idx];
            let left = header_rect.min.x + x1;
            let right = header_rect.min.x + x2;

            let col_rect = Rect::from_min_max(
                Pos2::new(left, header_rect.min.y),
                Pos2::new(right, header_rect.max.y),
            );

            let is_selected = col >= sel.start.col && col <= sel.end.col;
            let is_active = col == active_col;

            if is_selected {
                painter.rect_filled(col_rect, 0.0, SpreadsheetTheme::HEADER_BG_ACTIVE);
            }

            painter.line_segment(
                [Pos2::new(right, header_rect.min.y), Pos2::new(right, header_rect.max.y)],
                border_stroke,
            );

            let col_name = col_to_name(col);
            let text_color = if is_active {
                SpreadsheetTheme::HEADER_TEXT_ACTIVE
            } else {
                SpreadsheetTheme::HEADER_TEXT
            };

            painter.text(
                col_rect.center(),
                Align2::CENTER_CENTER,
                col_name,
                font_id.clone(),
                text_color,
            );
        }
    }

    fn draw_row_headers(
        &self,
        painter: &Painter,
        header_rect: Rect,
        rows: &[u32],
        row_pos: &[(f32, f32)],
        zoom: f32,
    ) {
        let font_id = egui::FontId::proportional(11.0 * zoom);
        let border_stroke = SpreadsheetTheme::header_border_stroke();

        painter.rect_filled(header_rect, 0.0, SpreadsheetTheme::HEADER_BG);
        painter.line_segment(
            [
                Pos2::new(header_rect.max.x, header_rect.min.y),
                Pos2::new(header_rect.max.x, header_rect.max.y),
            ],
            border_stroke,
        );

        let active_row = self.state.active_cell.row;
        let sel = self.state.selection;

        for (idx, &row) in rows.iter().enumerate() {
            let (y1, y2) = row_pos[idx];
            let top = header_rect.min.y + y1;
            let bottom = header_rect.min.y + y2;

            let row_rect = Rect::from_min_max(
                Pos2::new(header_rect.min.x, top),
                Pos2::new(header_rect.max.x, bottom),
            );

            let is_selected = row >= sel.start.row && row <= sel.end.row;
            let is_active = row == active_row;

            if is_selected {
                painter.rect_filled(row_rect, 0.0, SpreadsheetTheme::HEADER_BG_ACTIVE);
            }

            painter.line_segment(
                [Pos2::new(header_rect.min.x, bottom), Pos2::new(header_rect.max.x, bottom)],
                border_stroke,
            );

            let text_color = if is_active {
                SpreadsheetTheme::HEADER_TEXT_ACTIVE
            } else {
                SpreadsheetTheme::HEADER_TEXT
            };

            painter.text(
                row_rect.center(),
                Align2::CENTER_CENTER,
                row.to_string(),
                font_id.clone(),
                text_color,
            );
        }
    }

    fn draw_corner_cell(&self, painter: &Painter, corner_rect: Rect) {
        painter.rect_filled(corner_rect, 0.0, SpreadsheetTheme::HEADER_BG);
        painter.rect_stroke(
            corner_rect,
            0.0,
            SpreadsheetTheme::header_border_stroke(),
            StrokeKind::Inside,
        );

        let p1 = Pos2::new(corner_rect.max.x - 4.0, corner_rect.max.y - 4.0);
        let p2 = Pos2::new(corner_rect.max.x - 12.0, corner_rect.max.y - 4.0);
        let p3 = Pos2::new(corner_rect.max.x - 4.0, corner_rect.max.y - 12.0);

        painter.add(egui::Shape::convex_polygon(
            vec![p1, p2, p3],
            SpreadsheetTheme::HEADER_TEXT,
            Stroke::NONE,
        ));
    }

    fn handle_mouse_interactions(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        total_rect: Rect,
        grid_rect: Rect,
        row_header_w: f32,
        col_header_h: f32,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        zoom: f32,
        actions: &mut Vec<SpreadsheetAction>,
    ) {
        let hover_pos = ui.input(|i| i.pointer.hover_pos());

        if let Some(pos) = hover_pos {
            let col_header_rect = Rect::from_min_max(
                Pos2::new(grid_rect.min.x, total_rect.min.y),
                Pos2::new(total_rect.max.x, total_rect.min.y + col_header_h),
            );

            if col_header_rect.contains(pos) || self.state.col_resizing.is_some() {
                let mut near_col_div = None;
                for (idx, &col) in cols.iter().enumerate() {
                    let (_, x2) = col_pos[idx];
                    let screen_x = grid_rect.min.x + x2;
                    if (pos.x - screen_x).abs() <= SpreadsheetTheme::RESIZE_HANDLE_SLOP {
                        near_col_div = Some((col, screen_x));
                        break;
                    }
                }

                if let Some((col, _)) = near_col_div {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    if response.drag_started() {
                        let init_w = self.worksheet.col_width(col);
                        self.state.col_resizing = Some((col, init_w, pos.x));
                    }
                }

                if let Some((col, init_w, start_x)) = self.state.col_resizing {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    let delta = (pos.x - start_x) / zoom;
                    let new_w = (init_w + delta).max(20.0);
                    actions.push(SpreadsheetAction::ResizeColumn { col, width: new_w });

                    if response.drag_stopped() {
                        self.state.col_resizing = None;
                    }
                    return;
                }
            }
        }

        if let Some(pos) = hover_pos {
            let row_header_rect = Rect::from_min_max(
                Pos2::new(total_rect.min.x, grid_rect.min.y),
                Pos2::new(total_rect.min.x + row_header_w, total_rect.max.y),
            );

            if row_header_rect.contains(pos) || self.state.row_resizing.is_some() {
                let mut near_row_div = None;
                for (idx, &row) in rows.iter().enumerate() {
                    let (_, y2) = row_pos[idx];
                    let screen_y = grid_rect.min.y + y2;
                    if (pos.y - screen_y).abs() <= SpreadsheetTheme::RESIZE_HANDLE_SLOP {
                        near_row_div = Some((row, screen_y));
                        break;
                    }
                }

                if let Some((row, _)) = near_row_div {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    if response.drag_started() {
                        let init_h = self.worksheet.row_height(row);
                        self.state.row_resizing = Some((row, init_h, pos.y));
                    }
                }

                if let Some((row, init_h, start_y)) = self.state.row_resizing {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    let delta = (pos.y - start_y) / zoom;
                    let new_h = (init_h + delta).max(14.0);
                    actions.push(SpreadsheetAction::ResizeRow { row, height: new_h });

                    if response.drag_stopped() {
                        self.state.row_resizing = None;
                    }
                    return;
                }
            }
        }

        let corner_rect = Rect::from_min_max(
            total_rect.min,
            Pos2::new(total_rect.min.x + row_header_w, total_rect.min.y + col_header_h),
        );
        if let Some(pos) = hover_pos {
            if corner_rect.contains(pos) && response.clicked() {
                self.state.selection = CellRange::new(
                    CellCoord::must_new(1, 1),
                    CellCoord::must_new(MAX_ROWS, MAX_COLS),
                );
                return;
            }
        }

        if let Some(sel_rect) = self.get_range_screen_rect(
            grid_rect,
            cols,
            col_pos,
            rows,
            row_pos,
            self.state.selection,
        ) {
            let handle_size = SpreadsheetTheme::FILL_HANDLE_SIZE * zoom.clamp(0.8, 1.5) + 4.0;
            let handle_hit_rect = Rect::from_center_size(sel_rect.max, vec2(handle_size, handle_size));

            if let Some(pos) = hover_pos {
                if handle_hit_rect.contains(pos) {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::Crosshair);
                    if response.drag_started() {
                        self.state.is_dragging_fill_handle = true;
                        self.state.fill_handle_target_range = Some(self.state.selection);
                    }
                }
            }

            if self.state.is_dragging_fill_handle {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::Crosshair);
                if let Some(pos) = hover_pos {
                    if let Some(hovered_coord) = self.pos_to_cell_coord(grid_rect, cols, col_pos, rows, row_pos, pos) {
                        let orig = self.state.selection;
                        let target = CellRange::new(orig.start, hovered_coord);
                        self.state.fill_handle_target_range = Some(target);
                    }
                }

                if response.drag_stopped() {
                    self.state.is_dragging_fill_handle = false;
                    if let Some(target) = self.state.fill_handle_target_range.take() {
                        if target != self.state.selection {
                            actions.push(SpreadsheetAction::AutoFill {
                                source: self.state.selection,
                                target,
                            });
                            self.state.selection = target;
                        }
                    }
                }
                return;
            }
        }

        if let Some(pos) = hover_pos {
            if grid_rect.contains(pos) {
                if let Some(coord) = self.pos_to_cell_coord(grid_rect, cols, col_pos, rows, row_pos, pos) {
                    if response.double_clicked() {
                        let text = self.worksheet.get_cell(coord)
                            .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                            .unwrap_or_default();
                        self.state.start_editing(text);
                        return;
                    }

                    if response.drag_started() {
                        let shift = ui.input(|i| i.modifiers.shift);
                        if shift {
                            self.state.extend_selection(coord);
                        } else {
                            let formula_text = self.worksheet.get_cell(coord)
                                .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                                .unwrap_or_default();
                            self.state.select_cell(coord, formula_text);
                        }
                        self.state.is_dragging_selection = true;
                    } else if self.state.is_dragging_selection && response.dragged() {
                        self.state.extend_selection(coord);
                    }
                }
            }
        }

        if response.drag_stopped() {
            self.state.is_dragging_selection = false;
        }
    }

    fn handle_grid_keys(&mut self, ui: &mut Ui, actions: &mut Vec<SpreadsheetAction>) {
        if self.state.is_editing {
            return;
        }

        let input = ui.input(|i| i.clone());

        if input.key_pressed(Key::F2) {
            let coord = self.state.active_cell;
            let text = self.worksheet.get_cell(coord)
                .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                .unwrap_or_default();
            self.state.start_editing(text);
            return;
        }

        if input.key_pressed(Key::Delete) {
            actions.push(SpreadsheetAction::ClearCells {
                range: self.state.selection,
            });
            return;
        }

        let shift = input.modifiers.shift;
        let ctrl = input.modifiers.ctrl || input.modifiers.mac_cmd;

        let delta_r = if input.key_pressed(Key::ArrowDown) {
            1
        } else if input.key_pressed(Key::ArrowUp) {
            -1
        } else {
            0
        };

        let delta_c = if input.key_pressed(Key::ArrowRight) {
            1
        } else if input.key_pressed(Key::ArrowLeft) {
            -1
        } else {
            0
        };

        if delta_r != 0 || delta_c != 0 {
            let step = if ctrl { 10 } else { 1 };
            let new_row = ((self.state.active_cell.row as i32) + delta_r * step).clamp(1, MAX_ROWS as i32) as u32;
            let new_col = ((self.state.active_cell.col as i32) + delta_c * step).clamp(1, MAX_COLS as i32) as u32;
            let target = CellCoord::must_new(new_row, new_col);

            if shift {
                self.state.extend_selection(target);
            } else {
                let text = self.worksheet.get_cell(target)
                    .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                    .unwrap_or_default();
                self.state.select_cell(target, text);
            }
            return;
        }

        if input.key_pressed(Key::Enter) {
            let next_row = (self.state.active_cell.row + 1).min(MAX_ROWS);
            let next_coord = CellCoord::must_new(next_row, self.state.active_cell.col);
            let text = self.worksheet.get_cell(next_coord)
                .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                .unwrap_or_default();
            self.state.select_cell(next_coord, text);
            return;
        }

        if input.key_pressed(Key::Tab) {
            let next_col = (self.state.active_cell.col + 1).min(MAX_COLS);
            let next_coord = CellCoord::must_new(self.state.active_cell.row, next_col);
            let text = self.worksheet.get_cell(next_coord)
                .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                .unwrap_or_default();
            self.state.select_cell(next_coord, text);
            return;
        }

        for event in &input.events {
            if let Event::Text(t) = event {
                if !t.is_empty() && !input.modifiers.ctrl && !input.modifiers.mac_cmd && !input.modifiers.alt {
                    self.state.start_editing(t.clone());
                    return;
                }
            }
        }
    }

    fn render_inline_editor(
        &mut self,
        ui: &mut Ui,
        grid_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        _zoom: f32,
        actions: &mut Vec<SpreadsheetAction>,
    ) {
        let coord = self.state.active_cell;
        if let Some(cell_rect) = self.get_cell_screen_rect(grid_rect, cols, col_pos, rows, row_pos, coord.col, coord.row) {
            let mut edit_rect = cell_rect;
            edit_rect.max.x = edit_rect.max.x.max(edit_rect.min.x + 120.0);

            let mut child_ui = ui.new_child(UiBuilder::new().max_rect(edit_rect));
            let editor_response = child_ui.add(
                egui::TextEdit::singleline(&mut self.state.edit_buffer)
                    .id(Id::new("grid_inline_cell_editor"))
            );

            if self.state.edit_focus_requested {
                editor_response.request_focus();
                self.state.edit_focus_requested = false;
            }

            self.state.formula_buffer = self.state.edit_buffer.clone();

            if editor_response.lost_focus() || child_ui.input(|i| i.key_pressed(Key::Enter)) {
                let committed_text = self.state.commit_editing();
                self.apply_cell_edit(coord, committed_text, actions);

                let next_row = (coord.row + 1).min(MAX_ROWS);
                let next_coord = CellCoord::must_new(next_row, coord.col);
                let next_text = self.worksheet.get_cell(next_coord)
                    .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                    .unwrap_or_default();
                self.state.select_cell(next_coord, next_text);
            } else if child_ui.input(|i| i.key_pressed(Key::Escape)) {
                self.state.cancel_editing();
            } else if child_ui.input(|i| i.key_pressed(Key::Tab)) {
                let committed_text = self.state.commit_editing();
                self.apply_cell_edit(coord, committed_text, actions);

                let next_col = (coord.col + 1).min(MAX_COLS);
                let next_coord = CellCoord::must_new(coord.row, next_col);
                let next_text = self.worksheet.get_cell(next_coord)
                    .and_then(|c| c.formula.clone().or_else(|| Some(c.value.to_display_string())))
                    .unwrap_or_default();
                self.state.select_cell(next_coord, next_text);
            }
        }
    }

    fn apply_cell_edit(&self, coord: CellCoord, text: String, actions: &mut Vec<SpreadsheetAction>) {
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

    fn render_context_menu(&mut self, response: &Response, actions: &mut Vec<SpreadsheetAction>) {
        response.context_menu(|ui| {
            if ui.button("Cut (Ctrl+X)").clicked() {
                actions.push(SpreadsheetAction::Cut);
                ui.close();
            }
            if ui.button("Copy (Ctrl+C)").clicked() {
                actions.push(SpreadsheetAction::Copy);
                ui.close();
            }
            if ui.button("Paste (Ctrl+V)").clicked() {
                actions.push(SpreadsheetAction::Paste);
                ui.close();
            }
            ui.separator();
            if ui.button("Clear Contents (Del)").clicked() {
                actions.push(SpreadsheetAction::ClearCells {
                    range: self.state.selection,
                });
                ui.close();
            }
            ui.separator();
            if ui.button("AutoSum").clicked() {
                actions.push(SpreadsheetAction::AutoSum {
                    range: self.state.selection,
                    func: AutoSumFunction::Sum,
                });
                ui.close();
            }
            ui.separator();
            if ui.button("Format Cells... (Ctrl+1)").clicked() {
                self.state.show_format_cells_dialog = true;
                ui.close();
            }
        });
    }

    fn pos_to_cell_coord(
        &self,
        grid_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        pos: Pos2,
    ) -> Option<CellCoord> {
        let rel_x = pos.x - grid_rect.min.x;
        let rel_y = pos.y - grid_rect.min.y;

        let col = cols.iter().zip(col_pos.iter()).find_map(|(&c, &(x1, x2))| {
            if rel_x >= x1 && rel_x <= x2 {
                Some(c)
            } else {
                None
            }
        })?;

        let row = rows.iter().zip(row_pos.iter()).find_map(|(&r, &(y1, y2))| {
            if rel_y >= y1 && rel_y <= y2 {
                Some(r)
            } else {
                None
            }
        })?;

        CellCoord::new(row, col).ok()
    }

    fn get_cell_screen_rect(
        &self,
        grid_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        col: u32,
        row: u32,
    ) -> Option<Rect> {
        let (x1, x2) = cols.iter().position(|&c| c == col).map(|i| col_pos[i])?;
        let (y1, y2) = rows.iter().position(|&r| r == row).map(|i| row_pos[i])?;

        Some(Rect::from_min_max(
            Pos2::new(grid_rect.min.x + x1, grid_rect.min.y + y1),
            Pos2::new(grid_rect.min.x + x2, grid_rect.min.y + y2),
        ))
    }

    fn get_range_screen_rect(
        &self,
        grid_rect: Rect,
        cols: &[u32],
        col_pos: &[(f32, f32)],
        rows: &[u32],
        row_pos: &[(f32, f32)],
        range: CellRange,
    ) -> Option<Rect> {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        let mut has_intersection = false;

        for (idx, &c) in cols.iter().enumerate() {
            if c >= range.start.col && c <= range.end.col {
                has_intersection = true;
                let (x1, x2) = col_pos[idx];
                min_x = min_x.min(grid_rect.min.x + x1);
                max_x = max_x.max(grid_rect.min.x + x2);
            }
        }

        for (idx, &r) in rows.iter().enumerate() {
            if r >= range.start.row && r <= range.end.row {
                has_intersection = true;
                let (y1, y2) = row_pos[idx];
                min_y = min_y.min(grid_rect.min.y + y1);
                max_y = max_y.max(grid_rect.min.y + y2);
            }
        }

        if has_intersection && min_x <= max_x && min_y <= max_y {
            Some(Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y)))
        } else {
            None
        }
    }
}
