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

use egg::{Analysis, EGraph, Extractor, Language, RecExpr, Runner};
use egraph_serialize::ClassId;

use analysis::{Type, TypeAnalysis};
use cost::MathCostFn;
use extractor_structures::Extractor as NewExtractor;
use language::Math;
use rules::rules;

fn main() {
    env_logger::init();

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
        let (unopt_tree_cost, _) = unopt_tree_extractor.find_best(unopt_tree_runner.egraph.find(unopt_tree_runner.roots[0]));

        let unopt_dag_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // clone the analysis so we can reuse it
            .with_expr(&expr)
            .run(&[]);
        let unopt_dag_costfn =
            MathCostFn::new(unopt_dag_runner.egraph.clone(), "cost_model.json");
        let mut unopt_dag_serialized =
            egg_to_serialized_egraph::<Math, _>(&unopt_dag_runner.egraph, unopt_dag_costfn);
        unopt_dag_serialized
            .root_eclasses
            .push(ClassId::from(format!("{}", unopt_dag_runner.egraph.find(unopt_dag_runner.roots[0]))));
        let unopt_dag_extractor = faster_ilp_cbc::FasterCbcExtractor;
        let unopt_dag_result =
            unopt_dag_extractor.extract(&unopt_dag_serialized, &unopt_dag_serialized.root_eclasses);
        unopt_dag_result.check(&unopt_dag_serialized);
        let unopt_dag_cost =
            unopt_dag_result.dag_cost(&unopt_dag_serialized, &unopt_dag_serialized.root_eclasses);

        // 2. compute optimized cost (with rewrites)
        let tree_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // reuse the analysis struct
            .with_expr(&expr)
            .run(&rules());
        let tree_costfn = MathCostFn::new(tree_runner.egraph.clone(), "cost_model.json");
        let tree_extractor = Extractor::new(&tree_runner.egraph, tree_costfn);
        let (best_tree_cost, best_tree_expr) = tree_extractor.find_best(tree_runner.egraph.find(tree_runner.roots[0]));

        let dag_runner: Runner<Math, TypeAnalysis> = Runner::new(analysis.clone()) // clone the analysis so we can reuse it
            .with_explanations_enabled()
            .with_expr(&expr)
            .run(&rules());
        // for its in &dag_runner.iterations {
        //     println!("{:?}", dag_runner.roots);
        //     println!("{:?}", dag_runner.egraph.find(dag_runner.roots[0]));
        //     println!("{:?}", its.applied);
        // }
        // println!(
        //     "DAG Runner stopped after {} iterations, reason: {:?}",
        //     dag_runner.iterations.len(),
        //     dag_runner.stop_reason
        // );
        // println!("{}", dag_runner.egraph.dot());
        let dag_costfn = MathCostFn::new(dag_runner.egraph.clone(), "cost_model.json");
        let mut dag_serialized =
            egg_to_serialized_egraph::<Math, _>(&dag_runner.egraph, dag_costfn);
        dag_serialized
            .root_eclasses
            .push(ClassId::from(format!("{}", dag_runner.egraph.find(dag_runner.roots[0]))));
        let dag_extractor = faster_ilp_cbc::FasterCbcExtractor;
        // println!("{:?}", dag_serialized);
        let dag_result = dag_extractor.extract(&dag_serialized, &dag_serialized.root_eclasses);
        dag_result.check(&dag_serialized);
        // let tree = extraction_result.tree_cost(&serialized, &serialized.root_eclasses);
        let best_dag_cost = dag_result.dag_cost(&dag_serialized, &dag_serialized.root_eclasses);
        let best_dag_expr = dag_result
            .dag_extracted_exprs(&dag_serialized, &dag_serialized.root_eclasses)[0]
            .clone();

        println!(">>>");
        println!("Input expr           : {}\n", line);
        println!("Tree: Initial cost   : {}",   unopt_tree_cost);
        println!("Tree: Optimized expr : {}",   best_tree_expr);
        println!("Tree: Optimized cost : {}\n", best_tree_cost);
        println!("DAG:  Initial cost   : {}",   unopt_dag_cost);
        println!("DAG:  Optimized expr : {}",   best_dag_expr);
        println!("DAG:  Optimized cost : {}",   best_dag_cost);
        println!("<<<");
    }
}

pub fn egg_to_serialized_egraph<L, A>(
    egraph: &EGraph<Math, A>,
    mut costfn: MathCostFn,
) -> egraph_serialize::EGraph
where
    A: Analysis<Math>,
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
