use crate::ast::{BinaryOp, Expression, Literal, RangeRef, UnaryOp};
use crate::error::FormulaError;
use crate::lexer::Lexer;
use crate::token::{Token, TokenKind};

/// Recursive descent parser for Excel formula syntax.
pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &str) -> Result<Self, FormulaError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        Ok(Self::new(tokens))
    }

    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.cursor)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.cursor];
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<(), FormulaError> {
        let tok = self.current_token();
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(expected) {
            self.advance();
            Ok(())
        } else {
            Err(FormulaError::UnexpectedToken {
                token: tok.kind.to_string(),
                pos: tok.pos,
            })
        }
    }

    pub fn parse(&mut self) -> Result<Expression, FormulaError> {
        if matches!(self.peek(), TokenKind::Eof) {
            return Err(FormulaError::EmptyFormula);
        }

        let expr = self.parse_comparison()?;

        if !matches!(self.peek(), TokenKind::Eof) {
            let tok = self.current_token();
            return Err(FormulaError::UnexpectedToken {
                token: tok.kind.to_string(),
                pos: tok.pos,
            });
        }

        Ok(expr)
    }

    // Level 1: Comparison (=, <>, <, <=, >, >=)
    fn parse_comparison(&mut self) -> Result<Expression, FormulaError> {
        let mut expr = self.parse_concat()?;

        loop {
            let op = match self.peek() {
                TokenKind::Equal => BinaryOp::Eq,
                TokenKind::NotEqual => BinaryOp::NotEq,
                TokenKind::Less => BinaryOp::Lt,
                TokenKind::LessEqual => BinaryOp::Lte,
                TokenKind::Greater => BinaryOp::Gt,
                TokenKind::GreaterEqual => BinaryOp::Gte,
                _ => break,
            };

            self.advance();
            let right = self.parse_concat()?;
            expr = Expression::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    // Level 2: String concatenation (&)
    fn parse_concat(&mut self) -> Result<Expression, FormulaError> {
        let mut expr = self.parse_additive()?;

        while matches!(self.peek(), TokenKind::Ampersand) {
            self.advance();
            let right = self.parse_additive()?;
            expr = Expression::BinaryOp {
                op: BinaryOp::Concat,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    // Level 3: Addition and Subtraction (+, -)
    fn parse_additive(&mut self) -> Result<Expression, FormulaError> {
        let mut expr = self.parse_multiplicative()?;

        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative()?;
            expr = Expression::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    // Level 4: Multiplication and Division (*, /)
    fn parse_multiplicative(&mut self) -> Result<Expression, FormulaError> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = match self.peek() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => break,
            };

            self.advance();
            let right = self.parse_unary()?;
            expr = Expression::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    // Level 5: Unary operations (+, -)
    fn parse_unary(&mut self) -> Result<Expression, FormulaError> {
        match self.peek() {
            TokenKind::Plus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOp::Plus,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOp::Minus,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_power(),
        }
    }

    // Level 6: Power (^) - Right associative
    fn parse_power(&mut self) -> Result<Expression, FormulaError> {
        let mut expr = self.parse_postfix()?;

        if matches!(self.peek(), TokenKind::Caret) {
            self.advance();
            let right = self.parse_power()?;
            expr = Expression::BinaryOp {
                op: BinaryOp::Power,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    // Level 7: Postfix operators (e.g. % -> divide by 100)
    fn parse_postfix(&mut self) -> Result<Expression, FormulaError> {
        let mut expr = self.parse_primary()?;

        while matches!(self.peek(), TokenKind::Percent) {
            self.advance();
            expr = Expression::BinaryOp {
                op: BinaryOp::Div,
                left: Box::new(expr),
                right: Box::new(Expression::Literal(Literal::Number(100.0))),
            };
        }

        Ok(expr)
    }

    // Level 8: Primary expression: Literals, References, Ranges, Function calls, (expr)
    fn parse_primary(&mut self) -> Result<Expression, FormulaError> {
        let tok = self.current_token().clone();

        match tok.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expression::Literal(Literal::Number(n)))
            }
            TokenKind::String(s) => {
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            TokenKind::Boolean(b) => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(b)))
            }
            TokenKind::CellRef(cell_ref) => {
                self.advance();
                // Check if followed by colon indicating a range reference, e.g. A1:B10
                if matches!(self.peek(), TokenKind::Colon) {
                    self.advance(); // consume ':'
                    let next_tok = self.current_token().clone();
                    if let TokenKind::CellRef(mut end_ref) = next_tok.kind {
                        self.advance();
                        // If end cell does not specify a sheet, inherit start cell's sheet
                        if end_ref.sheet.is_none() && cell_ref.sheet.is_some() {
                            end_ref.sheet = cell_ref.sheet.clone();
                        }
                        let range_ref = RangeRef::new(cell_ref, end_ref);
                        Ok(Expression::Range(range_ref))
                    } else {
                        Err(FormulaError::InvalidRange(format!(
                            "{}:{}",
                            cell_ref, next_tok.kind
                        )))
                    }
                } else {
                    Ok(Expression::Cell(cell_ref))
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Ident followed by '(' is a function call
                if matches!(self.peek(), TokenKind::LParen) {
                    self.advance(); // consume '('
                    let mut args = Vec::new();

                    if !matches!(self.peek(), TokenKind::RParen) {
                        loop {
                            // Support empty argument, e.g. IF(A1>0, , 10)
                            if matches!(self.peek(), TokenKind::Comma)
                                || matches!(self.peek(), TokenKind::RParen)
                            {
                                args.push(Expression::Literal(Literal::Number(0.0)));
                            } else {
                                let arg = self.parse_comparison()?;
                                args.push(arg);
                            }

                            if matches!(self.peek(), TokenKind::Comma) {
                                self.advance(); // consume ','
                            } else {
                                break;
                            }
                        }
                    }

                    self.expect(&TokenKind::RParen)?;
                    Ok(Expression::FunctionCall { name, args })
                } else {
                    // Plain identifier without parens (could be named range or unknown variable)
                    // We treat it as function call with 0 args or identifier expression
                    Ok(Expression::FunctionCall {
                        name,
                        args: Vec::new(),
                    })
                }
            }
            TokenKind::LParen => {
                self.advance(); // consume '('
                let expr = self.parse_comparison()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::Eof => Err(FormulaError::UnexpectedEof),
            _ => Err(FormulaError::UnexpectedToken {
                token: tok.kind.to_string(),
                pos: tok.pos,
            }),
        }
    }
}

impl std::str::FromStr for Parser {
    type Err = FormulaError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        Ok(Self::new(tokens))
    }
}

/// Helper function to parse a formula string into an AST `Expression`.
pub fn parse_formula(input: &str) -> Result<Expression, FormulaError> {
    let mut parser = Parser::from_str(input)?;
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arithmetic() {
        let expr = parse_formula("=1 + 2 * 3").unwrap();
        // 1 + (2 * 3)
        if let Expression::BinaryOp { op, left, right } = expr {
            assert_eq!(op, BinaryOp::Add);
            assert_eq!(*left, Expression::Literal(Literal::Number(1.0)));
            if let Expression::BinaryOp {
                op: mul_op,
                left: m_l,
                right: m_r,
            } = *right
            {
                assert_eq!(mul_op, BinaryOp::Mul);
                assert_eq!(*m_l, Expression::Literal(Literal::Number(2.0)));
                assert_eq!(*m_r, Expression::Literal(Literal::Number(3.0)));
            } else {
                panic!("Expected Mul");
            }
        } else {
            panic!("Expected Add");
        }
    }

    #[test]
    fn test_parse_parentheses_and_power() {
        let expr = parse_formula("=(2 + 3) ^ 2").unwrap();
        if let Expression::BinaryOp { op, left, right } = expr {
            assert_eq!(op, BinaryOp::Power);
            assert_eq!(*right, Expression::Literal(Literal::Number(2.0)));
            assert!(matches!(
                *left,
                Expression::BinaryOp {
                    op: BinaryOp::Add,
                    ..
                }
            ));
        } else {
            panic!("Expected Power");
        }
    }

    #[test]
    fn test_parse_function_with_range() {
        let expr = parse_formula("=SUM(A1:B10, 42)").unwrap();
        if let Expression::FunctionCall { name, args } = expr {
            assert_eq!(name, "SUM");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], Expression::Range(_)));
            assert_eq!(args[1], Expression::Literal(Literal::Number(42.0)));
        } else {
            panic!("Expected FunctionCall");
        }
    }

    #[test]
    fn test_parse_cross_sheet_range() {
        let expr = parse_formula("=AVERAGE('Monthly Sales'!$A$1:$C$10)").unwrap();
        if let Expression::FunctionCall { name, args } = expr {
            assert_eq!(name, "AVERAGE");
            assert_eq!(args.len(), 1);
            if let Expression::Range(range) = &args[0] {
                assert_eq!(range.sheet, Some("Monthly Sales".to_string()));
                assert!(range.start.col_absolute);
                assert!(range.start.row_absolute);
                assert_eq!(range.start.coord.col, 1);
                assert_eq!(range.start.coord.row, 1);
                assert_eq!(range.end.coord.col, 3);
                assert_eq!(range.end.coord.row, 10);
            } else {
                panic!("Expected Range");
            }
        } else {
            panic!("Expected FunctionCall");
        }
    }

    #[test]
    fn test_parse_comparisons_and_concat() {
        let expr = parse_formula(r#"=IF(A1 > 10, "High: " & A1, "Low")"#).unwrap();
        if let Expression::FunctionCall { name, args } = expr {
            assert_eq!(name, "IF");
            assert_eq!(args.len(), 3);
            assert!(matches!(
                &args[0],
                Expression::BinaryOp {
                    op: BinaryOp::Gt,
                    ..
                }
            ));
            assert!(matches!(
                &args[1],
                Expression::BinaryOp {
                    op: BinaryOp::Concat,
                    ..
                }
            ));
            assert_eq!(
                args[2],
                Expression::Literal(Literal::String("Low".to_string()))
            );
        } else {
            panic!("Expected IF function call");
        }
    }

    #[test]
    fn test_extract_dependencies() {
        let expr = parse_formula("=SUM(A1:B2) + C1 + 'Sheet2'!D4").unwrap();
        let coords = expr.extract_all_cell_coords();
        // A1, A2, B1, B2, C1, Sheet2!D4 -> 6 coords
        assert_eq!(coords.len(), 6);
        assert!(coords.contains(&(None, exel_core::CellCoord::must_new(1, 1))));
        assert!(coords.contains(&(None, exel_core::CellCoord::must_new(2, 1))));
        assert!(coords.contains(&(None, exel_core::CellCoord::must_new(1, 2))));
        assert!(coords.contains(&(None, exel_core::CellCoord::must_new(2, 2))));
        assert!(coords.contains(&(None, exel_core::CellCoord::must_new(1, 3))));
        assert!(coords.contains(&(
            Some("Sheet2".to_string()),
            exel_core::CellCoord::must_new(4, 4)
        )));
    }
}
