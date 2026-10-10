//! Excel cell dependency graph engine.
//!
//! Provides directed acyclic graph tracking for cell coordinates, topological sorting,
//! Tarjan-based cycle detection with #CIRCULAR! isolation, and dirty-flagged incremental recalculation.

pub mod error;
pub mod graph;
pub mod node;

pub use error::DependencyError;
pub use graph::DependencyGraph;
pub use node::CellNode;
