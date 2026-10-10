use crate::context::CalcContext;
use crate::functions::math::EvalFn;
use exel_cell::{CellError, CellValue};
use exel_formula::Expression;

pub fn fn_average(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut sum = 0.0;
    let mut count = 0usize;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Number(n) => {
                            sum += n;
                            count += 1;
                        }
                        CellValue::Error(err) => return Err(err),
                        _ => {}
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                match val {
                    CellValue::Number(n) => {
                        sum += n;
                        count += 1;
                    }
                    CellValue::Boolean(b) => {
                        sum += if b { 1.0 } else { 0.0 };
                        count += 1;
                    }
                    CellValue::Text(s) => {
                        let parsed = s.trim().parse::<f64>().map_err(|_| CellError::Value)?;
                        sum += parsed;
                        count += 1;
                    }
                    CellValue::Empty => {}
                    CellValue::Error(err) => return Err(err),
                    _ => return Err(CellError::Value),
                }
            }
        }
    }

    if count == 0 {
        return Err(CellError::Div0);
    }

    Ok(CellValue::Number(sum / count as f64))
}

pub fn fn_min(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut min_val: Option<f64> = None;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Number(n) => {
                            min_val = Some(min_val.map_or(n, |m| m.min(n)));
                        }
                        CellValue::Error(err) => return Err(err),
                        _ => {}
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                match val {
                    CellValue::Number(n) => {
                        min_val = Some(min_val.map_or(n, |m| m.min(n)));
                    }
                    CellValue::Boolean(b) => {
                        let n = if b { 1.0 } else { 0.0 };
                        min_val = Some(min_val.map_or(n, |m| m.min(n)));
                    }
                    CellValue::Text(s) => {
                        let n = s.trim().parse::<f64>().map_err(|_| CellError::Value)?;
                        min_val = Some(min_val.map_or(n, |m| m.min(n)));
                    }
                    CellValue::Empty => {}
                    CellValue::Error(err) => return Err(err),
                    _ => return Err(CellError::Value),
                }
            }
        }
    }

    Ok(CellValue::Number(min_val.unwrap_or(0.0)))
}

pub fn fn_max(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut max_val: Option<f64> = None;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Number(n) => {
                            max_val = Some(max_val.map_or(n, |m| m.max(n)));
                        }
                        CellValue::Error(err) => return Err(err),
                        _ => {}
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                match val {
                    CellValue::Number(n) => {
                        max_val = Some(max_val.map_or(n, |m| m.max(n)));
                    }
                    CellValue::Boolean(b) => {
                        let n = if b { 1.0 } else { 0.0 };
                        max_val = Some(max_val.map_or(n, |m| m.max(n)));
                    }
                    CellValue::Text(s) => {
                        let n = s.trim().parse::<f64>().map_err(|_| CellError::Value)?;
                        max_val = Some(max_val.map_or(n, |m| m.max(n)));
                    }
                    CellValue::Empty => {}
                    CellValue::Error(err) => return Err(err),
                    _ => return Err(CellError::Value),
                }
            }
        }
    }

    Ok(CellValue::Number(max_val.unwrap_or(0.0)))
}

pub fn fn_count(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut count = 0usize;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Number(_) => count += 1,
                        CellValue::Error(err) => return Err(err),
                        _ => {}
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                match val {
                    CellValue::Number(_) => count += 1,
                    CellValue::Text(s) => {
                        if s.trim().parse::<f64>().is_ok() {
                            count += 1;
                        }
                    }
                    CellValue::Error(err) => return Err(err),
                    _ => {}
                }
            }
        }
    }

    Ok(CellValue::Number(count as f64))
}

pub fn fn_counta(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut count = 0usize;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    if !val.is_empty() {
                        count += 1;
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                if !val.is_empty() {
                    count += 1;
                }
            }
        }
    }

    Ok(CellValue::Number(count as f64))
}
