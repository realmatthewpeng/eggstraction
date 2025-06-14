mod analysis;
mod cost;
mod extractor_structures;
mod faster_greedy_dag;
mod faster_ilp_cbc;
mod language;
mod rules;

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};

use egg::{Language, EGraph, Extractor, RecExpr, Runner};
use egraph_serialize::ClassId;

use analysis::{FieldType, TypeAnalysis};
use cost::{MathCostFn, PairCostFn};
use extractor_structures::Extractor as NewExtractor;
use language::Math;
use rules::{rules, pair_rules};

fn main() {
    env_logger::init();

    let mut symbol_types_file = "inputs/symbol_types.json";
    let mut cost_model_file = "inputs/cost_model.json";
    let mut test_case_file = "inputs/tests.txt";

    // Read command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        ()
    }
    else if args.len() == 4 {
        symbol_types_file = &args[1];
        cost_model_file = &args[2];
        test_case_file = &args[3];
    } else {
        eprintln!("Usage: {} <symbol_types.json> <cost_model.json> <tests.txt>", args[0]);
        std::process::exit(1);
    }

    // --- load symbol types from JSON ---
    let sym_json =
        fs::read_to_string(symbol_types_file).expect("Could not open symbol_types.json");
    let symbol_map: HashMap<String, FieldType> =
        serde_json::from_str(&sym_json).expect("Invalid JSON in symbol_types.json");

    // --- read each test expression ---
    let reader = BufReader::new(fs::File::open(test_case_file).expect("Could not open tests.txt"));

    let mut counter = 0;
    for line in reader.lines().filter_map(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        counter += 1;
        println!("Optimizing_Test_Case {}: ", counter);

        // parse into egg’s RecExpr
        let orig_expr: RecExpr<Math> = line
            .parse()
            .unwrap_or_else(|_| panic!("Invalid expr: {}", line));

        let analysis = TypeAnalysis::new(symbol_map.clone());

        let simplifier: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) 
            .with_expr(&orig_expr)
            .run(&pair_rules());
        let pair_costfn = PairCostFn;
        let simplifier_extractor = Extractor::new(&simplifier.egraph, pair_costfn);
        let (_expr_cost, expr) =
            simplifier_extractor.find_best(simplifier.egraph.find(simplifier.roots[0]));
        // println!("{}", _expr_cost);

        // 1. compute initial cost (no rewrites)
        let unopt_runner: Runner<Math, TypeAnalysis> =
            Runner::new(analysis.clone()).with_expr(&expr).run(&[]);

        let unopt_tree_costfn =
            MathCostFn::from_file(&unopt_runner.egraph, cost_model_file).unwrap();
        let unopt_tree_extractor =
            Extractor::new(&unopt_runner.egraph, unopt_tree_costfn);
        let (unopt_tree_cost, _) = unopt_tree_extractor
            .find_best(unopt_runner.egraph.find(unopt_runner.roots[0]));

        let unopt_dag_costfn =
            MathCostFn::from_file(&unopt_runner.egraph, cost_model_file).unwrap();
        let mut unopt_dag_serialized =
            egg_to_serialized_egraph(&unopt_runner.egraph, unopt_dag_costfn);
        unopt_dag_serialized
            .root_eclasses
            .push(ClassId::from(format!(
                "{}",
                unopt_runner
                    .egraph
                    .find(unopt_runner.roots[0])
            )));
        let unopt_dag_extractor = faster_ilp_cbc::FasterCbcExtractorWithTimeout::<180>;
        let unopt_dag_result = unopt_dag_extractor.extract(
            &unopt_dag_serialized,
            &unopt_dag_serialized.root_eclasses,
        );
        unopt_dag_result.check(&unopt_dag_serialized);
        let unopt_dag_cost = unopt_dag_result.dag_cost(
            &unopt_dag_serialized,
            &unopt_dag_serialized.root_eclasses,
        );

        // 2. compute optimized cost (with rewrites)
        let runner: Runner<Math, TypeAnalysis> =
            Runner::new(analysis.clone())
            .with_expr(&expr)
            //.with_iter_limit(100)
            //.with_node_limit(100_000)
            .run(&rules());
        // for its in &runner.iterations {
        //     println!("{:?}", its.applied);
        // }
        // println!(
        //     "DAG Runner stopped after {} iterations, reason: {:?}",
        //     runner.iterations.len(),
        //     runner.stop_reason
        // );

        let tree_costfn =
            MathCostFn::from_file(&runner.egraph, cost_model_file).unwrap();
        let tree_extractor = Extractor::new(&runner.egraph, tree_costfn);
        let (best_tree_cost, best_tree_expr) = tree_extractor
            .find_best(runner.egraph.find(runner.roots[0]));

        let dag_costfn =
            MathCostFn::from_file(&runner.egraph, cost_model_file).unwrap();
        let mut dag_serialized =
            egg_to_serialized_egraph(&runner.egraph, dag_costfn);
        dag_serialized
            .root_eclasses
            .push(ClassId::from(format!(
                "{}",
                runner.egraph.find(runner.roots[0])
            )));
        let dag_extractor = faster_ilp_cbc::FasterCbcExtractorWithTimeout::<180>;
        let dag_result =
            dag_extractor.extract(&dag_serialized, &dag_serialized.root_eclasses);
        dag_result.check(&dag_serialized);
        let best_dag_cost =
            dag_result.dag_cost(&dag_serialized, &dag_serialized.root_eclasses);
        let best_dag_expr = dag_result.dag_extracted_exprs(
            &dag_serialized,
            &dag_serialized.root_eclasses,
        )[0]
        .clone();

        println!(">>>");
        println!("Input expr           : {}",   line);
        println!("Simplified expr      : {}\n", expr);
        println!("Tree: Initial cost   : {}",   unopt_tree_cost);
        println!("Tree: Optimized expr : {}",   best_tree_expr);
        println!("Tree: Optimized cost : {}\n", best_tree_cost);
        println!("DAG:  Initial cost   : {}",   unopt_dag_cost);
        println!("DAG:  Optimized expr : {}",   best_dag_expr);
        println!("DAG:  Optimized cost : {}",   best_dag_cost);
        println!("<<<");
    }
}

pub fn egg_to_serialized_egraph(
    egraph: &EGraph<Math, TypeAnalysis>,
    mut costfn: MathCostFn,
) -> egraph_serialize::EGraph
{
    use egraph_serialize::*;
    let mut out = EGraph::default();
    for class in egraph.classes() {
        for (i, node) in class.nodes.iter().enumerate() {
            let cost = costfn.calc_enode_cost(node) as f64;
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
                    cost: Cost::new(cost).unwrap(),
                },
            )
        }
    }
    out
}
