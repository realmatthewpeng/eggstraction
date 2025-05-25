use std::fs::File;
use std::io::{BufRead, BufReader};
use egg::{Runner, Extractor};

mod language;
mod rules;
mod cost;

use language::{Math};
use rules::rules;
use cost::MathCostFn;

fn main() {
    let file = File::open("test.txt").expect("Could not open test.txt");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let expr_str = line.expect("Failed to read line");
        if expr_str.trim().is_empty() { continue; }

        let expr: egg::RecExpr<Math> = expr_str.parse().unwrap();
        let runner = Runner::default().with_expr(&expr).run(&rules());
        let costfn = MathCostFn::new(runner.egraph.clone(), "cost_model.json");
        let extractor = Extractor::new(&runner.egraph, costfn);
        let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);

        println!("Input: {}", expr_str);
        println!("Best cost: {}", best_cost);
        println!("Best expr: {}", best_expr);
        println!("---");
    }
}