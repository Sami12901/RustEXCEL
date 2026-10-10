use crate::ast::CellRef;
use crate::error::FormulaError;
use crate::token::{Token, TokenKind};
use exel_core::{name_to_col, CellCoord, MAX_COLS, MAX_ROWS};

/// Lexer that tokenizes a spreadsheet formula string.
pub struct Lexer<'a> {
    input: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let trimmed = input.trim();
        // Skip leading '=' if present
        let formula = if let Some(stripped) = trimmed.strip_prefix('=') {
            stripped.trim_start()
        } else {
            trimmed
        };

        let chars: Vec<(usize, char)> = formula.char_indices().collect();
        Self {
            input: formula,
            chars,
            cursor: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.cursor).map(|&(_, ch)| ch)
    }

    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.chars.get(self.cursor + n).map(|&(_, ch)| ch)
    }

    fn current_pos(&self) -> usize {
        self.chars
            .get(self.cursor)
            .map(|&(pos, _)| pos)
            .unwrap_or_else(|| self.input.len())
    }

    fn advance(&mut self) -> Option<char> {
        if self.cursor < self.chars.len() {
            let ch = self.chars[self.cursor].1;
            self.cursor += 1;
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, FormulaError> {
        let mut tokens = Vec::new();

        self.skip_whitespace();
        if self.cursor >= self.chars.len() {
            return Ok(vec![Token::new(TokenKind::Eof, 0)]);
        }

        while self.cursor < self.chars.len() {
            self.skip_whitespace();
            if self.cursor >= self.chars.len() {
                break;
            }

            let start_pos = self.current_pos();
            let ch = self.peek().unwrap();

            match ch {
                // String literal
                '"' => {
                    let s = self.lex_string(start_pos)?;
                    tokens.push(Token::new(TokenKind::String(s), start_pos));
                }

                // Quoted sheet reference, e.g. 'Sheet 1'!A1
                '\'' => {
                    let cell_ref = self.lex_quoted_sheet_cell(start_pos)?;
                    tokens.push(Token::new(TokenKind::CellRef(cell_ref), start_pos));
                }

                // Absolute cell reference starting with '$', e.g. $A$1 or $A1
                '$' => {
                    let cell_ref = self.lex_absolute_cell(start_pos)?;
                    tokens.push(Token::new(TokenKind::CellRef(cell_ref), start_pos));
                }

                // Numbers: digits or '.' followed by digit
                '0'..='9' => {
                    let num = self.lex_number(start_pos)?;
                    tokens.push(Token::new(TokenKind::Number(num), start_pos));
                }
                '.' if self.peek_ahead(1).is_some_and(|c| c.is_ascii_digit()) => {
                    let num = self.lex_number(start_pos)?;
                    tokens.push(Token::new(TokenKind::Number(num), start_pos));
                }

                // Identifier, unquoted sheet cell, or cell ref
                'a'..='z' | 'A'..='Z' | '_' => {
                    let token = self.lex_ident_or_cell(start_pos)?;
                    tokens.push(token);
                }

                // Operators and punctuation
                '+' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Plus, start_pos));
                }
                '-' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Minus, start_pos));
                }
                '*' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Star, start_pos));
                }
                '/' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Slash, start_pos));
                }
                '^' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Caret, start_pos));
                }
                '&' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Ampersand, start_pos));
                }
                '%' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Percent, start_pos));
                }
                '=' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Equal, start_pos));
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::NotEqual, start_pos));
                    } else if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::LessEqual, start_pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Less, start_pos));
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::GreaterEqual, start_pos));
                    } else {
                        tokens.push(Token::new(TokenKind::Greater, start_pos));
                    }
                }
                '(' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::LParen, start_pos));
                }
                ')' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::RParen, start_pos));
                }
                ',' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Comma, start_pos));
                }
                ':' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Colon, start_pos));
                }

                _ => {
                    return Err(FormulaError::SyntaxError {
                        message: format!("Unexpected character '{}'", ch),
                        pos: start_pos,
                    });
                }
            }
        }

        tokens.push(Token::new(TokenKind::Eof, self.current_pos()));
        Ok(tokens)
    }

    fn lex_string(&mut self, start_pos: usize) -> Result<String, FormulaError> {
        self.advance(); // consume opening quote '"'
        let mut result = String::new();

        while let Some(ch) = self.advance() {
            if ch == '"' {
                if self.peek() == Some('"') {
                    // Escaped double quote
                    self.advance();
                    result.push('"');
                } else {
                    // End of string
                    return Ok(result);
                }
            } else {
                result.push(ch);
            }
        }

        Err(FormulaError::UnclosedString(start_pos))
    }

    fn lex_number(&mut self, start_pos: usize) -> Result<f64, FormulaError> {
        let mut s = String::new();
        let mut has_dot = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Optional scientific notation: e.g. 1e5, 1.2e-3
        if let Some(ch) = self.peek() {
            if ch == 'e' || ch == 'E' {
                s.push(ch);
                self.advance();
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        s.push(sign);
                        self.advance();
                    }
                }
                while let Some(digit) = self.peek() {
                    if digit.is_ascii_digit() {
                        s.push(digit);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        s.parse::<f64>()
            .map_err(|_| FormulaError::InvalidNumber(s, start_pos))
    }

    fn lex_quoted_sheet_cell(&mut self, start_pos: usize) -> Result<CellRef, FormulaError> {
        self.advance(); // consume opening single quote '\''
        let mut sheet_name = String::new();

        let mut closed = false;
        while let Some(ch) = self.advance() {
            if ch == '\'' {
                if self.peek() == Some('\'') {
                    self.advance();
                    sheet_name.push('\'');
                } else {
                    closed = true;
                    break;
                }
            } else {
                sheet_name.push(ch);
            }
        }

        if !closed {
            return Err(FormulaError::SyntaxError {
                message: "Unclosed single quote in sheet name".to_string(),
                pos: start_pos,
            });
        }

        if self.peek() != Some('!') {
            return Err(FormulaError::SyntaxError {
                message: "Expected '!' after sheet name".to_string(),
                pos: self.current_pos(),
            });
        }
        self.advance(); // consume '!'

        // Now read the cell reference
        let _cell_start = self.current_pos();
        let mut cell_str = String::new();
        while let Some(ch) = self.peek() {
            if ch == '$' || ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                cell_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if let Some((coord, col_abs, row_abs)) = parse_cell_ref_str(&cell_str) {
            Ok(CellRef {
                coord,
                col_absolute: col_abs,
                row_absolute: row_abs,
                sheet: Some(sheet_name),
            })
        } else {
            Err(FormulaError::InvalidCellReference(format!(
                "'{}'!{}",
                sheet_name, cell_str
            )))
        }
    }

    fn lex_absolute_cell(&mut self, _start_pos: usize) -> Result<CellRef, FormulaError> {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch == '$' || ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if let Some((coord, col_abs, row_abs)) = parse_cell_ref_str(&s) {
            Ok(CellRef {
                coord,
                col_absolute: col_abs,
                row_absolute: row_abs,
                sheet: None,
            })
        } else {
            Err(FormulaError::InvalidCellReference(s))
        }
    }

    fn lex_ident_or_cell(&mut self, start_pos: usize) -> Result<Token, FormulaError> {
        let mut s = String::new();

        // Read initial identifier characters (including possible '$' for mixed refs like A$1)
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check if followed by '!' indicating an unquoted sheet name
        if self.peek() == Some('!') {
            self.advance(); // consume '!'
            let sheet_name = s;
            let mut cell_str = String::new();
            while let Some(ch) = self.peek() {
                if ch == '$' || ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                    cell_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }

            if let Some((coord, col_abs, row_abs)) = parse_cell_ref_str(&cell_str) {
                return Ok(Token::new(
                    TokenKind::CellRef(CellRef {
                        coord,
                        col_absolute: col_abs,
                        row_absolute: row_abs,
                        sheet: Some(sheet_name),
                    }),
                    start_pos,
                ));
            } else {
                return Err(FormulaError::InvalidCellReference(format!(
                    "{}!{}",
                    sheet_name, cell_str
                )));
            }
        }

        // Check if it's a valid cell reference (e.g. A1, A$1, XFD1048576)
        if let Some((coord, col_abs, row_abs)) = parse_cell_ref_str(&s) {
            return Ok(Token::new(
                TokenKind::CellRef(CellRef {
                    coord,
                    col_absolute: col_abs,
                    row_absolute: row_abs,
                    sheet: None,
                }),
                start_pos,
            ));
        }

        // Check for booleans TRUE and FALSE
        if s.eq_ignore_ascii_case("TRUE") {
            return Ok(Token::new(TokenKind::Boolean(true), start_pos));
        }
        if s.eq_ignore_ascii_case("FALSE") {
            return Ok(Token::new(TokenKind::Boolean(false), start_pos));
        }

        // Otherwise it is an identifier or function name
        Ok(Token::new(
            TokenKind::Ident(s.to_ascii_uppercase()),
            start_pos,
        ))
    }
}

/// Parses a cell reference string like "A1", "$A$1", "A$1", "$A1"
/// Returns `Some((coord, col_absolute, row_absolute))` on success.
pub fn parse_cell_ref_str(s: &str) -> Option<(CellCoord, bool, bool)> {
    if s.is_empty() {
        return None;
    }

    let mut chars = s.chars().peekable();
    let col_abs = if chars.peek() == Some(&'$') {
        chars.next();
        true
    } else {
        false
    };

    let mut col_str = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_alphabetic() {
            col_str.push(ch.to_ascii_uppercase());
            chars.next();
        } else {
            break;
        }
    }

    if col_str.is_empty() {
        return None;
    }

    let row_abs = if chars.peek() == Some(&'$') {
        chars.next();
        true
    } else {
        false
    };

    let mut row_str = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            row_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    // Must have digits and no extra trailing characters
    if row_str.is_empty() || chars.next().is_some() {
        return None;
    }

    let col = name_to_col(&col_str).ok()?;
    let row: u32 = row_str.parse().ok()?;

    if row == 0 || row > MAX_ROWS || col == 0 || col > MAX_COLS {
        return None;
    }

    Some((CellCoord::must_new(row, col), col_abs, row_abs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_basic() {
        let mut lexer = Lexer::new("=A1 + 10");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0].kind, TokenKind::CellRef(_)));
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::Number(10.0));
        assert_eq!(tokens[3].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_absolute_and_sheet_refs() {
        let mut lexer = Lexer::new("='Sheet 1'!$A$1 + Sheet2!B$2");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4);

        if let TokenKind::CellRef(ref c1) = tokens[0].kind {
            assert_eq!(c1.sheet, Some("Sheet 1".to_string()));
            assert!(c1.col_absolute);
            assert!(c1.row_absolute);
            assert_eq!(c1.coord.col, 1);
            assert_eq!(c1.coord.row, 1);
        } else {
            panic!("Expected CellRef");
        }

        if let TokenKind::CellRef(ref c2) = tokens[2].kind {
            assert_eq!(c2.sheet, Some("Sheet2".to_string()));
            assert!(!c2.col_absolute);
            assert!(c2.row_absolute);
            assert_eq!(c2.coord.col, 2);
            assert_eq!(c2.coord.row, 2);
        } else {
            panic!("Expected CellRef");
        }
    }

    #[test]
    fn test_lex_string_with_escaped_quotes() {
        let mut lexer = Lexer::new(r#"="Hello ""World""" & 42"#);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(
            tokens[0].kind,
            TokenKind::String("Hello \"World\"".to_string())
        );
        assert_eq!(tokens[1].kind, TokenKind::Ampersand);
        assert_eq!(tokens[2].kind, TokenKind::Number(42.0));
    }
}
