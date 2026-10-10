use exel_core::CellCoord;
use exel_dependency_graph::{CellNode, DependencyGraph};

fn node(sheet: Option<&str>, row: u32, col: u32) -> CellNode {
    let coord = CellCoord::must_new(row, col);
    match sheet {
        Some(s) => CellNode::with_sheet(s, coord),
        None => CellNode::new(coord),
    }
}

#[test]
fn test_deep_chain_topological_sort() {
    let mut graph = DependencyGraph::new();
    let depth = 50;

    for i in 1..depth {
        let parent = node(None, i, 1);
        let child = node(None, i + 1, 1);
        graph.set_dependencies(child, vec![parent]);
    }

    assert_eq!(graph.node_count(), depth as usize);
    let order = graph.topological_sort().unwrap();
    assert_eq!(order.len(), depth as usize);

    for i in 1..depth {
        let parent = node(None, i, 1);
        let child = node(None, i + 1, 1);
        let p_idx = order.iter().position(|n| n == &parent).unwrap();
        let c_idx = order.iter().position(|n| n == &child).unwrap();
        assert!(p_idx < c_idx);
    }
}

#[test]
fn test_multiple_independent_cycles() {
    let mut graph = DependencyGraph::new();

    // Cycle 1: A1 <-> B1
    let a1 = node(None, 1, 1);
    let b1 = node(None, 1, 2);
    graph.set_dependencies(a1.clone(), vec![b1.clone()]);
    graph.set_dependencies(b1.clone(), vec![a1.clone()]);

    // Cycle 2: C1 -> D1 -> E1 -> C1
    let c1 = node(None, 1, 3);
    let d1 = node(None, 1, 4);
    let e1 = node(None, 1, 5);
    graph.set_dependencies(c1.clone(), vec![e1.clone()]);
    graph.set_dependencies(d1.clone(), vec![c1.clone()]);
    graph.set_dependencies(e1.clone(), vec![d1.clone()]);

    // Non-cycle: X1 -> Y1
    let x1 = node(None, 2, 1);
    let y1 = node(None, 2, 2);
    graph.set_dependencies(y1.clone(), vec![x1.clone()]);

    let cycles = graph.find_cycles();
    assert_eq!(cycles.len(), 2);

    let (valid_order, circular) = graph.topological_sort_partitioned();
    assert_eq!(circular.len(), 5);
    assert!(circular.contains(&a1));
    assert!(circular.contains(&b1));
    assert!(circular.contains(&c1));
    assert!(circular.contains(&d1));
    assert!(circular.contains(&e1));

    assert!(valid_order.contains(&x1));
    assert!(valid_order.contains(&y1));
}

#[test]
fn test_break_cycle_and_recover() {
    let mut graph = DependencyGraph::new();
    let a1 = node(None, 1, 1);
    let b1 = node(None, 1, 2);

    // Form cycle
    graph.set_dependencies(a1.clone(), vec![b1.clone()]);
    graph.set_dependencies(b1.clone(), vec![a1.clone()]);
    assert!(graph.topological_sort().is_err());

    // Break cycle by making B1 depend on nothing (static value)
    graph.clear_dependencies(&b1);
    assert!(!graph.is_circular(&a1));
    assert!(!graph.is_circular(&b1));

    let order = graph.topological_sort().unwrap();
    assert!(order.contains(&a1));
    assert!(order.contains(&b1));
}

#[test]
fn test_cross_sheet_subgraph_recalc() {
    let mut graph = DependencyGraph::new();
    let s1_a1 = node(Some("Input"), 1, 1);
    let s2_b1 = node(Some("Calc"), 1, 2);
    let s3_c1 = node(Some("Summary"), 1, 3);

    graph.set_dependencies(s2_b1.clone(), vec![s1_a1.clone()]);
    graph.set_dependencies(s3_c1.clone(), vec![s2_b1.clone()]);

    let affected = graph.get_affected_nodes(&[s1_a1.clone()]);
    assert_eq!(affected.len(), 3);

    let recalc = graph.ordered_recalc_subgraph(&[s1_a1.clone()]).unwrap();
    assert_eq!(recalc, vec![s1_a1, s2_b1, s3_c1]);
}
