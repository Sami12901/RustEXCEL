use crate::ast::CellRef;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),

    // Identifier or Function Name
    Ident(String),

    // Cell Reference (e.g. A1, $B$2, Sheet1!A1)
    CellRef(CellRef),

    // Operators
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Caret,     // ^
    Ampersand, // &
    Percent,   // %

    // Comparisons
    Equal,        // =
    NotEqual,     // <>
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    // Delimiters
    LParen, // (
    RParen, // )
    Comma,  // ,
    Colon,  // :
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Number(n) => write!(f, "{}", n),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            TokenKind::Ident(id) => write!(f, "{}", id),
            TokenKind::CellRef(c) => write!(f, "{}", c),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Equal => write!(f, "="),
            TokenKind::NotEqual => write!(f, "<>"),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Eof => write!(f, "<EOF>"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}

impl Token {
    pub fn new(kind: TokenKind, pos: usize) -> Self {
        Self { kind, pos }
    }
}
