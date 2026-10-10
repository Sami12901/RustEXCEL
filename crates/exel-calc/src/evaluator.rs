use crate::context::CalcContext;
use crate::functions::FunctionRegistry;
use exel_cell::{CellError, CellValue};
use exel_formula::{BinaryOp, Expression, Literal, UnaryOp};
use std::cmp::Ordering;

/// Evaluates an AST expression in the given calculation context.
pub fn evaluate_expression(
    registry: &FunctionRegistry,
    ctx: &CalcContext,
    expr: &Expression,
) -> Result<CellValue, CellError> {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Number(n) => Ok(CellValue::Number(*n)),
            Literal::String(s) => Ok(CellValue::Text(s.clone())),
            Literal::Boolean(b) => Ok(CellValue::Boolean(*b)),
        },

        Expression::Cell(cell_ref) => {
            let val = ctx.get_cell_value(cell_ref.sheet.as_deref(), cell_ref.coord);
            Ok(val)
        }

        Expression::Range(range_ref) => {
            // In a scalar context, Excel returns the top-left cell of the range
            let val = ctx.get_cell_value(range_ref.sheet.as_deref(), range_ref.start.coord);
            Ok(val)
        }

        Expression::UnaryOp { op, expr } => {
            let val = evaluate_expression(registry, ctx, expr)?;
            if let CellValue::Error(err) = val {
                return Err(err);
            }
            let n = coerce_to_number(&val)?;
            match op {
                UnaryOp::Plus => Ok(CellValue::Number(n)),
                UnaryOp::Minus => Ok(CellValue::Number(-n)),
            }
        }

        Expression::BinaryOp { op, left, right } => {
            let l_val = evaluate_expression(registry, ctx, left)?;
            if let CellValue::Error(err) = l_val {
                return Err(err);
            }
            let r_val = evaluate_expression(registry, ctx, right)?;
            if let CellValue::Error(err) = r_val {
                return Err(err);
            }

            match op {
                BinaryOp::Add => {
                    let l = coerce_to_number(&l_val)?;
                    let r = coerce_to_number(&r_val)?;
                    Ok(CellValue::Number(l + r))
                }
                BinaryOp::Sub => {
                    let l = coerce_to_number(&l_val)?;
                    let r = coerce_to_number(&r_val)?;
                    Ok(CellValue::Number(l - r))
                }
                BinaryOp::Mul => {
                    let l = coerce_to_number(&l_val)?;
                    let r = coerce_to_number(&r_val)?;
                    Ok(CellValue::Number(l * r))
                }
                BinaryOp::Div => {
                    let l = coerce_to_number(&l_val)?;
                    let r = coerce_to_number(&r_val)?;
                    if r == 0.0 {
                        return Err(CellError::Div0);
                    }
                    Ok(CellValue::Number(l / r))
                }
                BinaryOp::Power => {
                    let l = coerce_to_number(&l_val)?;
                    let r = coerce_to_number(&r_val)?;
                    if l == 0.0 && r < 0.0 {
                        return Err(CellError::Div0);
                    }
                    if l < 0.0 && r.fract() != 0.0 {
                        return Err(CellError::Num);
                    }
                    let res = l.powf(r);
                    if res.is_nan() || res.is_infinite() {
                        return Err(CellError::Num);
                    }
                    Ok(CellValue::Number(res))
                }
                BinaryOp::Concat => {
                    let l_str = l_val.to_display_string();
                    let r_str = r_val.to_display_string();
                    Ok(CellValue::Text(format!("{}{}", l_str, r_str)))
                }
                BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::Lte
                | BinaryOp::Gt
                | BinaryOp::Gte => {
                    let ord = compare_values(&l_val, &r_val);
                    let cmp_result = match op {
                        BinaryOp::Eq => ord == Some(Ordering::Equal),
                        BinaryOp::NotEq => ord != Some(Ordering::Equal),
                        BinaryOp::Lt => ord == Some(Ordering::Less),
                        BinaryOp::Lte => {
                            ord == Some(Ordering::Less) || ord == Some(Ordering::Equal)
                        }
                        BinaryOp::Gt => ord == Some(Ordering::Greater),
                        BinaryOp::Gte => {
                            ord == Some(Ordering::Greater) || ord == Some(Ordering::Equal)
                        }
                        _ => unreachable!(),
                    };
                    Ok(CellValue::Boolean(cmp_result))
                }
            }
        }

        Expression::FunctionCall { name, args } => {
            if let Some(handler) = registry.get(name) {
                let eval_cb = |c: &CalcContext, e: &Expression| evaluate_expression(registry, c, e);
                handler(ctx, args, &eval_cb)
            } else {
                Err(CellError::Name)
            }
        }
    }
}

/// Compares two CellValues following Excel ordering rules:
/// Numbers < Text < Booleans.
/// Text comparison is case-insensitive.
fn compare_values(a: &CellValue, b: &CellValue) -> Option<Ordering> {
    match (a, b) {
        (CellValue::Number(n1), CellValue::Number(n2)) => n1.partial_cmp(n2),
        (CellValue::Text(s1), CellValue::Text(s2)) => {
            Some(s1.to_ascii_uppercase().cmp(&s2.to_ascii_uppercase()))
        }
        (CellValue::Boolean(b1), CellValue::Boolean(b2)) => Some(b1.cmp(b2)),
        (CellValue::Empty, CellValue::Empty) => Some(Ordering::Equal),
        (CellValue::Empty, CellValue::Number(n)) => 0.0f64.partial_cmp(n),
        (CellValue::Number(n), CellValue::Empty) => n.partial_cmp(&0.0f64),
        (CellValue::Empty, CellValue::Text(s)) => Some("".cmp(s.as_str())),
        (CellValue::Text(s), CellValue::Empty) => Some(s.as_str().cmp("")),
        // Numbers < Text < Booleans
        (CellValue::Number(_), CellValue::Text(_)) => Some(Ordering::Less),
        (CellValue::Text(_), CellValue::Number(_)) => Some(Ordering::Greater),
        (CellValue::Number(_), CellValue::Boolean(_)) => Some(Ordering::Less),
        (CellValue::Boolean(_), CellValue::Number(_)) => Some(Ordering::Greater),
        (CellValue::Text(_), CellValue::Boolean(_)) => Some(Ordering::Less),
        (CellValue::Boolean(_), CellValue::Text(_)) => Some(Ordering::Greater),
        _ => None,
    }
}

pub fn coerce_to_number(val: &CellValue) -> Result<f64, CellError> {
    match val {
        CellValue::Number(n) => Ok(*n),
        CellValue::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
        CellValue::Empty => Ok(0.0),
        CellValue::Text(s) => s.trim().parse::<f64>().map_err(|_| CellError::Value),
        CellValue::Error(err) => Err(*err),
        _ => Err(CellError::Value),
    }
}

