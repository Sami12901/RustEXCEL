use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FormulaError {
    #[error("Empty formula")]
    EmptyFormula,

    #[error("Syntax error at position {pos}: {message}")]
    SyntaxError { message: String, pos: usize },

    #[error("Unexpected token '{token}' at position {pos}")]
    UnexpectedToken { token: String, pos: usize },

    #[error("Unexpected end of formula")]
    UnexpectedEof,

    #[error("Unclosed parenthesis in formula")]
    UnclosedParenthesis,

    #[error("Unclosed string literal starting at position {0}")]
    UnclosedString(usize),

    #[error("Invalid cell reference '{0}'")]
    InvalidCellReference(String),

    #[error("Invalid range reference '{0}'")]
    InvalidRange(String),

    #[error("Invalid number format '{0}' at position {1}")]
    InvalidNumber(String, usize),
}
