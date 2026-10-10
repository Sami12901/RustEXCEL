//! Excel formula calculation engine and standard function library.
//!
//! Provides the core formula evaluation engine, `EngineAdapter` trait, function registry,
//! and standard Excel mathematical, logical, statistical, text, and lookup functions.
//! Integrates with `exel-dependency-graph` to maintain topological recalculation and cycle isolation.

pub mod adapter;
pub mod context;
pub mod engine;
pub mod error;
pub mod evaluator;
pub mod functions;

pub use adapter::EngineAdapter;
pub use context::CalcContext;
pub use engine::{CalcEngine, RecalcResult};
pub use error::CalcError;
pub use evaluator::evaluate_expression;
pub use functions::FunctionRegistry;
