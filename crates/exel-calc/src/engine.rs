use crate::adapter::EngineAdapter;
use crate::context::CalcContext;
use crate::evaluator::evaluate_expression;
use crate::functions::FunctionRegistry;
use exel_cell::{CellError, CellState, CellValue};
use exel_core::{CellCoord, SheetId};
use exel_dependency_graph::{CellNode, DependencyGraph};
use exel_formula::{parse_formula, Expression};
use exel_workbook::Workbook;
use std::collections::HashMap;

/// Summary result of a recalculation pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecalcResult {
    /// Number of formulas successfully recomputed.
    pub recalculated_cells: usize,
    /// Cells tagged with #CIRCULAR! due to circular references.
    pub circular_cells: Vec<CellNode>,
    /// Number of formula evaluations resulting in an error.
    pub error_count: usize,
}

/// The core spreadsheet calculation engine.
///
/// Implements `EngineAdapter` and manages dependency tracking, cycle detection,
/// topological sorting, and incremental evaluation for `Workbook` documents.
pub struct CalcEngine {
    pub registry: FunctionRegistry,
    pub dep_graph: DependencyGraph,
    parsed_cache: HashMap<CellNode, Expression>,
}

impl Default for CalcEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CalcEngine {
    pub fn new() -> Self {
        Self {
            registry: FunctionRegistry::new(),
            dep_graph: DependencyGraph::new(),
            parsed_cache: HashMap::new(),
        }
    }

    /// Builds or synchronizes the dependency graph from all formulas present across all sheets in the workbook.
    pub fn rebuild_dependency_graph(&mut self, wb: &Workbook) {
        self.dep_graph = DependencyGraph::new();
        self.parsed_cache.clear();

        for sheet in &wb.sheets {
            for (coord, state) in sheet.cells.iter() {
                let node = CellNode::with_sheet(&sheet.name, *coord);
                if let Some(formula) = &state.formula {
                    if let Ok(expr) = parse_formula(formula) {
                        let raw_precedents = expr.extract_all_cell_coords();
                        let precedents: Vec<CellNode> = raw_precedents
                            .into_iter()
                            .map(|(s_opt, c)| {
                                let sheet_name = s_opt.unwrap_or_else(|| sheet.name.clone());
                                CellNode::with_sheet(sheet_name, c)
                            })
                            .collect();

                        self.dep_graph.set_dependencies(node.clone(), precedents);
                        self.parsed_cache.insert(node, expr);
                    } else {
                        self.dep_graph.clear_dependencies(&node);
                    }
                } else {
                    self.dep_graph.clear_dependencies(&node);
                }
            }
        }
    }

    /// Recalculates all formulas across the entire workbook.
    ///
    /// Detects circular dependencies, sets `#CIRCULAR!` on affected cells,
    /// and evaluates non-circular cells in topologically sorted order.
    pub fn recalculate_workbook(&mut self, wb: &mut Workbook) -> RecalcResult {
        self.rebuild_dependency_graph(wb);

        let (eval_order, circular_set) = self.dep_graph.topological_sort_partitioned();
        let mut result = RecalcResult::default();

        // 1. Mark circular cells with #CIRCULAR!
        for circ_node in &circular_set {
            if let Some(sheet_name) = &circ_node.sheet {
                if let Some(sheet) = wb.get_sheet_by_name_mut(sheet_name) {
                    if let Some(cell) = sheet.cells.get_mut(circ_node.coord) {
                        cell.value = CellValue::Error(CellError::Circular);
                    }
                }
            }
            result.circular_cells.push(circ_node.clone());
        }

        // 2. Evaluate valid cells in topological order
        for node in eval_order {
            let sheet_name = match &node.sheet {
                Some(s) => s.clone(),
                None => continue,
            };

            // Retrieve formula
            let formula_opt = wb
                .get_sheet_by_name(&sheet_name)
                .and_then(|s| s.get_cell(node.coord))
                .and_then(|c| c.formula.clone());

            if let Some(formula) = formula_opt {
                let expr = if let Some(cached) = self.parsed_cache.get(&node) {
                    cached.clone()
                } else {
                    match parse_formula(&formula) {
                        Ok(e) => e,
                        Err(_) => {
                            if let Some(sheet) = wb.get_sheet_by_name_mut(&sheet_name) {
                                if let Some(cell) = sheet.cells.get_mut(node.coord) {
                                    cell.value = CellValue::Error(CellError::Value);
                                }
                            }
                            result.error_count += 1;
                            continue;
                        }
                    }
                };

                let ctx = CalcContext::with_coord(wb, &sheet_name, node.coord);
                let computed = match self.evaluate(&ctx, &expr) {
                    Ok(val) => val,
                    Err(err) => {
                        result.error_count += 1;
                        CellValue::Error(err)
                    }
                };

                if let Some(sheet) = wb.get_sheet_by_name_mut(&sheet_name) {
                    if let Some(cell) = sheet.cells.get_mut(node.coord) {
                        cell.value = computed;
                    }
                }
                result.recalculated_cells += 1;
            }
        }

        result
    }

