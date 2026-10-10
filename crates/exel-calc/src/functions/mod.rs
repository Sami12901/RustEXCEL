pub mod logical;
pub mod lookup;
pub mod math;
pub mod statistical;
pub mod text;

use crate::context::CalcContext;
use exel_cell::{CellError, CellValue};
use exel_formula::Expression;
use math::EvalFn;
use std::collections::HashMap;

pub type FunctionHandler = fn(&CalcContext, &[Expression], EvalFn) -> Result<CellValue, CellError>;

/// Registry holding all available Excel formula functions.
pub struct FunctionRegistry {
    functions: HashMap<String, FunctionHandler>,
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    pub fn register(&mut self, name: &str, handler: FunctionHandler) {
        self.functions.insert(name.to_ascii_uppercase(), handler);
    }

    pub fn get(&self, name: &str) -> Option<FunctionHandler> {
        self.functions.get(&name.to_ascii_uppercase()).copied()
    }

    fn register_builtins(&mut self) {
        // Mathematical
        self.register("SUM", math::fn_sum);
        self.register("PRODUCT", math::fn_product);
        self.register("ABS", math::fn_abs);
        self.register("ROUND", math::fn_round);
        self.register("INT", math::fn_int);
        self.register("MOD", math::fn_mod);
        self.register("POWER", math::fn_power);
        self.register("SQRT", math::fn_sqrt);
        self.register("PI", math::fn_pi);

        // Logical
        self.register("IF", logical::fn_if);
        self.register("AND", logical::fn_and);
        self.register("OR", logical::fn_or);
        self.register("NOT", logical::fn_not);
        self.register("IFERROR", logical::fn_iferror);

        // Statistical
        self.register("AVERAGE", statistical::fn_average);
        self.register("MIN", statistical::fn_min);
        self.register("MAX", statistical::fn_max);
        self.register("COUNT", statistical::fn_count);
        self.register("COUNTA", statistical::fn_counta);

        // Text
        self.register("CONCAT", text::fn_concat);
        self.register("CONCATENATE", text::fn_concat);
        self.register("LEFT", text::fn_left);
        self.register("RIGHT", text::fn_right);
        self.register("MID", text::fn_mid);
        self.register("LEN", text::fn_len);
        self.register("UPPER", text::fn_upper);
        self.register("LOWER", text::fn_lower);
        self.register("TRIM", text::fn_trim);

        // Lookup
        self.register("VLOOKUP", lookup::fn_vlookup);
        self.register("MATCH", lookup::fn_match);
        self.register("INDEX", lookup::fn_index);
    }
}
