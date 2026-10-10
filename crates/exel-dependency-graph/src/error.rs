use crate::CellNode;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DependencyError {
    #[error("Circular dependency detected involving cells: {0}")]
    CircularDependency(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

impl DependencyError {
    pub fn circular_from_nodes(nodes: &[CellNode]) -> Self {
        let node_names: Vec<String> = nodes.iter().map(|n| n.to_string()).collect();
        DependencyError::CircularDependency(node_names.join(" -> "))
    }
}
