use crate::context::CalcContext;
use crate::functions::math::EvalFn;
use exel_cell::{CellError, CellValue};
use exel_formula::Expression;

fn coerce_to_bool(val: &CellValue) -> Result<bool, CellError> {
    match val {
        CellValue::Boolean(b) => Ok(*b),
        CellValue::Number(n) => Ok(*n != 0.0),
        CellValue::Text(s) => {
            if s.eq_ignore_ascii_case("TRUE") {
                Ok(true)
            } else if s.eq_ignore_ascii_case("FALSE") {
                Ok(false)
            } else {
                Err(CellError::Value)
            }
        }
        CellValue::Empty => Ok(false),
        CellValue::Error(err) => Err(*err),
        _ => Err(CellError::Value),
    }
}

pub fn fn_if(ctx: &CalcContext, args: &[Expression], eval: EvalFn) -> Result<CellValue, CellError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(CellError::Value);
    }
    let cond_val = eval(ctx, &args[0])?;
    let condition = coerce_to_bool(&cond_val)?;

    if condition {
        eval(ctx, &args[1])
    } else if args.len() == 3 {
        eval(ctx, &args[2])
    } else {
        Ok(CellValue::Boolean(false))
    }
}

pub fn fn_and(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let mut evaluated_any = false;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Boolean(b) => {
                            evaluated_any = true;
                            if !b {
                                return Ok(CellValue::Boolean(false));
                            }
                        }
                        CellValue::Number(n) => {
                            evaluated_any = true;
                            if n == 0.0 {
                                return Ok(CellValue::Boolean(false));
                            }
                        }
                        CellValue::Error(err) => return Err(err),
                        _ => {}
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                let b = coerce_to_bool(&val)?;
                evaluated_any = true;
                if !b {
                    return Ok(CellValue::Boolean(false));
                }
            }
        }
    }

    if evaluated_any {
        Ok(CellValue::Boolean(true))
    } else {
        Err(CellError::Value)
    }
}

pub fn fn_or(ctx: &CalcContext, args: &[Expression], eval: EvalFn) -> Result<CellValue, CellError> {
    if args.is_empty() {
        return Err(CellError::Value);
    }
    let mut evaluated_any = false;

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    match val {
                        CellValue::Boolean(b) => {
                            evaluated_any = true;
                            if b {
                                return Ok(CellValue::Boolean(true));
                            }
                        }
                        CellValue::Number(n) => {
                            evaluated_any = true;
                            if n != 0.0 {
                                return Ok(CellValue::Boolean(true));
                            }
                        }
                        CellValue::Error(err) => return Err(err),
                        _ => {}
                    }
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                let b = coerce_to_bool(&val)?;
                evaluated_any = true;
                if b {
                    return Ok(CellValue::Boolean(true));
                }
            }
        }
    }

    if evaluated_any {
        Ok(CellValue::Boolean(false))
    } else {
        Err(CellError::Value)
    }
}

pub fn fn_not(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let b = coerce_to_bool(&val)?;
    Ok(CellValue::Boolean(!b))
}

pub fn fn_iferror(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 2 {
        return Err(CellError::Value);
    }
    match eval(ctx, &args[0]) {
        Ok(CellValue::Error(_)) | Err(_) => eval(ctx, &args[1]),
        Ok(val) => Ok(val),
    }
}
