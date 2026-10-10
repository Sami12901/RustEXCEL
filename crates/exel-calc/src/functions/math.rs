use crate::context::CalcContext;
use exel_cell::{CellError, CellValue};
use exel_formula::Expression;

pub type EvalFn<'a> = &'a dyn Fn(&CalcContext, &Expression) -> Result<CellValue, CellError>;

pub fn fn_sum(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut sum = 0.0;
    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Number(n) => sum += n,
                        CellValue::Error(err) => return Err(err),
                        _ => {} // Ignore text, booleans, empty in ranges
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                match val {
                    CellValue::Number(n) => sum += n,
                    CellValue::Boolean(b) => sum += if b { 1.0 } else { 0.0 },
                    CellValue::Text(s) => {
                        let parsed = s.trim().parse::<f64>().map_err(|_| CellError::Value)?;
                        sum += parsed;
                    }
                    CellValue::Empty => {}
                    CellValue::Error(err) => return Err(err),
                    _ => return Err(CellError::Value),
                }
            }
        }
    }
    Ok(CellValue::Number(sum))
}

pub fn fn_product(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut product = 1.0;
    let mut has_number = false;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Number(n) => {
                            product *= n;
                            has_number = true;
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
                        product *= n;
                        has_number = true;
                    }
                    CellValue::Boolean(b) => {
                        product *= if b { 1.0 } else { 0.0 };
                        has_number = true;
                    }
                    CellValue::Text(s) => {
                        let parsed = s.trim().parse::<f64>().map_err(|_| CellError::Value)?;
                        product *= parsed;
                        has_number = true;
                    }
                    CellValue::Empty => {}
                    CellValue::Error(err) => return Err(err),
                    _ => return Err(CellError::Value),
                }
            }
        }
    }

    if has_number {
        Ok(CellValue::Number(product))
    } else {
        Ok(CellValue::Number(0.0))
    }
}

pub fn fn_abs(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let n = val.as_number().ok_or(CellError::Value)?;
    Ok(CellValue::Number(n.abs()))
}

pub fn fn_round(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 2 {
        return Err(CellError::Value);
    }
    let num_val = eval(ctx, &args[0])?;
    let digits_val = eval(ctx, &args[1])?;

    let n = num_val.as_number().ok_or(CellError::Value)?;
    let d = digits_val.as_number().ok_or(CellError::Value)? as i32;

    let factor = 10f64.powi(d);
    let rounded = (n * factor).round() / factor;
    Ok(CellValue::Number(rounded))
}

pub fn fn_int(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let n = val.as_number().ok_or(CellError::Value)?;
    Ok(CellValue::Number(n.floor()))
}

pub fn fn_mod(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 2 {
        return Err(CellError::Value);
    }
    let n_val = eval(ctx, &args[0])?;
    let d_val = eval(ctx, &args[1])?;

    let n = n_val.as_number().ok_or(CellError::Value)?;
    let d = d_val.as_number().ok_or(CellError::Value)?;

    if d == 0.0 {
        return Err(CellError::Div0);
    }

    let result = n - d * (n / d).floor();
    Ok(CellValue::Number(result))
}

pub fn fn_power(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 2 {
        return Err(CellError::Value);
    }
    let base_val = eval(ctx, &args[0])?;
    let exp_val = eval(ctx, &args[1])?;

    let base = base_val.as_number().ok_or(CellError::Value)?;
    let exp = exp_val.as_number().ok_or(CellError::Value)?;

    if base == 0.0 && exp < 0.0 {
        return Err(CellError::Div0);
    }
    if base < 0.0 && exp.fract() != 0.0 {
        return Err(CellError::Num);
    }

    let result = base.powf(exp);
    if result.is_nan() || result.is_infinite() {
        return Err(CellError::Num);
    }
    Ok(CellValue::Number(result))
}

pub fn fn_sqrt(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let n = val.as_number().ok_or(CellError::Value)?;
    if n < 0.0 {
        return Err(CellError::Num);
    }
    Ok(CellValue::Number(n.sqrt()))
}

pub fn fn_pi(
    _ctx: &CalcContext,
    args: &[Expression],
    _eval: EvalFn,
) -> Result<CellValue, CellError> {
    if !args.is_empty() {
        return Err(CellError::Value);
    }
    Ok(CellValue::Number(std::f64::consts::PI))
}
