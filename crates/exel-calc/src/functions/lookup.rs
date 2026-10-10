use crate::context::CalcContext;
use crate::functions::math::EvalFn;
use exel_cell::{CellError, CellValue};
use exel_formula::Expression;
use std::cmp::Ordering;

fn is_exact_match(a: &CellValue, b: &CellValue) -> bool {
    match (a, b) {
        (CellValue::Number(n1), CellValue::Number(n2)) => (n1 - n2).abs() < 1e-9,
        (CellValue::Text(s1), CellValue::Text(s2)) => s1.eq_ignore_ascii_case(s2),
        (CellValue::Boolean(b1), CellValue::Boolean(b2)) => b1 == b2,
        (CellValue::Empty, CellValue::Empty) => true,
        // Also support string coercion if user looked up number as string or vice versa
        (CellValue::Number(n1), CellValue::Text(s2)) => {
            if let Ok(n2) = s2.trim().parse::<f64>() {
                (n1 - n2).abs() < 1e-9
            } else {
                false
            }
        }
        (CellValue::Text(s1), CellValue::Number(n2)) => {
            if let Ok(n1) = s1.trim().parse::<f64>() {
                (n1 - n2).abs() < 1e-9
            } else {
                false
            }
        }
        _ => false,
    }
}

fn compare_for_lookup(a: &CellValue, b: &CellValue) -> Option<Ordering> {
    match (a, b) {
        (CellValue::Number(n1), CellValue::Number(n2)) => n1.partial_cmp(n2),
        (CellValue::Text(s1), CellValue::Text(s2)) => {
            Some(s1.to_ascii_uppercase().cmp(&s2.to_ascii_uppercase()))
        }
        (CellValue::Boolean(b1), CellValue::Boolean(b2)) => Some(b1.cmp(b2)),
        // In Excel: Numbers < Text < Booleans
        (CellValue::Number(_), CellValue::Text(_)) => Some(Ordering::Less),
        (CellValue::Text(_), CellValue::Number(_)) => Some(Ordering::Greater),
        (CellValue::Number(_), CellValue::Boolean(_)) => Some(Ordering::Less),
        (CellValue::Boolean(_), CellValue::Number(_)) => Some(Ordering::Greater),
        (CellValue::Text(_), CellValue::Boolean(_)) => Some(Ordering::Less),
        (CellValue::Boolean(_), CellValue::Text(_)) => Some(Ordering::Greater),
        _ => None,
    }
}

pub fn fn_vlookup(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() < 3 || args.len() > 4 {
        return Err(CellError::Value);
    }

    let lookup_val = eval(ctx, &args[0])?;
    if let CellValue::Error(err) = lookup_val {
        return Err(err);
    }

    let range = match &args[1] {
        Expression::Range(r) => r,
        _ => return Err(CellError::Value),
    };

    let col_idx_val = eval(ctx, &args[2])?;
    let col_idx = col_idx_val.as_number().ok_or(CellError::Value)? as i32;
    if col_idx < 1 {
        return Err(CellError::Value);
    }

    let range_lookup = if args.len() == 4 {
        let val = eval(ctx, &args[3])?;
        match val {
            CellValue::Boolean(b) => b,
            CellValue::Number(n) => n != 0.0,
            _ => true,
        }
    } else {
        true // Excel default is TRUE (approximate match)
    };

    let grid = ctx.get_range_grid(None, range);
    if grid.is_empty() || grid[0].is_empty() {
        return Err(CellError::Ref);
    }

    if (col_idx as usize) > grid[0].len() {
        return Err(CellError::Ref);
    }

    let target_col = (col_idx - 1) as usize;

    if !range_lookup {
        // Exact match
        for row in &grid {
            if is_exact_match(&row[0], &lookup_val) {
                return Ok(row[target_col].clone());
            }
        }
        Err(CellError::NA)
    } else {
        // Approximate match: find largest value in column 0 that is <= lookup_val
        let mut best_row_idx: Option<usize> = None;

        for (i, row) in grid.iter().enumerate() {
            if let Some(ord) = compare_for_lookup(&row[0], &lookup_val) {
                if ord == Ordering::Less || ord == Ordering::Equal {
                    best_row_idx = Some(i);
                } else if ord == Ordering::Greater {
                    // Assumes sorted ascending; once greater, stop
                    break;
                }
            }
        }

        if let Some(row_idx) = best_row_idx {
            Ok(grid[row_idx][target_col].clone())
        } else {
            Err(CellError::NA)
        }
    }
}

