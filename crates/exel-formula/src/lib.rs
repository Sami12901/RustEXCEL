//! Excel formula parser and Abstract Syntax Tree (AST) definitions.
//!
//! Provides a recursive descent parser for spreadsheet formulas supporting A1 notation,
//! absolute references ($A$1), cross-sheet references ('Sheet1'!A1), rectangular range references (A1:B10),
//! binary and unary operators, literals, and Excel function calls.

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod token;

pub use ast::{BinaryOp, CellRef, Expression, Literal, RangeRef, UnaryOp};
pub use error::FormulaError;
pub use lexer::Lexer;
pub use parser::{parse_formula, Parser};
pub use token::{Token, TokenKind};
