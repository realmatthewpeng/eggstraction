use std::{collections::HashMap, fs};
use egg::{CostFunction, EGraph, Id, Language, LpCostFunction, Analysis};
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
                 || self.egraph[*a].data == Type::Fp
                 || self.egraph[*b].data == Type::Fp
                {
                    "*const"
                } else {
                    "*"
                }
            }
            Math::Add(_)       => "+",
            Math::Sub(_)       => "-",
            Math::Sq(_a) => {
                // if let Some(name) = id_to_name(&self.egraph, *a) {
                //     println!("Math::Sq enode: sq({}) {:?}", name, a);
                // } else {
                //     println!("Math::Sq enode: sq({:?})", a);
                // }
                "sq"
            },
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

fn id_to_name(egraph: &EGraph<Math, TypeAnalysis>, id: Id) -> Option<String> {
    egraph[id]
        .nodes
        .iter()
        .find_map(|n| match n {
            Math::Symbol(sym) => Some(sym.to_string()),
            Math::Constant(cons) => Some(cons.to_string()),
            _ => None,
        })
}

impl LpCostFunction<Math, TypeAnalysis> for MathCostFn {
    fn node_cost(&mut self, _egraph: &EGraph<Math, TypeAnalysis>, _eclass: Id, _enode: &Math) -> f64 {
        match _enode {
            Math::Inv(_)       => 80.0,
            Math::Mul([a, b])  => 10.0,
            Math::Add(_)       => 1.0,
            Math::Sub(_)       => 1.0,
            Math::Sq(_a) => 6.0,
            _                 => 0.0,
        }   
    }
}