pub fn fn_match(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(CellError::Value);
    }

    let lookup_val = eval(ctx, &args[0])?;
    if let CellValue::Error(err) = lookup_val {
        return Err(err);
    }

    let range = match &args[1] {
        Expression::Range(r) => r,
        _ => return Err(CellError::Value),
    };

    let match_type = if args.len() == 3 {
        let m_val = eval(ctx, &args[2])?;
        m_val.as_number().ok_or(CellError::Value)? as i32
    } else {
        1 // Default is 1 (largest <=)
    };

    let grid = ctx.get_range_grid(None, range);
    if grid.is_empty() {
        return Err(CellError::NA);
    }

    // Must be 1D array
    let items: Vec<CellValue> = if grid.len() == 1 {
        grid[0].clone()
    } else if grid[0].len() == 1 {
        grid.iter().map(|row| row[0].clone()).collect()
    } else {
        return Err(CellError::NA);
    };

    if match_type == 0 {
        // Exact match
        for (idx, item) in items.iter().enumerate() {
            if is_exact_match(item, &lookup_val) {
                return Ok(CellValue::Number((idx + 1) as f64));
            }
        }
        Err(CellError::NA)
    } else if match_type == 1 {
        // Largest <= lookup_val (ascending)
        let mut best_idx: Option<usize> = None;
        for (idx, item) in items.iter().enumerate() {
            if let Some(ord) = compare_for_lookup(item, &lookup_val) {
                if ord == Ordering::Less || ord == Ordering::Equal {
                    best_idx = Some(idx + 1);
                } else if ord == Ordering::Greater {
                    break;
                }
            }
        }
        best_idx
            .map(|i| CellValue::Number(i as f64))
            .ok_or(CellError::NA)
    } else if match_type == -1 {
        // Smallest >= lookup_val (descending)
        let mut best_idx: Option<usize> = None;
        for (idx, item) in items.iter().enumerate() {
            if let Some(ord) = compare_for_lookup(item, &lookup_val) {
                if ord == Ordering::Greater || ord == Ordering::Equal {
                    best_idx = Some(idx + 1);
                } else if ord == Ordering::Less {
                    break;
                }
            }
        }
        best_idx
            .map(|i| CellValue::Number(i as f64))
            .ok_or(CellError::NA)
    } else {
        Err(CellError::Value)
    }
}

pub fn fn_index(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(CellError::Value);
    }

    let range = match &args[0] {
        Expression::Range(r) => r,
        _ => return Err(CellError::Value),
    };

    let row_val = eval(ctx, &args[1])?;
    let row_idx = row_val.as_number().ok_or(CellError::Value)? as i32;
    if row_idx < 1 {
        return Err(CellError::Ref);
    }

    let grid = ctx.get_range_grid(None, range);
    if grid.is_empty() || grid[0].is_empty() {
        return Err(CellError::Ref);
    }

    let (r, c) = if args.len() == 3 {
        let col_val = eval(ctx, &args[2])?;
        let col_idx = col_val.as_number().ok_or(CellError::Value)? as i32;
        if col_idx < 1 {
            return Err(CellError::Ref);
        }
        (row_idx as usize, col_idx as usize)
    } else {
        // 1D array handling
        if grid.len() == 1 {
            (1, row_idx as usize)
        } else if grid[0].len() == 1 {
            (row_idx as usize, 1)
        } else {
            return Err(CellError::Ref);
        }
    };

    if r > grid.len() || c > grid[0].len() {
        return Err(CellError::Ref);
    }

    Ok(grid[r - 1][c - 1].clone())
}
