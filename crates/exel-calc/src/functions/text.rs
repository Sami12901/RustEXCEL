use crate::context::CalcContext;
use crate::functions::math::EvalFn;
use exel_cell::{CellError, CellValue};
use exel_formula::Expression;

fn to_string_val(val: &CellValue) -> String {
    val.to_display_string()
}

pub fn fn_concat(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    let mut out = String::new();

    for arg in args {
        match arg {
            Expression::Range(range) => {
                let values = ctx.get_range_values(None, range);
                for val in values {
                    if let CellValue::Error(err) = val {
                        return Err(err);
                    }
                    out.push_str(&to_string_val(&val));
                }
            }
            _ => {
                let val = eval(ctx, arg)?;
                out.push_str(&to_string_val(&val));
            }
        }
    }

    Ok(CellValue::Text(out))
}

pub fn fn_left(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.is_empty() || args.len() > 2 {
        return Err(CellError::Value);
    }
    let text_val = eval(ctx, &args[0])?;
    let text = to_string_val(&text_val);

    let num_chars = if args.len() == 2 {
        let n_val = eval(ctx, &args[1])?;
        let n = n_val.as_number().ok_or(CellError::Value)? as i32;
        if n < 0 {
            return Err(CellError::Value);
        }
        n as usize
    } else {
        1
    };

    let result: String = text.chars().take(num_chars).collect();
    Ok(CellValue::Text(result))
}

pub fn fn_right(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.is_empty() || args.len() > 2 {
        return Err(CellError::Value);
    }
    let text_val = eval(ctx, &args[0])?;
    let text = to_string_val(&text_val);

    let num_chars = if args.len() == 2 {
        let n_val = eval(ctx, &args[1])?;
        let n = n_val.as_number().ok_or(CellError::Value)? as i32;
        if n < 0 {
            return Err(CellError::Value);
        }
        n as usize
    } else {
        1
    };

    let char_count = text.chars().count();
    let skip_count = char_count.saturating_sub(num_chars);
    let result: String = text.chars().skip(skip_count).collect();
    Ok(CellValue::Text(result))
}

pub fn fn_mid(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 3 {
        return Err(CellError::Value);
    }
    let text_val = eval(ctx, &args[0])?;
    let start_val = eval(ctx, &args[1])?;
    let count_val = eval(ctx, &args[2])?;

    let text = to_string_val(&text_val);
    let start_num = start_val.as_number().ok_or(CellError::Value)? as i32;
    let count = count_val.as_number().ok_or(CellError::Value)? as i32;

    if start_num < 1 || count < 0 {
        return Err(CellError::Value);
    }

    let skip_count = (start_num - 1) as usize;
    let take_count = count as usize;
    let result: String = text.chars().skip(skip_count).take(take_count).collect();
    Ok(CellValue::Text(result))
}

pub fn fn_len(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let text = to_string_val(&val);
    Ok(CellValue::Number(text.chars().count() as f64))
}

pub fn fn_upper(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let text = to_string_val(&val);
    Ok(CellValue::Text(text.to_uppercase()))
}

pub fn fn_lower(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let text = to_string_val(&val);
    Ok(CellValue::Text(text.to_lowercase()))
}

pub fn fn_trim(
    ctx: &CalcContext,
    args: &[Expression],
    eval: EvalFn,
) -> Result<CellValue, CellError> {
    if args.len() != 1 {
        return Err(CellError::Value);
    }
    let val = eval(ctx, &args[0])?;
    let text = to_string_val(&val);

    let trimmed = text.trim();
    let mut result = String::with_capacity(trimmed.len());
    let mut last_was_space = false;

    for ch in trimmed.chars() {
        if ch == ' ' {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    Ok(CellValue::Text(result))
}
