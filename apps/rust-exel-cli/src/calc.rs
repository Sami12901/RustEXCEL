use exel_cell::{CellCoord, CellError, CellRange, CellValue};
use exel_workbook::{Workbook, Worksheet};

/// Recalculates all formulas across all sheets in the workbook.
pub fn recalculate_workbook(wb: &mut Workbook) {
    for sheet in &mut wb.sheets {
        recalculate_sheet(sheet);
    }
}

/// Evaluates all formula cells in a single worksheet.
pub fn recalculate_sheet(sheet: &mut Worksheet) {
    let formula_coords: Vec<(CellCoord, String)> = sheet
        .cells
        .iter()
        .filter_map(|(coord, state)| {
            state.formula.as_ref().map(|f| (*coord, f.clone()))
        })
        .collect();

    for (coord, formula) in formula_coords {
        let evaluated_value = evaluate_formula(sheet, &formula);
        if let Some(cell) = sheet.cells.get_mut(coord) {
            cell.value = evaluated_value;
        }
    }
}

/// Evaluates an individual formula string against worksheet cell values.
pub fn evaluate_formula(sheet: &Worksheet, formula: &str) -> CellValue {
    let expr = formula.trim().strip_prefix('=').unwrap_or(formula).trim();

    // 1. Function evaluation: NAME(...)
    if let Some((func_name, args_str)) = parse_function_call(expr) {
        return evaluate_function(sheet, &func_name, &args_str);
    }

    // 2. Simple binary operations: + - * /
    if let Some(val) = evaluate_binary_op(sheet, expr) {
        return val;
    }

    // 3. Single cell reference: e.g. A1
    if let Ok(ref_coord) = CellCoord::from_a1(expr) {
        return sheet.get_cell_value(ref_coord);
    }

    // 4. Number literal
    if let Ok(n) = expr.parse::<f64>() {
        return CellValue::Number(n);
    }

    // 5. String literal e.g. "Text"
    if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
        return CellValue::Text(expr[1..expr.len() - 1].to_string());
    }

    CellValue::Error(CellError::Value)
}

fn parse_function_call(expr: &str) -> Option<(String, String)> {
    if let Some(open_paren) = expr.find('(') {
        if expr.ends_with(')') {
            let func_name = expr[..open_paren].trim().to_ascii_uppercase();
            let args = expr[open_paren + 1..expr.len() - 1].trim().to_string();
            return Some((func_name, args));
        }
    }
    None
}

fn evaluate_function(sheet: &Worksheet, name: &str, args: &str) -> CellValue {
    let numbers = collect_numbers_from_args(sheet, args);

    match name {
        "SUM" => CellValue::Number(numbers.iter().sum()),
        "AVERAGE" => {
            if numbers.is_empty() {
                CellValue::Error(CellError::Div0)
            } else {
                CellValue::Number(numbers.iter().sum::<f64>() / (numbers.len() as f64))
            }
        }
        "COUNT" => CellValue::Number(numbers.len() as f64),
        "MAX" => {
            if numbers.is_empty() {
                CellValue::Number(0.0)
            } else {
                CellValue::Number(numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max))
            }
        }
        "MIN" => {
            if numbers.is_empty() {
                CellValue::Number(0.0)
            } else {
                CellValue::Number(numbers.iter().copied().fold(f64::INFINITY, f64::min))
            }
        }
        _ => CellValue::Error(CellError::Name),
    }
}

fn collect_numbers_from_args(sheet: &Worksheet, args: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();

    for part in parts {
        if let Ok(range) = CellRange::from_a1(part) {
            for r in range.start.row..=range.end.row {
                for c in range.start.col..=range.end.col {
                    if let Ok(coord) = CellCoord::new(r, c) {
                        if let Some(n) = sheet.get_cell_value(coord).as_number() {
                            numbers.push(n);
                        }
                    }
                }
            }
        } else if let Ok(n) = part.parse::<f64>() {
            numbers.push(n);
        } else if let Ok(coord) = CellCoord::from_a1(part) {
            if let Some(n) = sheet.get_cell_value(coord).as_number() {
                numbers.push(n);
            }
        }
    }

    numbers
}

fn evaluate_binary_op(sheet: &Worksheet, expr: &str) -> Option<CellValue> {
    for op in ['+', '-', '*', '/'] {
        if let Some((left_s, right_s)) = expr.split_once(op) {
            let left_val = resolve_operand_number(sheet, left_s.trim())?;
            let right_val = resolve_operand_number(sheet, right_s.trim())?;

            let res = match op {
                '+' => left_val + right_val,
                '-' => left_val - right_val,
                '*' => left_val * right_val,
                '/' => {
                    if right_val == 0.0 {
                        return Some(CellValue::Error(CellError::Div0));
                    }
                    left_val / right_val
                }
                _ => return None,
            };
            return Some(CellValue::Number(res));
        }
    }
    None
}

fn resolve_operand_number(sheet: &Worksheet, operand: &str) -> Option<f64> {
    if let Ok(n) = operand.parse::<f64>() {
        Some(n)
    } else if let Ok(coord) = CellCoord::from_a1(operand) {
        sheet.get_cell_value(coord).as_number()
    } else {
        None
    }
}
