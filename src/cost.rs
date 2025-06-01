use crate::analysis::{Type, TypeAnalysis};
use crate::language::Math;
use egg::{CostFunction, EGraph, Id, Language, LpCostFunction};
use std::{collections::HashMap, fs};

pub struct MathCostFn {
    cost_models: HashMap<Type, HashMap<String, usize>>,
    egraph: EGraph<Math, TypeAnalysis>,
}

impl MathCostFn {
    pub fn new(egraph: EGraph<Math, TypeAnalysis>, path: &str) -> Self {
        let data = fs::read_to_string(path).expect("Failed to read cost model JSON");
        let cost_models: HashMap<Type, HashMap<String, usize>> =
            serde_json::from_str(&data).expect("Invalid JSON in cost model");
        MathCostFn {
            cost_models,
            egraph,
        }
    }

    pub fn calc_enode_cost(&mut self, enode: &Math) -> usize {
        // 1. Figure out the result‐type of this enode
        let enode_type = match enode {
            Math::Pair(_) => Type::Ext2,
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

        // 2. Pick the JSON key for this operator
        let key = match enode {
            Math::Inv(_) => "inv",
            Math::Mul([a, b]) => {
                // cheaper constant‐mul if either side is a lit
                if matches!(self.egraph[*a].nodes[0], Math::Constant(_))
                    || matches!(self.egraph[*b].nodes[0], Math::Constant(_))
                    || self.egraph[*a].data == Type::Fp
                    || self.egraph[*b].data == Type::Fp
                {
                    "*const"
                } else {
                    "*"
                }
            }
            Math::Add(_) => "+",
            Math::Sub(_) => "-",
            Math::Sq(_a) => "sq",
            _ => "",
        };

        // 3. Lookup cost and return it
        self.cost_models
            .get(&enode_type)
            .and_then(|m| m.get(key))
            .cloned()
            .unwrap_or(0)
    }
}

impl CostFunction<Math> for MathCostFn {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &Math, mut child_costs: C) -> usize
    where
        C: FnMut(Id) -> usize,
    {
        let op_cost = self.calc_enode_cost(enode);
        enode.fold(op_cost, |sum, id| sum + child_costs(id))
    }
}

fn _id_to_name(egraph: &EGraph<Math, TypeAnalysis>, id: Id) -> Option<String> {
    egraph[id].nodes.iter().find_map(|n| match n {
        Math::Symbol(sym) => Some(sym.to_string()),
        Math::Constant(cons) => Some(cons.to_string()),
        _ => None,
    })
}

impl LpCostFunction<Math, TypeAnalysis> for MathCostFn {
    fn node_cost(
        &mut self,
        _egraph: &EGraph<Math, TypeAnalysis>,
        _eclass: Id,
        _enode: &Math,
    ) -> f64 {
       let op_cost = self.calc_enode_cost(_enode);

        op_cost as f64
    }
}

pub struct PairCostFn;

impl CostFunction<Math> for PairCostFn {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &Math, mut child_costs: C) -> usize
    where
        C: FnMut(Id) -> usize,
    {
        let op_cost = match enode {
            Math::Constant(_) => 0,
            Math::Symbol(_) => 0,
            Math::Pair(_) => 10,
            _ => 2,
        };
        match enode {
            Math::Pair(_) => enode.fold(10, |sum: usize, id| sum + op_cost * child_costs(id)),
            _ => enode.fold(0, |sum: usize, id| sum + op_cost * child_costs(id))

        }
    }
}