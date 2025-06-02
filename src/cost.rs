use crate::analysis::{FieldType, TypeAnalysis};
use crate::language::Math;
use egg::{CostFunction, EGraph, Id, Language};
use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};

/// JSON format example:
///
/// {
///   "costs": {
///     "fp":  { "+": 1, "-": 1, "*": 3, "*const": 2, "inv": 10, "sq": 2, "const": 0, "symbol": 0 },
///     "fp2": { "+": 2, "-": 2, "*": 8, "*const": 5, "inv": 50, "sq": 5, "const": 0, "symbol": 0 },
///     "fp4": { "+": 4, "-": 4, "*": 20, "*const": 12, "inv": 200, "sq": 20, "const": 0, "symbol": 0 }
///   },
///   "default_costs": {
///     "+": 1, "-": 1, "*": 3, "*const": 2, "inv": 10, "sq": 2, "const": 0, "symbol": 0
///   }
/// }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModel {
    /// Mapping e.g. "fp" → { "+": 1, "*": 3, ... }
    pub costs: HashMap<String, HashMap<String, usize>>,
    /// Fallbacks if a field‐type or operation is missing
    pub default_costs: HashMap<String, usize>,
}

impl CostModel {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = fs::read_to_string(path)?;
        let model: CostModel = serde_json::from_str(&data)?;
        Ok(model)
    }

    /// Look up “costs[field_type_str][operation]”, or fallback to default_costs[operation], or 1.
    pub fn get_cost(&self, field_type: &FieldType, operation: &str) -> usize {
        let key = field_type.to_string();
        if let Some(field_costs) = self.costs.get(&key) {
            if let Some(&c) = field_costs.get(operation) {
                return c;
            }
        }
        self.default_costs.get(operation).copied().unwrap_or(0)
    }
}

/// A single struct that implements both `CostFunction<Math>` (for tree‐extraction)
/// and also exposes a `calc_enode_cost(...)` helper (for serializing to DAG‐ILP).
pub struct MathCostFn {
    cost_model: CostModel,
    /// Copy of the symbol_type map (so we can label “symbol” ops if needed).
    symbol_types: HashMap<String, FieldType>,
    /// **NEW**: store a clone of the current EGraph so `cost(...)` can inspect child‐node types.
    pub egraph: EGraph<Math, TypeAnalysis>,
}

impl MathCostFn {
    /// Build from an existing EGraph; clones it internally.
    pub fn from_file(
        egraph: &EGraph<Math, TypeAnalysis>,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cost_model = CostModel::from_file(path)?;
        let symbol_types = egraph.analysis.symbol_types.clone();
        let egraph_clone = egraph.clone();
        Ok(MathCostFn {
            cost_model,
            symbol_types,
            egraph: egraph_clone,
        })
    }

    /// Recompute the “field‐type” of a given enode (Add/Sub/Mul/Inv/Sq/Const/Symbol),
    /// exactly as TypeAnalysis did during `make(...)`.
    fn determine_enode_type(&self, enode: &Math) -> FieldType {
        match enode {
            Math::Pair([a, b]) => {
                let type_a = &self.egraph[*a].data;
                let type_b = &self.egraph[*b].data;
                // Result type is determined by pairing logic
                match (type_a, type_b) {
                    (FieldType::Fp, FieldType::Fp) => FieldType::FpExt(2),
                    (FieldType::FpExt(n1), FieldType::FpExt(n2)) if n1 == n2 => {
                        FieldType::FpExt(n1 * 2)
                    }
                    _ => type_a.lcm_extension(type_b),
                }
            }
            
            Math::Fst(id) | Math::Snd(id) => {
                let input_type = &self.egraph[*id].data;
                match input_type {
                    FieldType::FpExt(n) if *n > 1 => {
                        let new_degree = n / 2;
                        if new_degree == 1 {
                            FieldType::Fp
                        } else {
                            FieldType::FpExt(new_degree)
                        }
                    }
                    _ => FieldType::Fp,
                }
            }

            Math::Add([a, b]) | Math::Sub([a, b]) | Math::Mul([a, b]) => {
                // Take LCM of both children’s types
                let type_a = &self.egraph[*a].data;
                let type_b = &self.egraph[*b].data;
                type_a.lcm_extension(type_b)
            }
            Math::Inv(x) | Math::Sq(x) => self.egraph[*x].data.clone(),
            Math::Constant(_) => FieldType::Constant,
            Math::Symbol(sym) => {
                let name = sym.as_str().to_string();
                self.symbol_types.get(&name).cloned().unwrap_or(FieldType::Fp)
            }
        }
    }

    /// Decide the operation‐string (e.g. "+", "-", "*", "*const", "inv", "sq", "const", "symbol").
    fn get_operation_string(&self, enode: &Math) -> String {
        match enode {
            Math::Add(_) => "+".to_string(),
            Math::Sub(_) => "-".to_string(),
            Math::Mul([a, b]) => {
                let type_a = &self.egraph[*a].data;
                let type_b = &self.egraph[*b].data;
                // If either child’s eclass has a Math::Constant(_), call it "*const"
                let child_a_const = self.egraph[*a]
                    .nodes
                    .iter()
                    .any(|n| matches!(n, Math::Constant(_))) || (*type_a == FieldType::Constant);
                let child_b_const = self.egraph[*b]
                    .nodes
                    .iter()
                    .any(|n| matches!(n, Math::Constant(_))) || (*type_b == FieldType::Constant);
                if child_a_const || child_b_const {
                    "*const".to_string()
                } else {
                    "*".to_string()
                }
            }
            Math::Inv(_) => "inv".to_string(),
            Math::Sq(_) => "sq".to_string(),
            Math::Constant(_) => "const".to_string(),
            Math::Symbol(_) => "symbol".to_string(),
            Math::Pair(_) => "pair".to_string(),
            Math::Fst(_) => "fst".to_string(),
            Math::Snd(_) => "snd".to_string(),
        }
    }

    /// This is the core “per‐enode” cost function used by both tree and DAG codepaths.
    pub fn calc_enode_cost(&mut self, enode: &Math) -> usize {
        // 1. Find the resulting FieldType
        let enode_type = self.determine_enode_type(enode);
        // 2. Pick operation‐string
        let op = self.get_operation_string(enode);
        // 3. Look up numeric cost
        self.cost_model.get_cost(&enode_type, &op)
    }
}

/// === IMPLEMENT THE `CostFunction<Math>` TRAIT SO Extractor::new(...) COMPILES ===
impl CostFunction<Math> for MathCostFn {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &Math, mut child_costs: C) -> usize
    where
        C: FnMut(Id) -> usize,
    {
        let op_cost = self.calc_enode_cost(enode);
        // `enode.fold(initial, |sum, id| sum + child_costs(id))` adds up all children
        enode.fold(op_cost, |sum, id| sum + child_costs(id))
    }
}