    /// Recalculates all formulas belonging to a specific sheet.
    pub fn recalculate_sheet(&mut self, wb: &mut Workbook, sheet_id: &SheetId) -> RecalcResult {
        let sheet_name = match wb.get_sheet(sheet_id) {
            Some(s) => s.name.clone(),
            None => return RecalcResult::default(),
        };

        // Rebuild graph to ensure consistency
        self.rebuild_dependency_graph(wb);

        let (eval_order, circular_set) = self.dep_graph.topological_sort_partitioned();
        let mut result = RecalcResult::default();

        // 1. Mark circular cells on this sheet
        for circ_node in &circular_set {
            if circ_node.sheet.as_deref() == Some(&sheet_name) {
                if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
                    if let Some(cell) = sheet.cells.get_mut(circ_node.coord) {
                        cell.value = CellValue::Error(CellError::Circular);
                    }
                }
                result.circular_cells.push(circ_node.clone());
            }
        }

        // 2. Evaluate cells belonging to this sheet (or dependencies)
        for node in eval_order {
            if node.sheet.as_deref() != Some(&sheet_name) {
                continue;
            }

            let formula_opt = wb
                .get_sheet(sheet_id)
                .and_then(|s| s.get_cell(node.coord))
                .and_then(|c| c.formula.clone());

            if let Some(formula) = formula_opt {
                let expr = if let Some(cached) = self.parsed_cache.get(&node) {
                    cached.clone()
                } else {
                    match parse_formula(&formula) {
                        Ok(e) => e,
                        Err(_) => {
                            if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
                                if let Some(cell) = sheet.cells.get_mut(node.coord) {
                                    cell.value = CellValue::Error(CellError::Value);
                                }
                            }
                            result.error_count += 1;
                            continue;
                        }
                    }
                };

                let ctx = CalcContext::with_coord(wb, &sheet_name, node.coord);
                let computed = match self.evaluate(&ctx, &expr) {
                    Ok(val) => val,
                    Err(err) => {
                        result.error_count += 1;
                        CellValue::Error(err)
                    }
                };

                if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
                    if let Some(cell) = sheet.cells.get_mut(node.coord) {
                        cell.value = computed;
                    }
                }
                result.recalculated_cells += 1;
            }
        }

        result
    }

    /// Evaluates and returns the updated value of a single cell.
    pub fn recalculate_cell(
        &mut self,
        wb: &mut Workbook,
        sheet_id: &SheetId,
        coord: CellCoord,
    ) -> CellValue {
        let sheet_name = match wb.get_sheet(sheet_id) {
            Some(s) => s.name.clone(),
            None => return CellValue::Error(CellError::Ref),
        };

        let node = CellNode::with_sheet(&sheet_name, coord);
        if self.dep_graph.is_circular(&node) {
            let err_val = CellValue::Error(CellError::Circular);
            if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
                if let Some(cell) = sheet.cells.get_mut(coord) {
                    cell.value = err_val.clone();
                }
            }
            return err_val;
        }

        let formula_opt = wb
            .get_sheet(sheet_id)
            .and_then(|s| s.get_cell(coord))
            .and_then(|c| c.formula.clone());

        if let Some(formula) = formula_opt {
            let expr = match parse_formula(&formula) {
                Ok(e) => e,
                Err(_) => {
                    let err_val = CellValue::Error(CellError::Value);
                    if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
                        if let Some(cell) = sheet.cells.get_mut(coord) {
                            cell.value = err_val.clone();
                        }
                    }
                    return err_val;
                }
            };

            let ctx = CalcContext::with_coord(wb, &sheet_name, coord);
            let computed = match self.evaluate(&ctx, &expr) {
                Ok(v) => v,
                Err(e) => CellValue::Error(e),
            };

            if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
                if let Some(cell) = sheet.cells.get_mut(coord) {
                    cell.value = computed.clone();
                }
            }

            computed
        } else {
            wb.get_sheet(sheet_id)
                .map(|s| s.get_cell_value(coord))
                .unwrap_or(CellValue::Empty)
        }
    }

    /// Updates a cell's formula and recalculates affected downstream dependents incrementally.
    pub fn update_cell_formula(
        &mut self,
        wb: &mut Workbook,
        sheet_id: &SheetId,
        coord: CellCoord,
        formula: String,
    ) -> CellValue {
        let sheet_name = match wb.get_sheet(sheet_id) {
            Some(s) => s.name.clone(),
            None => return CellValue::Error(CellError::Ref),
        };

        let node = CellNode::with_sheet(&sheet_name, coord);

        // Parse formula and update dependencies in graph
        if let Ok(expr) = parse_formula(&formula) {
            let raw_precedents = expr.extract_all_cell_coords();
            let precedents: Vec<CellNode> = raw_precedents
                .into_iter()
                .map(|(s_opt, c)| {
                    let s_name = s_opt.unwrap_or_else(|| sheet_name.clone());
                    CellNode::with_sheet(s_name, c)
                })
                .collect();

            self.dep_graph.set_dependencies(node.clone(), precedents);
            self.parsed_cache.insert(node, expr);
        } else {
            self.dep_graph.clear_dependencies(&node);
        }

        // Store formula with placeholder value
        if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
            sheet.set_cell_formula(coord, formula, CellValue::Empty);
        }

        // Recalculate workbook to update downstream cells
        self.recalculate_workbook(wb);

        wb.get_sheet(sheet_id)
            .map(|s| s.get_cell_value(coord))
            .unwrap_or(CellValue::Empty)
    }

    /// Sets a static cell value and triggers incremental recalculation of affected downstream dependents.
    pub fn update_cell_value(
        &mut self,
        wb: &mut Workbook,
        sheet_id: &SheetId,
        coord: CellCoord,
        value: CellValue,
    ) -> RecalcResult {
        let sheet_name = match wb.get_sheet(sheet_id) {
            Some(s) => s.name.clone(),
            None => return RecalcResult::default(),
        };

        let node = CellNode::with_sheet(&sheet_name, coord);
        self.dep_graph.clear_dependencies(&node);
        self.parsed_cache.remove(&node);

        if let Some(sheet) = wb.get_sheet_mut(sheet_id) {
            sheet.set_cell(coord, CellState::from_value(value));
        }

        // Mark node dirty to notify downstream dependents
        self.dep_graph.mark_dirty(&node);
        self.recalculate_workbook(wb)
    }
}

