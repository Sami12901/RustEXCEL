use exel_core::{CellCoord, CellRange};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Binary operators supported in spreadsheet formulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryOp {
    /// Addition: `+`
    Add,
    /// Subtraction: `-`
    Sub,
    /// Multiplication: `*`
    Mul,
    /// Division: `/`
    Div,
    /// Exponentiation: `^`
    Power,
    /// String concatenation: `&`
    Concat,
    /// Equal comparison: `=`
    Eq,
    /// Not equal comparison: `<>`
    NotEq,
    /// Less than: `<`
    Lt,
    /// Less than or equal: `<=`
    Lte,
    /// Greater than: `>`
    Gt,
    /// Greater than or equal: `>=`
    Gte,
}

impl BinaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Power => "^",
            BinaryOp::Concat => "&",
            BinaryOp::Eq => "=",
            BinaryOp::NotEq => "<>",
            BinaryOp::Lt => "<",
            BinaryOp::Lte => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Gte => ">=",
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unary operators supported in spreadsheet formulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnaryOp {
    /// Unary plus: `+`
    Plus,
    /// Unary minus / negation: `-`
    Minus,
}

impl UnaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnaryOp::Plus => "+",
            UnaryOp::Minus => "-",
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Literal values in formulas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s.replace('"', "\"\"")),
            Literal::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
        }
    }
}

/// Reference to a single cell, with optional sheet qualifier and absolute flags.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellRef {
    pub coord: CellCoord,
    pub col_absolute: bool,
    pub row_absolute: bool,
    pub sheet: Option<String>,
}

impl CellRef {
    pub fn new(coord: CellCoord) -> Self {
        Self {
            coord,
            col_absolute: false,
            row_absolute: false,
            sheet: None,
        }
    }

    pub fn with_sheet<S: Into<String>>(coord: CellCoord, sheet: S) -> Self {
        Self {
            coord,
            col_absolute: false,
            row_absolute: false,
            sheet: Some(sheet.into()),
        }
    }

    pub fn to_a1(&self) -> String {
        let col_prefix = if self.col_absolute { "$" } else { "" };
        let row_prefix = if self.row_absolute { "$" } else { "" };
        let cell_str = format!(
            "{}{}{}{}",
            col_prefix,
            exel_core::col_to_name(self.coord.col),
            row_prefix,
            self.coord.row
        );

        match &self.sheet {
            Some(sheet) => {
                if sheet.contains(' ') || sheet.contains('\'') {
                    format!("'{}'!{}", sheet.replace('\'', "''"), cell_str)
                } else {
                    format!("{}!{}", sheet, cell_str)
                }
            }
            None => cell_str,
        }
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

/// Reference to a rectangular range of cells.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RangeRef {
    pub start: CellRef,
    pub end: CellRef,
    pub sheet: Option<String>,
}

impl RangeRef {
    pub fn new(start: CellRef, end: CellRef) -> Self {
        let sheet = start.sheet.clone().or_else(|| end.sheet.clone());
        Self { start, end, sheet }
    }

    pub fn to_cell_range(&self) -> CellRange {
        CellRange::new(self.start.coord, self.end.coord)
    }

    pub fn to_a1(&self) -> String {
        let range_part = format!("{}:{}", self.start.to_a1_no_sheet(), self.end.to_a1_no_sheet());
        match &self.sheet {
            Some(sheet) => {
                if sheet.contains(' ') || sheet.contains('\'') {
                    format!("'{}'!{}", sheet.replace('\'', "''"), range_part)
                } else {
                    format!("{}!{}", sheet, range_part)
                }
            }
            None => range_part,
        }
    }

    /// Iterates over all cell coordinates in this rectangular range.
    pub fn iter_coords(&self) -> impl Iterator<Item = CellCoord> {
        let range = self.to_cell_range();
        let min_row = range.start.row;
        let max_row = range.end.row;
        let min_col = range.start.col;
        let max_col = range.end.col;
        (min_row..=max_row).flat_map(move |r| {
            (min_col..=max_col).map(move |c| CellCoord::must_new(r, c))
        })
    }
}

impl CellRef {
    pub fn to_a1_no_sheet(&self) -> String {
        let col_prefix = if self.col_absolute { "$" } else { "" };
        let row_prefix = if self.row_absolute { "$" } else { "" };
        format!(
            "{}{}{}{}",
            col_prefix,
            exel_core::col_to_name(self.coord.col),
            row_prefix,
            self.coord.row
        )
    }
}

impl fmt::Display for RangeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

/// Abstract Syntax Tree (AST) node for a spreadsheet expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// Constant literal value (Number, String, Boolean)
    Literal(Literal),
    /// Single cell reference (e.g. A1, $B$2, Sheet1!C3)
    Cell(CellRef),
    /// Cell range reference (e.g. A1:B10, Sheet1!A1:B10)
    Range(RangeRef),
    /// Function call: `SUM(A1:B10, 10)`
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    /// Binary operation: `left + right`, `left = right`
    BinaryOp {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// Unary operation: `-A1`, `+5`
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expression>,
    },
}

impl Expression {
    /// Extracts all individual cell references directly or indirectly referenced in this expression,
    /// expanding any ranges into individual cell coordinates.
    pub fn extract_all_cell_coords(&self) -> Vec<(Option<String>, CellCoord)> {
        let mut results = Vec::new();
        self.collect_coords_internal(&mut results);
        results
    }

    fn collect_coords_internal(&self, acc: &mut Vec<(Option<String>, CellCoord)>) {
        match self {
            Expression::Literal(_) => {}
            Expression::Cell(cell_ref) => {
                acc.push((cell_ref.sheet.clone(), cell_ref.coord));
            }
            Expression::Range(range_ref) => {
                let sheet = range_ref.sheet.clone();
                for coord in range_ref.iter_coords() {
                    acc.push((sheet.clone(), coord));
                }
            }
            Expression::FunctionCall { args, .. } => {
                for arg in args {
                    arg.collect_coords_internal(acc);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                left.collect_coords_internal(acc);
                right.collect_coords_internal(acc);
            }
            Expression::UnaryOp { expr, .. } => {
                expr.collect_coords_internal(acc);
            }
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Literal(lit) => write!(f, "{}", lit),
            Expression::Cell(cell) => write!(f, "{}", cell),
            Expression::Range(range) => write!(f, "{}", range),
            Expression::FunctionCall { name, args } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expression::BinaryOp { op, left, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
            Expression::UnaryOp { op, expr } => {
                write!(f, "{}{}", op, expr)
            }
        }
    }
}
