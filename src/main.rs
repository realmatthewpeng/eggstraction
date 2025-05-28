mod analysis;
mod cost;
mod language;
mod rules;

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};

use egg::{Extractor, LpExtractor, RecExpr, Runner};

use analysis::{Type, TypeAnalysis};
use cost::MathCostFn;
use language::Math;
use rules::rules;

fn main() {
    // --- load symbol types from JSON ---
    let sym_json =
        fs::read_to_string("symbol_types.json").expect("Could not open symbol_types.json");
    let symbol_map: HashMap<String, Type> =
        serde_json::from_str(&sym_json).expect("Invalid JSON in symbol_types.json");

    // --- read each test expression ---
    let reader = BufReader::new(fs::File::open("tests.txt").expect("Could not open tests.txt"));

    for line in reader.lines().filter_map(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // parse into egg’s RecExpr
        let expr: RecExpr<Math> = line
            .parse()
            .unwrap_or_else(|_| panic!("Invalid expr: {}", line));

        // make a fresh TypeAnalysis + Runner
        let analysis = TypeAnalysis::new(symbol_map.clone());

        // 1. compute initial cost (no rewrites)
        let unopt_tree_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // clone the analysis so we can reuse it
            .with_expr(&expr)
            .run(&[]); // no rules
        let unopt_tree_costfn =
            MathCostFn::new(unopt_tree_runner.egraph.clone(), "cost_model.json");
        let unopt_tree_extractor = Extractor::new(&unopt_tree_runner.egraph, unopt_tree_costfn);
        let (unopt_tree_cost, _) = unopt_tree_extractor.find_best(unopt_tree_runner.roots[0]);

        // 2. compute optimized cost (with rewrites)
        let tree_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // reuse the analysis struct
            .with_expr(&expr)
            .run(&rules());
        let tree_costfn = MathCostFn::new(tree_runner.egraph.clone(), "cost_model.json");
        let tree_extractor = Extractor::new(&tree_runner.egraph, tree_costfn);
        let (best_tree_cost, best_tree_expr) = tree_extractor.find_best(tree_runner.roots[0]);

        let dag_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // clone the analysis so we can reuse it
            .with_expr(&expr)
            .run(&rules()); // no rules
        let dag_costfn = MathCostFn::new(dag_runner.egraph.clone(), "cost_model.json");
        let mut dag_egraph = dag_runner.egraph.clone();
        let dag_egraph_root = dag_egraph.add_expr(&expr);
        let dag_best_expr = LpExtractor::new(&dag_egraph, dag_costfn).solve(dag_egraph_root);

        // 3) print both
        println!("In                  : {}", line);
        println!("Initial tree cost   : {}", unopt_tree_cost);
        println!("Optimized tree expr : {}", best_tree_expr);
        println!("Optimized tree cost : {}", best_tree_cost);
        println!("Optimized DAG expr  : {}", dag_best_expr);
        println!("---");
    }
}
