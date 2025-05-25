use std::{collections::HashMap, fs};
use egg::{CostFunction, Id, Language};
use crate::language::Math;

pub struct MathCostFn {
    costs: HashMap<String, usize>,
    egraph: crate::language::EGraph,
}

impl MathCostFn {
    pub fn new(egraph: crate::language::EGraph, path: &str) -> Self {
        let data = fs::read_to_string(path)
            .expect("Failed to read cost model JSON");
        let costs: HashMap<String, usize> = serde_json::from_str(&data)
            .expect("Invalid JSON in cost model");
        MathCostFn { costs, egraph }
    }
}

impl CostFunction<Math> for MathCostFn {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &Math, mut child_costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        // base op cost from JSON
        let op = match enode {
            Math::Inv(_)      => "inv",
            Math::Mul([a, b]) => {
                let a_const = self.egraph[*a].nodes.iter().any(|n| matches!(n, Math::Constant(..)));
                let b_const = self.egraph[*b].nodes.iter().any(|n| matches!(n, Math::Constant(..)));
                if a_const || b_const {
                    "mul_const"
                } else {
                    "mul"
                }
            }
            Math::Add(_)      => "plus",
            Math::Sub(_)      => "minus",
            _                 => "",
        };
        let op_cost = *self.costs.get(op).unwrap_or(&0);
        enode.fold(op_cost, |sum, id| sum + child_costs(id))
    }
}