use exel_cell::CellError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CalcError {
    #[error("Excel error: {0}")]
    Cell(CellError),

    #[error("Sheet not found: {0}")]
    SheetNotFound(String),

    #[error("Formula parse error: {0}")]
    ParseError(String),
}

impl From<CellError> for CalcError {
    fn from(err: CellError) -> Self {
        CalcError::Cell(err)
    }
}
