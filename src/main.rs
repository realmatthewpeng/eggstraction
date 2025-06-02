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

use egg::{EGraph, Extractor, RecExpr, Runner};
use egraph_serialize::ClassId;

use analysis::{FieldType, TypeAnalysis};
use cost::{MathCostFn, PairCostFn};
use extractor_structures::Extractor as NewExtractor;
use language::Math;
use rules::{rules, pair_rules};

fn main() {
    env_logger::init();

    let mut symbol_types_file = "symbol_types.json";
    let mut cost_model_file = "cost_model.json";
    let mut test_case_file = "tests.txt";

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

        // 1. compute initial cost (no rewrites)
        let unopt_runner: Runner<Math, TypeAnalysis> =
            Runner::new(analysis.clone()).with_expr(&expr).run(&[]);

        let unopt_tree_costfn =
            MathCostFn::from_file(&unopt_runner.egraph, cost_model_file).unwrap();
        let unopt_tree_extractor =
            Extractor::new(&unopt_runner.egraph, unopt_tree_costfn);
        let (unopt_tree_cost, _) = unopt_tree_extractor
            .find_best(unopt_runner.egraph.find(unopt_runner.roots[0]));

        let mut unopt_dag_costfn =
            MathCostFn::from_file(&unopt_runner.egraph, cost_model_file).unwrap();
        let mut unopt_dag_serialized =
            egg_to_serialized_egraph(&unopt_runner.egraph, &mut unopt_dag_costfn);
        unopt_dag_serialized
            .root_eclasses
            .push(ClassId::from(format!(
                "{}",
                unopt_runner
                    .egraph
                    .find(unopt_runner.roots[0])
            )));
        let unopt_dag_extractor = faster_ilp_cbc::FasterCbcExtractor;
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
            Runner::new(analysis.clone()).with_expr(&expr).run(&rules());

        let tree_costfn =
            MathCostFn::from_file(&runner.egraph, cost_model_file).unwrap();
        let tree_extractor = Extractor::new(&runner.egraph, tree_costfn);
        let (best_tree_cost, best_tree_expr) = tree_extractor
            .find_best(runner.egraph.find(runner.roots[0]));

        let mut dag_costfn =
            MathCostFn::from_file(&runner.egraph, cost_model_file).unwrap();
        let mut dag_serialized =
            egg_to_serialized_egraph(&runner.egraph, &mut dag_costfn);
        dag_serialized
            .root_eclasses
            .push(ClassId::from(format!(
                "{}",
                runner.egraph.find(runner.roots[0])
            )));
        let dag_extractor = faster_ilp_cbc::FasterCbcExtractor;
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
    costfn: &mut MathCostFn,
) -> egraph_serialize::EGraph {
    use egg::Language; // brings `.children()` into scope
    use egraph_serialize::*;

    // 1. Clone once, up front. Do *not* re-clone on every enode.
    costfn.egraph = egraph.clone();

    let mut out = EGraph::default();
    for class in egraph.classes() {
        let eclass_id = class.id;
        // Precompute the string version of this eclass:
        let eclass_str = eclass_id.to_string();

        for (i, node) in class.nodes.iter().enumerate() {
            // 2. Compute cost now that costfn.egraph is already set:
            let cost = costfn.calc_enode_cost(node) as f64;

            // 3. Build the unique node ID just once per enode:
            let node_id = format!("{}.{}", eclass_str, i);

            // 4. Build children list, reusing the same eclass string for prefix:
            //    Each child is an eclass‐ID, so we do "<child_eclass>.0"
            let mut children = Vec::with_capacity(node.children().len());
            for &child_id in node.children() {
                // only one format! child_id is usize, so:
                children.push(NodeId::from(format!("{}.0", child_id)));
            }

            out.add_node(
                node_id,
                Node {
                    op: node.to_string(),
                    children,
                    // convert the eclass_str into a ClassId once:
                    eclass: ClassId::from(eclass_str.clone()),
                    cost: Cost::new(cost).unwrap(),
                },
            );
        }
    }
    out
}
