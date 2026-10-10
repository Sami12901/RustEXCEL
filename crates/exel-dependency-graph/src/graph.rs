use crate::error::DependencyError;
use crate::node::CellNode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A directed graph representing spreadsheet cell dependencies.
///
/// In this graph:
/// - A node is a `CellNode`.
/// - A directed edge from `A` to `B` (`A -> B`) means `A` is a *precedent* of `B`,
///   meaning `B` depends on `A`, and `A` must be evaluated before `B`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Maps a precedent cell to all cells that directly depend on it. (A -> {B})
    dependents: HashMap<CellNode, HashSet<CellNode>>,
    /// Maps a dependent cell to all cells it directly depends on. (B -> {A})
    precedents: HashMap<CellNode, HashSet<CellNode>>,
    /// Set of cells flagged as dirty for incremental recalculation.
    dirty_nodes: HashSet<CellNode>,
}

impl DependencyGraph {
    /// Creates a new, empty dependency graph.
    pub fn new() -> Self {
        Self {
            dependents: HashMap::new(),
            precedents: HashMap::new(),
            dirty_nodes: HashSet::new(),
        }
    }

    /// Checks if a node exists in the graph.
    pub fn contains_node(&self, node: &CellNode) -> bool {
        self.precedents.contains_key(node) || self.dependents.contains_key(node)
    }

