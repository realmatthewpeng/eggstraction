use crate::analysis::TypeAnalysis;
use crate::language::Math;
use egg::{Rewrite, rewrite as rw, EGraph, Id, Subst};

fn is_not_same(a: &str, b: &str) -> impl Fn(&mut EGraph<Math, TypeAnalysis>, Id, &Subst) -> bool {
    let a = a.parse().unwrap();
    let b = b.parse().unwrap();
    move |egraph, _, subst| {
        // println!("Checking if {} != {}: {}", egraph.find(subst[a]), egraph.find(subst[b]), egraph.find(subst[a]) != egraph.find(subst[b]));
        // for node in &egraph[subst[a]].nodes {
        //     println!("Node in e-class for a: {:?}", node);
        // }
        // for node in &egraph[subst[b]].nodes {
        //     println!("Node in e-class for b: {:?}", node);
        // }
        egraph.find(subst[a]) != egraph.find(subst[b])
    }
}

// Rules are actually automatically bidirectional once triggered by lhs
pub fn rules() -> Vec<Rewrite<Math, TypeAnalysis>> {
    vec![
        // commutativity
        rw!("comm-add";     "(+ ?a ?b)"    => "(+ ?b ?a)"),
        rw!("comm-mul";     "(* ?a ?b)"    => "(* ?b ?a)"),

        // associativity
        rw!("assoc-mul";    "(* ?a (* ?b ?c))"     => "(* (* ?a ?b) ?c)"),
        rw!("assoc-add";    "(+ ?a (+ ?b ?c))"     => "(+ (+ ?a ?b) ?c)"),

        // squaring
        
        //rw!("sq-to-mul";    "(sq ?x)"       => "(* ?x ?x)"),
        rw!("mul-to-sq";    "(* ?x ?x)"     => "(sq ?x)"),

        // addition, subtraction
        rw!("sub";          "(- (+ ?a ?b) ?b)"      => "?a"),
        rw!("add-same";     "(+ ?a ?a)"             => "(* 2 ?a)"),

        // distributivity
        rw!("dist-left";        "(* ?a (+ ?b ?c))"          => "(+ (* ?a ?b) (* ?a ?c))" if is_not_same("?b", "?c")),
        rw!("dist-right-add";   "(+ (* ?a ?c) (* ?b ?c))"   => "(* (+ ?a ?b) ?c)" if is_not_same("?a", "?b")),
        rw!("dist-right-sub";   "(- (* ?a ?c) (* ?b ?c))"   => "(* (- ?a ?b) ?c)"),
        
        // binomial
        //rw!("binomial";         "(sq (+ ?a ?b))"    => "(+ (+ (sq ?a) (* 2 (* ?a ?b))) (sq ?b))"), // Conflicts w/ mul2-binomial
        rw!("mul2-binomial";    "(* 2 (* ?a ?b))"   => "(- (- (sq (+ ?a ?b)) (sq ?a)) (sq ?b))"),
    ]
}
