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

    let mut counter = 0;
    for line in reader.lines().filter_map(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        counter += 1;
        println!("Optimizing_Test_Case {}: ", counter);

        // parse into egg’s RecExpr
        let expr: RecExpr<Math> = line
            .parse()
            .unwrap_or_else(|_| panic!("Invalid expr: {}", line));

        let analysis = TypeAnalysis::new(symbol_map.clone());

        // 1. compute initial cost (no rewrites)
        let unopt_tree_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // clone the analysis so we can reuse it
            .with_expr(&expr)
            .run(&[]); // no rules
        let unopt_tree_costfn =
            MathCostFn::new(unopt_tree_runner.egraph.clone(), "cost_model.json");
        let unopt_tree_extractor = Extractor::new(&unopt_tree_runner.egraph, unopt_tree_costfn);
        let (unopt_tree_cost, _) = unopt_tree_extractor.find_best(unopt_tree_runner.roots[0]);

        let unopt_dag_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // clone the analysis so we can reuse it
            .with_expr(&expr)
            .run(&[]);
        let unopt_dag_costfn = MathCostFn::new(unopt_dag_runner.egraph.clone(), "cost_model.json");
        let unopt_dag_roots = unopt_dag_runner.roots.iter().map(|id| unopt_dag_runner.egraph.find(*id)).collect::<Vec<_>>();
        LpExtractor::new(&unopt_dag_runner.egraph, unopt_dag_costfn).solve_multiple(&unopt_dag_roots);

        // 2. compute optimized cost (with rewrites)
        let tree_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // reuse the analysis struct
            .with_expr(&expr)
            .run(&rules());
        let tree_costfn = MathCostFn::new(tree_runner.egraph.clone(), "cost_model.json");
        let tree_extractor = Extractor::new(&tree_runner.egraph, tree_costfn);
        let (best_tree_cost, best_tree_expr) = tree_extractor.find_best(tree_runner.roots[0]);

        let dag_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // clone the analysis so we can reuse it
            .with_expr(&expr)
            .run(&rules());
        println!(
            "DAG Runner stopped after {} iterations, reason: {:?}",
            dag_runner.iterations.len(),
            dag_runner.stop_reason
        );
        // println!("{}", dag_runner.egraph.dot());
        let dag_costfn = MathCostFn::new(dag_runner.egraph.clone(), "cost_model.json");
        let dag_roots = dag_runner.roots.iter().map(|id| dag_runner.egraph.find(*id)).collect::<Vec<_>>();
        let (dag_best_expr, _) = LpExtractor::new(&dag_runner.egraph, dag_costfn).solve_multiple(&dag_roots);

        println!(">>>");
        println!("Input expr           : {}\n", line);
        println!("Tree: Initial cost   : {}", unopt_tree_cost);
        println!("Tree: Optimized expr : {}", best_tree_expr);
        println!("Tree: Optimized cost : {}\n", best_tree_cost);
        println!("DAG: Optimized expr  : {}", dag_best_expr);
        println!("<<<");
    }
}
