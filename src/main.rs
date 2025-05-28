mod language;
mod rules;
mod analysis;
mod cost;

use std::{
    fs,
    io::{BufRead, BufReader},
    collections::HashMap,
};
use egg::{Runner, Extractor, RecExpr, LpExtractor};
use language::Math;
use rules::rules;
use analysis::{Type, TypeAnalysis};
use cost::MathCostFn;


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
            Runner::new(analysis.clone())            // reuse the analysis struct
                  .with_expr(&expr)
                  .run(&rules());
        let cost1 = MathCostFn::new(runner1.egraph.clone(), "cost_model.json");
        let ext1  = Extractor::new(&runner1.egraph, cost1);
        let (best_cost, best) = ext1.find_best(runner1.roots[0]);
        println!("{:?}", runner1.egraph);


        let runner2: Runner<Math, TypeAnalysis> =
            Runner::new(analysis.clone())   // clone the analysis so we can reuse it
                  .with_expr(&expr)
                  .run(&rules());                 // no rules
        let mut egraph = runner2.egraph.clone();
        let root = egraph.add_expr(&expr);

        let cost2 = MathCostFn::new(runner2.egraph.clone(), "cost_model.json");

        let lp_best = LpExtractor::new(&egraph, cost2).solve(root);
        println!("LP best: {}", lp_best);

        // 3) print both
        println!("In             : {}", line);
        println!("Initial cost   : {}", init_cost);
        println!("Optimized expr : {}", best);
        println!("Optimized cost : {}", best_cost);
        // println!("---");
    }
}