    /// Returns the number of distinct nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.all_nodes().len()
    }

    /// Returns the number of directed edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.dependents.values().map(|set| set.len()).sum()
    }

    /// Returns all distinct nodes present in the graph.
    pub fn all_nodes(&self) -> HashSet<CellNode> {
        let mut nodes = HashSet::new();
        for k in self.precedents.keys() {
            nodes.insert(k.clone());
        }
        for k in self.dependents.keys() {
            nodes.insert(k.clone());
        }
        for set in self.precedents.values() {
            nodes.extend(set.iter().cloned());
        }
        for set in self.dependents.values() {
            nodes.extend(set.iter().cloned());
        }
        nodes
    }

    /// Sets the dependencies for a given dependent node.
    ///
    /// Replaces any existing precedents of `node` with the new set of `new_precedents`.
    /// Also marks `node` and its downstream dependents as dirty.
    pub fn set_dependencies<I>(&mut self, node: CellNode, new_precedents: I)
    where
        I: IntoIterator<Item = CellNode>,
    {
        // 1. Remove old precedents for this node
        if let Some(old_precedents) = self.precedents.remove(&node) {
            for prec in old_precedents {
                if let Some(dep_set) = self.dependents.get_mut(&prec) {
                    dep_set.remove(&node);
                    if dep_set.is_empty() {
                        self.dependents.remove(&prec);
                    }
                }
            }
        }

        // 2. Insert new precedents
        let prec_set: HashSet<CellNode> = new_precedents.into_iter().collect();
        for prec in &prec_set {
            self.dependents
                .entry(prec.clone())
                .or_default()
                .insert(node.clone());
        }

        self.precedents.insert(node.clone(), prec_set);

        // 3. Flag node and its downstream dependents as dirty
        self.mark_dirty(&node);
    }

    /// Clears all precedents for a cell (e.g. when a formula is replaced by a static value).
    pub fn clear_dependencies(&mut self, node: &CellNode) {
        if let Some(old_precedents) = self.precedents.remove(node) {
            for prec in old_precedents {
                if let Some(dep_set) = self.dependents.get_mut(&prec) {
                    dep_set.remove(node);
                    if dep_set.is_empty() {
                        self.dependents.remove(&prec);
                    }
                }
            }
        }
        self.mark_dirty(node);
    }

    /// Completely removes a node and all its incoming and outgoing dependency edges.
    pub fn remove_node(&mut self, node: &CellNode) {
        // Remove incoming edges (where node depends on precedents)
        if let Some(old_precedents) = self.precedents.remove(node) {
            for prec in old_precedents {
                if let Some(dep_set) = self.dependents.get_mut(&prec) {
                    dep_set.remove(node);
                    if dep_set.is_empty() {
                        self.dependents.remove(&prec);
                    }
                }
            }
        }

        // Remove outgoing edges (where other nodes depend on this node)
        if let Some(old_dependents) = self.dependents.remove(node) {
            for dep in old_dependents {
                if let Some(prec_set) = self.precedents.get_mut(&dep) {
                    prec_set.remove(node);
                    if prec_set.is_empty() {
                        self.precedents.remove(&dep);
                    }
                }
            }
        }

        self.dirty_nodes.remove(node);
    }

    /// Returns direct precedents of a node (cells that this node depends on).
    pub fn get_precedents(&self, node: &CellNode) -> Option<&HashSet<CellNode>> {
        self.precedents.get(node)
    }

    /// Returns direct dependents of a node (cells that depend on this node).
    pub fn get_dependents(&self, node: &CellNode) -> Option<&HashSet<CellNode>> {
        self.dependents.get(node)
    }

    /// Marks a cell and all its transitive downstream dependents as dirty.
    pub fn mark_dirty(&mut self, node: &CellNode) {
        let mut queue = VecDeque::new();
        queue.push_back(node.clone());
        self.dirty_nodes.insert(node.clone());

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.dependents.get(&current) {
                for dep in deps {
                    if self.dirty_nodes.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
    }

    /// Marks multiple cells and all their transitive downstream dependents as dirty.
    pub fn mark_dirty_batch(&mut self, nodes: &[CellNode]) {
        for node in nodes {
            self.mark_dirty(node);
        }
    }

    /// Checks if a cell is currently marked dirty.
    pub fn is_dirty(&self, node: &CellNode) -> bool {
        self.dirty_nodes.contains(node)
    }

    /// Returns the set of all currently dirty cell nodes.
    pub fn dirty_nodes(&self) -> &HashSet<CellNode> {
        &self.dirty_nodes
    }

    /// Clears all dirty flags.
    pub fn clear_dirty(&mut self) {
        self.dirty_nodes.clear();
    }

    /// Removes dirty flag for a specific cell.
    pub fn clear_node_dirty(&mut self, node: &CellNode) {
        self.dirty_nodes.remove(node);
    }

    /// Returns all cells that are affected by changes to `changed_nodes`
    /// (the changed nodes themselves plus all transitive downstream dependents).
    pub fn get_affected_nodes(&self, changed_nodes: &[CellNode]) -> HashSet<CellNode> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        for node in changed_nodes {
            affected.insert(node.clone());
            queue.push_back(node.clone());
        }

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.dependents.get(&current) {
                for dep in deps {
                    if affected.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        affected
    }

    /// Detects all strongly connected components (SCCs) in the graph that form cycles
    /// using Tarjan's SCC algorithm.
    pub fn find_cycles(&self) -> Vec<Vec<CellNode>> {
        let all_nodes: Vec<CellNode> = self.all_nodes().into_iter().collect();
        let mut index = 0;
        let mut indices: HashMap<CellNode, usize> = HashMap::new();
        let mut lowlink: HashMap<CellNode, usize> = HashMap::new();
        let mut on_stack: HashSet<CellNode> = HashSet::new();
        let mut stack: Vec<CellNode> = Vec::new();
        let mut sccs: Vec<Vec<CellNode>> = Vec::new();

        #[allow(clippy::too_many_arguments)]
        fn strong_connect(
            v: &CellNode,
            index: &mut usize,
            indices: &mut HashMap<CellNode, usize>,
            lowlink: &mut HashMap<CellNode, usize>,
            on_stack: &mut HashSet<CellNode>,
            stack: &mut Vec<CellNode>,
            sccs: &mut Vec<Vec<CellNode>>,
            dependents: &HashMap<CellNode, HashSet<CellNode>>,
        ) {
            indices.insert(v.clone(), *index);
            lowlink.insert(v.clone(), *index);
            *index += 1;
            stack.push(v.clone());
            on_stack.insert(v.clone());

            if let Some(deps) = dependents.get(v) {
                for w in deps {
                    if !indices.contains_key(w) {
                        strong_connect(
                            w, index, indices, lowlink, on_stack, stack, sccs, dependents,
                        );
                        let w_low = lowlink[w];
                        let v_low = lowlink.get_mut(v).unwrap();
                        *v_low = (*v_low).min(w_low);
                    } else if on_stack.contains(w) {
                        let w_index = indices[w];
                        let v_low = lowlink.get_mut(v).unwrap();
                        *v_low = (*v_low).min(w_index);
                    }
                }
            }

            if lowlink[v] == indices[v] {
                let mut scc = Vec::new();
                loop {
                    let w = stack.pop().unwrap();
                    on_stack.remove(&w);
                    scc.push(w.clone());
                    if &w == v {
                        break;
                    }
                }

                // An SCC is a cycle if it has > 1 node, or a 1-node self loop
                if scc.len() > 1 {
                    sccs.push(scc);
                } else if scc.len() == 1 {
                    let node = &scc[0];
                    if let Some(deps) = dependents.get(node) {
                        if deps.contains(node) {
                            sccs.push(scc);
                        }
                    }
                }
            }
        }

        for node in &all_nodes {
            if !indices.contains_key(node) {
                strong_connect(
                    node,
                    &mut index,
                    &mut indices,
                    &mut lowlink,
                    &mut on_stack,
                    &mut stack,
                    &mut sccs,
                    &self.dependents,
                );
            }
        }

        sccs
    }

    /// Identifies all circular nodes:
    /// This includes all nodes directly participating in cycles, plus all nodes
    /// that transitively depend on any cycle.
    pub fn find_all_circular_nodes(&self) -> HashSet<CellNode> {
        let cycles = self.find_cycles();
        let mut circular = HashSet::new();

        for cycle in cycles {
            for node in cycle {
                circular.insert(node);
            }
        }

        // Taint all downstream dependents of circular nodes
        let mut queue: VecDeque<CellNode> = circular.iter().cloned().collect();
        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.dependents.get(&current) {
                for dep in deps {
                    if circular.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        circular
    }

    /// Checks if a specific node is involved in a circular dependency (directly or transitively).
    pub fn is_circular(&self, node: &CellNode) -> bool {
        self.find_all_circular_nodes().contains(node)
    }

    /// Performs a topological sort of all nodes in the graph using Kahn's algorithm.
    ///
    /// If successful, returns the nodes in evaluation order (precedents before dependents).
    /// If cycles are detected, returns an Err containing a list of circular nodes.
    pub fn topological_sort(&self) -> Result<Vec<CellNode>, DependencyError> {
        let circular_nodes = self.find_all_circular_nodes();
        if !circular_nodes.is_empty() {
            let mut list: Vec<CellNode> = circular_nodes.into_iter().collect();
            list.sort();
            return Err(DependencyError::circular_from_nodes(&list));
        }

        let all_nodes = self.all_nodes();
        let mut in_degrees: HashMap<CellNode, usize> = HashMap::new();

        for node in &all_nodes {
            let deg = self.precedents.get(node).map(|p| p.len()).unwrap_or(0);
            in_degrees.insert(node.clone(), deg);
        }

        let mut queue: VecDeque<CellNode> = in_degrees
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(n, _)| n.clone())
            .collect();

        // Sort queue initially for deterministic output
        let mut sorted_queue: Vec<CellNode> = queue.into_iter().collect();
        sorted_queue.sort();
        queue = sorted_queue.into();

        let mut order = Vec::with_capacity(all_nodes.len());

        while let Some(node) = queue.pop_front() {
            order.push(node.clone());

            if let Some(deps) = self.dependents.get(&node) {
                // Collect and sort dependents for determinism
                let mut dep_list: Vec<CellNode> = deps.iter().cloned().collect();
                dep_list.sort();

                for dep in dep_list {
                    if let Some(deg) = in_degrees.get_mut(&dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        if order.len() == all_nodes.len() {
            Ok(order)
        } else {
            // Some nodes were not reachable due to an undetected cycle
            let unvisited: Vec<CellNode> = all_nodes
                .into_iter()
                .filter(|n| !order.contains(n))
                .collect();
            Err(DependencyError::circular_from_nodes(&unvisited))
        }
    }

    /// Performs a topological sort of valid nodes while isolating and returning circular nodes.
    ///
    /// Returns `(valid_evaluation_order, circular_nodes)`.
    pub fn topological_sort_partitioned(&self) -> (Vec<CellNode>, HashSet<CellNode>) {
        let circular = self.find_all_circular_nodes();
        let all_nodes = self.all_nodes();

        // Valid nodes are those that are NOT circular
        let valid_nodes: HashSet<CellNode> = all_nodes
            .into_iter()
            .filter(|n| !circular.contains(n))
            .collect();

        let mut in_degrees: HashMap<CellNode, usize> = HashMap::new();
        for node in &valid_nodes {
            let deg = self
                .precedents
                .get(node)
                .map(|p| p.iter().filter(|pn| valid_nodes.contains(pn)).count())
                .unwrap_or(0);
            in_degrees.insert(node.clone(), deg);
        }

        let mut queue: VecDeque<CellNode> = in_degrees
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(n, _)| n.clone())
            .collect();

        let mut sorted_queue: Vec<CellNode> = queue.into_iter().collect();
        sorted_queue.sort();
        queue = sorted_queue.into();

        let mut order = Vec::with_capacity(valid_nodes.len());
        while let Some(node) = queue.pop_front() {
            order.push(node.clone());

            if let Some(deps) = self.dependents.get(&node) {
                let mut dep_list: Vec<CellNode> = deps
                    .iter()
                    .filter(|d| valid_nodes.contains(d))
                    .cloned()
                    .collect();
                dep_list.sort();

                for dep in dep_list {
                    if let Some(deg) = in_degrees.get_mut(&dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        (order, circular)
    }

    /// Computes an ordered incremental recalculation plan for cells affected by changes to `changed_nodes`.
    ///
    /// Returns `Ok(evaluation_sequence)` of cells that need recalculation in proper topological order.
    /// If any circular dependencies exist within the affected subgraph, returns `Err(circular_cells)`.
    pub fn ordered_recalc_subgraph(
        &self,
        changed_nodes: &[CellNode],
    ) -> Result<Vec<CellNode>, HashSet<CellNode>> {
        let affected = self.get_affected_nodes(changed_nodes);
        if affected.is_empty() {
            return Ok(Vec::new());
        }

        // Check if any circular dependencies affect this subgraph
        let all_circular = self.find_all_circular_nodes();
        let affected_circular: HashSet<CellNode> = affected
            .iter()
            .filter(|n| all_circular.contains(n))
            .cloned()
            .collect();

        if !affected_circular.is_empty() {
            return Err(affected_circular);
        }

        // Subgraph contains only affected nodes that have formulas (precedents),
        // or we sort all affected nodes
        let mut in_degrees: HashMap<CellNode, usize> = HashMap::new();
        for node in &affected {
            let deg = self
                .precedents
                .get(node)
                .map(|p| p.iter().filter(|pn| affected.contains(pn)).count())
                .unwrap_or(0);
            in_degrees.insert(node.clone(), deg);
        }

        let mut queue: VecDeque<CellNode> = in_degrees
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(n, _)| n.clone())
            .collect();

        let mut sorted_queue: Vec<CellNode> = queue.into_iter().collect();
        sorted_queue.sort();
        queue = sorted_queue.into();

        let mut order = Vec::with_capacity(affected.len());
        while let Some(node) = queue.pop_front() {
            order.push(node.clone());

            if let Some(deps) = self.dependents.get(&node) {
                let mut dep_list: Vec<CellNode> = deps
                    .iter()
                    .filter(|d| affected.contains(d))
                    .cloned()
                    .collect();
                dep_list.sort();

                for dep in dep_list {
                    if let Some(deg) = in_degrees.get_mut(&dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_core::CellCoord;

    fn c(row: u32, col: u32) -> CellNode {
        CellNode::new(CellCoord::must_new(row, col))
    }

    #[test]
    fn test_linear_dependency() {
        let mut graph = DependencyGraph::new();
        let a1 = c(1, 1);
        let b1 = c(1, 2);
        let c1 = c(1, 3);

        // B1 = A1 + 1 (B1 depends on A1)
        graph.set_dependencies(b1.clone(), vec![a1.clone()]);
        // C1 = B1 + 1 (C1 depends on B1)
        graph.set_dependencies(c1.clone(), vec![b1.clone()]);

        let order = graph.topological_sort().unwrap();
        let a1_pos = order.iter().position(|x| x == &a1).unwrap();
        let b1_pos = order.iter().position(|x| x == &b1).unwrap();
        let c1_pos = order.iter().position(|x| x == &c1).unwrap();

        assert!(a1_pos < b1_pos);
        assert!(b1_pos < c1_pos);
    }

    #[test]
    fn test_diamond_dependency() {
        let mut graph = DependencyGraph::new();
        let a1 = c(1, 1);
        let b1 = c(1, 2);
        let c1 = c(1, 3);
        let d1 = c(1, 4);

        // B1 = A1, C1 = A1, D1 = B1 + C1
        graph.set_dependencies(b1.clone(), vec![a1.clone()]);
        graph.set_dependencies(c1.clone(), vec![a1.clone()]);
        graph.set_dependencies(d1.clone(), vec![b1.clone(), c1.clone()]);

        let order = graph.topological_sort().unwrap();
        let a1_pos = order.iter().position(|x| x == &a1).unwrap();
        let b1_pos = order.iter().position(|x| x == &b1).unwrap();
        let c1_pos = order.iter().position(|x| x == &c1).unwrap();
        let d1_pos = order.iter().position(|x| x == &d1).unwrap();

        assert!(a1_pos < b1_pos);
        assert!(a1_pos < c1_pos);
        assert!(b1_pos < d1_pos);
        assert!(c1_pos < d1_pos);
    }

    #[test]
    fn test_self_cycle() {
        let mut graph = DependencyGraph::new();
        let a1 = c(1, 1);

        // A1 = A1 + 1
        graph.set_dependencies(a1.clone(), vec![a1.clone()]);

        assert!(graph.is_circular(&a1));
        assert!(graph.topological_sort().is_err());
        let circular = graph.find_all_circular_nodes();
        assert!(circular.contains(&a1));
    }

    #[test]
    fn test_mutual_cycle() {
        let mut graph = DependencyGraph::new();
        let a1 = c(1, 1);
        let b1 = c(1, 2);

        // A1 = B1, B1 = A1
        graph.set_dependencies(a1.clone(), vec![b1.clone()]);
        graph.set_dependencies(b1.clone(), vec![a1.clone()]);

        assert!(graph.is_circular(&a1));
        assert!(graph.is_circular(&b1));
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn test_transitive_cycle_taint() {
        let mut graph = DependencyGraph::new();
        let a1 = c(1, 1);
        let b1 = c(1, 2);
        let c1 = c(1, 3);

        // A1 = B1, B1 = A1, C1 = B1 + 10
        graph.set_dependencies(a1.clone(), vec![b1.clone()]);
        graph.set_dependencies(b1.clone(), vec![a1.clone()]);
        graph.set_dependencies(c1.clone(), vec![b1.clone()]);

        let circular = graph.find_all_circular_nodes();
        assert!(circular.contains(&a1));
        assert!(circular.contains(&b1));
        assert!(circular.contains(&c1)); // C1 tainted by cycle
    }

    #[test]
    fn test_dirty_flagging_and_incremental_recalc() {
        let mut graph = DependencyGraph::new();
        let a1 = c(1, 1);
        let b1 = c(1, 2);
        let c1 = c(1, 3);
        let x1 = c(2, 1);
        let y1 = c(2, 2);

        graph.set_dependencies(b1.clone(), vec![a1.clone()]);
        graph.set_dependencies(c1.clone(), vec![b1.clone()]);
        graph.set_dependencies(y1.clone(), vec![x1.clone()]);
        graph.clear_dirty();

        // Mutate A1
        graph.mark_dirty(&a1);
        assert!(graph.is_dirty(&a1));
        assert!(graph.is_dirty(&b1));
        assert!(graph.is_dirty(&c1));
        assert!(!graph.is_dirty(&x1));
        assert!(!graph.is_dirty(&y1));

        let recalc = graph
            .ordered_recalc_subgraph(std::slice::from_ref(&a1))
            .unwrap();
        assert_eq!(recalc, vec![a1, b1, c1]);
    }

    #[test]
    fn test_update_dependencies() {
        let mut graph = DependencyGraph::new();
        let a1 = c(1, 1);
        let b1 = c(1, 2);
        let c1 = c(1, 3);

        // Initially B1 = A1
        graph.set_dependencies(b1.clone(), vec![a1.clone()]);
        assert_eq!(graph.get_precedents(&b1).unwrap().len(), 1);

        // Update B1 = C1
        graph.set_dependencies(b1.clone(), vec![c1.clone()]);
        assert_eq!(graph.get_precedents(&b1).unwrap().len(), 1);
        assert!(graph.get_precedents(&b1).unwrap().contains(&c1));
        assert!(!graph.get_precedents(&b1).unwrap().contains(&a1));
        assert!(graph.get_dependents(&a1).is_none());
        assert!(graph.get_dependents(&c1).unwrap().contains(&b1));
    }

    #[test]
    fn test_cross_sheet_dependency() {
        let mut graph = DependencyGraph::new();
        let s1_a1 = CellNode::with_sheet("Sheet1", CellCoord::must_new(1, 1));
        let s2_b1 = CellNode::with_sheet("Sheet2", CellCoord::must_new(1, 2));

        graph.set_dependencies(s2_b1.clone(), vec![s1_a1.clone()]);
        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec![s1_a1, s2_b1]);
    }
}