impl EngineAdapter for CalcEngine {
    fn evaluate(&self, ctx: &CalcContext, expr: &Expression) -> Result<CellValue, CellError> {
        evaluate_expression(&self.registry, ctx, expr)
    }

    fn evaluate_formula(&self, ctx: &CalcContext, formula: &str) -> Result<CellValue, CellError> {
        let expr = parse_formula(formula).map_err(|_| CellError::Value)?;
        self.evaluate(ctx, &expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::CellState;

    #[test]
    fn test_simple_arithmetic_evaluation() {
        let wb = Workbook::with_default_sheet();
        let engine = CalcEngine::new();
        let ctx = CalcContext::new(&wb, "Sheet1");

        assert_eq!(
            engine.evaluate_formula(&ctx, "=1 + 2 * 3").unwrap(),
            CellValue::Number(7.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=(2 + 3) ^ 2").unwrap(),
            CellValue::Number(25.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=10 / 2").unwrap(),
            CellValue::Number(5.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=10 / 0"),
            Err(CellError::Div0)
        );
    }

    #[test]
    fn test_math_functions() {
        let mut wb = Workbook::with_default_sheet();
        let sheet = wb.active_sheet_mut();
        sheet.set_cell(CellCoord::must_new(1, 1), CellState::from_value(CellValue::Number(10.0))); // A1
        sheet.set_cell(CellCoord::must_new(2, 1), CellState::from_value(CellValue::Number(20.0))); // A2
        sheet.set_cell(CellCoord::must_new(3, 1), CellState::from_value(CellValue::Number(30.0))); // A3

        let engine = CalcEngine::new();
        let ctx = CalcContext::new(&wb, "Sheet1");

        assert_eq!(
            engine.evaluate_formula(&ctx, "=SUM(A1:A3)").unwrap(),
            CellValue::Number(60.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=PRODUCT(A1:A3)").unwrap(),
            CellValue::Number(6000.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=ABS(-42)").unwrap(),
            CellValue::Number(42.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=ROUND(3.14159, 2)").unwrap(),
            CellValue::Number(3.14)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=INT(9.9)").unwrap(),
            CellValue::Number(9.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=MOD(10, 3)").unwrap(),
            CellValue::Number(1.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=POWER(2, 3)").unwrap(),
            CellValue::Number(8.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=SQRT(16)").unwrap(),
            CellValue::Number(4.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=SQRT(-1)"),
            Err(CellError::Num)
        );
        assert!(matches!(
            engine.evaluate_formula(&ctx, "=PI()").unwrap(),
            CellValue::Number(n) if (n - 3.14159).abs() < 1e-4
        ));
    }

    #[test]
    fn test_logical_functions() {
        let wb = Workbook::with_default_sheet();
        let engine = CalcEngine::new();
        let ctx = CalcContext::new(&wb, "Sheet1");

        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=IF(10 > 5, "YES", "NO")"#).unwrap(),
            CellValue::Text("YES".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=IF(10 < 5, "YES", "NO")"#).unwrap(),
            CellValue::Text("NO".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=AND(TRUE, 1, 5 > 2)").unwrap(),
            CellValue::Boolean(true)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=AND(TRUE, 0)").unwrap(),
            CellValue::Boolean(false)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=OR(FALSE, 10 > 5)").unwrap(),
            CellValue::Boolean(true)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=NOT(TRUE)").unwrap(),
            CellValue::Boolean(false)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=IFERROR(10 / 0, "Error Occurred")"#).unwrap(),
            CellValue::Text("Error Occurred".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=IFERROR(10 / 2, "Error Occurred")"#).unwrap(),
            CellValue::Number(5.0)
        );
    }

    #[test]
    fn test_statistical_functions() {
        let mut wb = Workbook::with_default_sheet();
        let sheet = wb.active_sheet_mut();
        sheet.set_cell(CellCoord::must_new(1, 1), CellState::from_value(CellValue::Number(10.0)));
        sheet.set_cell(CellCoord::must_new(2, 1), CellState::from_value(CellValue::Number(20.0)));
        sheet.set_cell(CellCoord::must_new(3, 1), CellState::from_value(CellValue::Number(30.0)));
        sheet.set_cell(CellCoord::must_new(4, 1), CellState::from_value(CellValue::Text("Not a number".into())));

        let engine = CalcEngine::new();
        let ctx = CalcContext::new(&wb, "Sheet1");

        assert_eq!(
            engine.evaluate_formula(&ctx, "=AVERAGE(A1:A3)").unwrap(),
            CellValue::Number(20.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=MIN(A1:A3)").unwrap(),
            CellValue::Number(10.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=MAX(A1:A3)").unwrap(),
            CellValue::Number(30.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=COUNT(A1:A4)").unwrap(),
            CellValue::Number(3.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, "=COUNTA(A1:A4)").unwrap(),
            CellValue::Number(4.0)
        );
    }

    #[test]
    fn test_text_functions() {
        let wb = Workbook::with_default_sheet();
        let engine = CalcEngine::new();
        let ctx = CalcContext::new(&wb, "Sheet1");

        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=CONCAT("Hello", " ", "World")"#).unwrap(),
            CellValue::Text("Hello World".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=LEFT("Spreadsheet", 6)"#).unwrap(),
            CellValue::Text("Spread".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=RIGHT("Spreadsheet", 5)"#).unwrap(),
            CellValue::Text("sheet".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=MID("Rust EXEL", 6, 4)"#).unwrap(),
            CellValue::Text("EXEL".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=LEN("Rust EXEL")"#).unwrap(),
            CellValue::Number(9.0)
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=UPPER("hello")"#).unwrap(),
            CellValue::Text("HELLO".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=LOWER("HELLO")"#).unwrap(),
            CellValue::Text("hello".to_string())
        );
        assert_eq!(
            engine.evaluate_formula(&ctx, r#"=TRIM("   too   many    spaces   ")"#).unwrap(),
            CellValue::Text("too many spaces".to_string())
        );
    }

    #[test]
    fn test_lookup_functions() {
        let mut wb = Workbook::with_default_sheet();
        let sheet = wb.active_sheet_mut();
        // Setup lookup table:
        // A1: 101, B1: "Alice"
        // A2: 102, B2: "Bob"
        // A3: 103, B3: "Charlie"
        sheet.set_cell(CellCoord::must_new(1, 1), CellState::from_value(CellValue::Number(101.0)));
        sheet.set_cell(CellCoord::must_new(1, 2), CellState::from_value(CellValue::Text("Alice".into())));
        sheet.set_cell(CellCoord::must_new(2, 1), CellState::from_value(CellValue::Number(102.0)));
        sheet.set_cell(CellCoord::must_new(2, 2), CellState::from_value(CellValue::Text("Bob".into())));
        sheet.set_cell(CellCoord::must_new(3, 1), CellState::from_value(CellValue::Number(103.0)));
        sheet.set_cell(CellCoord::must_new(3, 2), CellState::from_value(CellValue::Text("Charlie".into())));

        let engine = CalcEngine::new();
        let ctx = CalcContext::new(&wb, "Sheet1");

        // VLOOKUP exact match
        assert_eq!(
            engine.evaluate_formula(&ctx, "=VLOOKUP(102, A1:B3, 2, FALSE)").unwrap(),
            CellValue::Text("Bob".to_string())
        );
        // VLOOKUP not found
        assert_eq!(
            engine.evaluate_formula(&ctx, "=VLOOKUP(999, A1:B3, 2, FALSE)"),
            Err(CellError::NA)
        );

        // MATCH exact match
        assert_eq!(
            engine.evaluate_formula(&ctx, "=MATCH(102, A1:A3, 0)").unwrap(),
            CellValue::Number(2.0)
        );

        // INDEX lookup
        assert_eq!(
            engine.evaluate_formula(&ctx, "=INDEX(A1:B3, 3, 2)").unwrap(),
            CellValue::Text("Charlie".to_string())
        );
    }

    #[test]
    fn test_recalculate_workbook_dependencies_and_cycle_detection() {
        let mut wb = Workbook::with_default_sheet();
        let sheet_id = wb.sheets[0].id.clone();
        let mut engine = CalcEngine::new();

        // Chain: A1 = 10, B1 = A1 * 2, C1 = B1 + 5
        let a1 = CellCoord::must_new(1, 1);
        let b1 = CellCoord::must_new(1, 2);
        let c1 = CellCoord::must_new(1, 3);

        engine.update_cell_value(&mut wb, &sheet_id, a1, CellValue::Number(10.0));
        engine.update_cell_formula(&mut wb, &sheet_id, b1, "=A1 * 2".into());
        engine.update_cell_formula(&mut wb, &sheet_id, c1, "=B1 + 5".into());

        assert_eq!(wb.sheets[0].get_cell_value(b1), CellValue::Number(20.0));
        assert_eq!(wb.sheets[0].get_cell_value(c1), CellValue::Number(25.0));

        // Update A1 -> incremental update recalculates B1 and C1
        engine.update_cell_value(&mut wb, &sheet_id, a1, CellValue::Number(50.0));
        assert_eq!(wb.sheets[0].get_cell_value(b1), CellValue::Number(100.0));
        assert_eq!(wb.sheets[0].get_cell_value(c1), CellValue::Number(105.0));

        // Create cycle: D1 = E1 + 1, E1 = D1 + 1
        let d1 = CellCoord::from_a1("D1").unwrap();
        let e1 = CellCoord::from_a1("E1").unwrap();
        engine.update_cell_formula(&mut wb, &sheet_id, d1, "=E1 + 1".into());
        engine.update_cell_formula(&mut wb, &sheet_id, e1, "=D1 + 1".into());

        assert_eq!(wb.sheets[0].get_cell_value(d1), CellValue::Error(CellError::Circular));
        assert_eq!(wb.sheets[0].get_cell_value(e1), CellValue::Error(CellError::Circular));
    }

    #[test]
    fn test_cross_sheet_recalculation() {
        let mut wb = Workbook::with_default_sheet();
        let s2_id = wb.add_sheet("Sheet2").unwrap();
        let s1_id = wb.sheets[0].id.clone();
        let mut engine = CalcEngine::new();

        // Sheet1!A1 = 100
        let a1 = CellCoord::must_new(1, 1);
        engine.update_cell_value(&mut wb, &s1_id, a1, CellValue::Number(100.0));

        // Sheet2!B1 = Sheet1!A1 * 1.5
        let b1 = CellCoord::must_new(1, 2);
        engine.update_cell_formula(&mut wb, &s2_id, b1, "=Sheet1!A1 * 1.5".into());

        assert_eq!(wb.get_sheet(&s2_id).unwrap().get_cell_value(b1), CellValue::Number(150.0));
    }
}
