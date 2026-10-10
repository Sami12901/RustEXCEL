use exel_formula::{parse_formula, BinaryOp, Expression, Literal, UnaryOp};

#[test]
fn test_complex_precedence_and_associativity() {
    // 1 + 2 * 3 ^ 2 - 4 / 2
    // 3 ^ 2 = 9
    // 2 * 9 = 18
    // 1 + 18 = 19
    // 4 / 2 = 2
    // 19 - 2 = 17
    let expr = parse_formula("=1 + 2 * 3 ^ 2 - 4 / 2").unwrap();

    // Outermost should be Sub(-)
    if let Expression::BinaryOp { op, left, right } = expr {
        assert_eq!(op, BinaryOp::Sub);
        // right is 4 / 2
        if let Expression::BinaryOp { op: r_op, .. } = *right {
            assert_eq!(r_op, BinaryOp::Div);
        } else {
            panic!("Expected Div on right");
        }
        // left is 1 + (2 * (3 ^ 2))
        if let Expression::BinaryOp { op: l_op, .. } = *left {
            assert_eq!(l_op, BinaryOp::Add);
        } else {
            panic!("Expected Add on left");
        }
    } else {
        panic!("Expected Sub");
    }
}

#[test]
fn test_case_insensitivity_and_whitespace() {
    let expr = parse_formula("  =  sum (   a1 : $b$10 , 42  )  ").unwrap();
    if let Expression::FunctionCall { name, args } = expr {
        assert_eq!(name, "SUM");
        assert_eq!(args.len(), 2);
    } else {
        panic!("Expected FunctionCall");
    }
}

#[test]
fn test_sheet_names_with_escaped_quotes() {
    let expr = parse_formula("='Bob''s Budget'!A1 + '2026 Projections'!$C$5").unwrap();
    if let Expression::BinaryOp { left, right, .. } = expr {
        if let Expression::Cell(c1) = *left {
            assert_eq!(c1.sheet, Some("Bob's Budget".to_string()));
        } else {
            panic!("Expected cell on left");
        }
        if let Expression::Cell(c2) = *right {
            assert_eq!(c2.sheet, Some("2026 Projections".to_string()));
            assert!(c2.col_absolute);
            assert!(c2.row_absolute);
        } else {
            panic!("Expected cell on right");
        }
    } else {
        panic!("Expected BinaryOp");
    }
}

#[test]
fn test_comparison_operators_and_not_equal() {
    let expr1 = parse_formula("=A1 <> B1").unwrap();
    assert!(matches!(
        expr1,
        Expression::BinaryOp {
            op: BinaryOp::NotEq,
            ..
        }
    ));

    let expr2 = parse_formula("=A1 <= B1").unwrap();
    assert!(matches!(
        expr2,
        Expression::BinaryOp {
            op: BinaryOp::Lte,
            ..
        }
    ));

    let expr3 = parse_formula("=A1 >= B1").unwrap();
    assert!(matches!(
        expr3,
        Expression::BinaryOp {
            op: BinaryOp::Gte,
            ..
        }
    ));
}

#[test]
fn test_nested_function_calls() {
    let expr = parse_formula("=IF(ISNUMBER(A1), ROUND(SQRT(A1), 2), 0)").unwrap();
    if let Expression::FunctionCall { name, args } = expr {
        assert_eq!(name, "IF");
        assert_eq!(args.len(), 3);
        assert!(matches!(&args[0], Expression::FunctionCall { name, .. } if name == "ISNUMBER"));
        assert!(matches!(&args[1], Expression::FunctionCall { name, .. } if name == "ROUND"));
        assert_eq!(args[2], Expression::Literal(Literal::Number(0.0)));
    } else {
        panic!("Expected IF");
    }
}

#[test]
fn test_multiple_unary_negations() {
    let expr = parse_formula("=- -A1").unwrap();
    if let Expression::UnaryOp {
        op: op1,
        expr: inner,
    } = expr
    {
        assert_eq!(op1, UnaryOp::Minus);
        if let Expression::UnaryOp { op: op2, .. } = *inner {
            assert_eq!(op2, UnaryOp::Minus);
        } else {
            panic!("Expected nested minus");
        }
    } else {
        panic!("Expected UnaryOp");
    }
}

#[test]
fn test_max_row_and_column() {
    let expr = parse_formula("=XFD1048576").unwrap();
    if let Expression::Cell(cell) = expr {
        assert_eq!(cell.coord.col, 16384);
        assert_eq!(cell.coord.row, 1048576);
    } else {
        panic!("Expected CellRef");
    }
}
