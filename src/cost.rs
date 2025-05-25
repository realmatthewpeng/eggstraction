use std::{collections::HashMap, fs};
use egg::{CostFunction, EGraph, Id, Language};
use crate::language::Math;
use crate::analysis::{Type, TypeAnalysis};

pub struct MathCostFn {
    cost_models: HashMap<Type, HashMap<String, usize>>,
    egraph: EGraph<Math, TypeAnalysis>,
}

impl MathCostFn {
    pub fn new(egraph: EGraph<Math, TypeAnalysis>, path: &str) -> Self {
        let data = fs::read_to_string(path)
            .expect("Failed to read cost model JSON");
        let cost_models: HashMap<Type, HashMap<String, usize>> =
            serde_json::from_str(&data)
                .expect("Invalid JSON in cost model");
        MathCostFn { cost_models, egraph }
    }
}

impl CostFunction<Math> for MathCostFn {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &Math, mut child_costs: C) -> usize
    where
        C: FnMut(Id) -> usize,
    {
        // 1) Figure out the result‐type of this enode
        let enode_type = match enode {
            Math::Pair(_)        => Type::Ext2,
            Math::Fst(_) | Math::Snd(_) => Type::Fp,
            Math::Add(_) | Math::Sub(_) | Math::Inv(_) | Math::Sq(_) => {
                // get the Id of the first child
                let first_child = enode.children()[0];
                // look up its Data
                self.egraph[first_child].data.clone()
            }
            Math::Mul(_) => {
                let ta = enode.children()[0];
                let tb = enode.children()[1];
                if self.egraph[ta].data == Type::Ext2 || self.egraph[tb].data == Type::Ext2 {
                    Type::Ext2
                } else {
                    Type::Fp
                }
            }
            Math::Constant(_) | Math::Symbol(_) => Type::Fp,
        };

        // 2) Pick the JSON key for this operator
        let key = match enode {
            Math::Inv(_)       => "inv",
            Math::Mul([a, b])  => {
                // cheaper constant‐mul if either side is a lit
                if matches!(self.egraph[*a].nodes[0], Math::Constant(_))
                 || matches!(self.egraph[*b].nodes[0], Math::Constant(_))
                {
                    "*const"
                } else {
                    "*"
                }
            }
            Math::Add(_)       => "+",
            Math::Sub(_)       => "-",
            Math::Sq(_)        => "sq",
            _                  => "",
        };

        // 3) Lookup cost and fold in children
        let op_cost = self.cost_models
            .get(&enode_type)
            .and_then(|m| m.get(key))
            .cloned()
            .unwrap_or(0);
        enode.fold(op_cost, |sum, id| sum + child_costs(id))
    }
}
