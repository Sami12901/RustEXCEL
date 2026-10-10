use crate::context::CalcContext;
use exel_cell::{CellError, CellValue};
use exel_formula::Expression;

/// Trait defining the pluggable interface for formula calculation engines.
pub trait EngineAdapter: Send + Sync {
    /// Evaluates an AST expression in the given calculation context.
    fn evaluate(&self, ctx: &CalcContext, expr: &Expression) -> Result<CellValue, CellError>;

    /// Evaluates a raw formula string in the given calculation context.
    fn evaluate_formula(&self, ctx: &CalcContext, formula: &str) -> Result<CellValue, CellError>;
}
