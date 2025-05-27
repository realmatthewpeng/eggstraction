mod language;
mod rules;
mod analysis;
mod cost;
mod greedy_dag;

use std::{
    fs,
    io::{BufRead, BufReader},
    collections::HashMap,
};
use egg::{Runner, Extractor, RecExpr, EGraph, Analysis, Language, Id};
use language::Math;
use rules::rules;
use analysis::{Type, TypeAnalysis};
use cost::MathCostFn;
use std::fmt::Display;
use crate::greedy_dag::Extractor as GreedyDagExtractorTrait;
use egraph_serialize::{ClassId};


fn main() {
    // --- load symbol types from JSON ---
    let sym_json = fs::read_to_string("symbol_types.json")
        .expect("Could not open symbol_types.json");
    let symbol_map: HashMap<String, Type> =
        serde_json::from_str(&sym_json)
            .expect("Invalid JSON in symbol_types.json");

    // --- read each test expression ---
    let reader = BufReader::new(
        fs::File::open("tests.txt").expect("Could not open tests.txt")
    );

    for line in reader.lines().filter_map(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // parse into egg’s RecExpr
        let expr: RecExpr<Math> = line.parse()
            .unwrap_or_else(|_| panic!("Invalid expr: {}", line));

        // make a fresh TypeAnalysis + Runner
        let analysis = TypeAnalysis::new(symbol_map.clone());

        // 1) compute initial cost (no rewrites)
        let runner0: Runner<Math, TypeAnalysis> =
            Runner::new(analysis.clone())   // clone the analysis so we can reuse it
                  .with_expr(&expr)
                  .run(&[]);                 // no rules
        let cost0 = MathCostFn::new(runner0.egraph.clone(), "cost_model.json");
        let ext0  = Extractor::new(&runner0.egraph, cost0);
        let (init_cost, _) = ext0.find_best(runner0.roots[0]);

        // 2) compute optimized cost (with your rules)
        let runner1: Runner<Math, TypeAnalysis> =
            Runner::new(analysis)            // reuse the analysis struct
                  .with_expr(&expr)
                  .run(&rules());
        let cost1 = MathCostFn::new(runner1.egraph.clone(), "cost_model.json");
        let ext1  = Extractor::new(&runner1.egraph, cost1);
        let (best_cost, best) = ext1.find_best(runner1.roots[0]);

        let mut serialized = egg_to_serialized_egraph(&runner0.egraph);
        serialized.root_eclasses.push(ClassId::from(format!("{}", runner0.roots[0])));
        println!("Serialized egraph: {:?}", serialized);
        let extractor = greedy_dag::FasterGreedyDagExtractor;
        let extraction_result = GreedyDagExtractorTrait::extract(&extractor, &serialized, &serialized.root_eclasses);

        extraction_result.check(&serialized);
        let tree = extraction_result.tree_cost(&serialized, &serialized.root_eclasses);
        let dag = extraction_result.dag_cost(&serialized, &serialized.root_eclasses);
        println!("Tree cost: {}", tree);
        println!("DAG cost: {}", dag);

        // 3) print both
        println!("In             : {}", line);
        println!("Initial cost   : {}", init_cost);
        println!("Optimized expr : {}", best);
        println!("Optimized cost : {}", best_cost);
        println!("---");
    }
}

pub fn egg_to_serialized_egraph<L, A>(egraph: &EGraph<L, A>) -> egraph_serialize::EGraph
where
    L: Language + Display,
    A: Analysis<L>,
{
    use egraph_serialize::*;
    let mut out = EGraph::default();
    for class in egraph.classes() {
        for (i, node) in class.nodes.iter().enumerate() {
            out.add_node(
                format!("{}.{}", class.id, i),
                Node {
                    op: node.to_string(),
                    children: node
                        .children()
                        .iter()
                        .map(|id| NodeId::from(format!("{}.0", id)))
                        .collect(),
                    eclass: ClassId::from(format!("{}", class.id)),
                    cost: Cost::new(1.0).unwrap(),
                },
            )
        }
    }
    out
}