use crate::analysis::TypeAnalysis;
use crate::language::Math;
use egg::{Rewrite, rewrite as rw};

// Note: we’re now returning Rewrite<Math,TypeAnalysis>
pub fn rules() -> Vec<Rewrite<Math, TypeAnalysis>> {
    vec![
        // commutativity
        rw!("comm-add";  "(+ ?a ?b)"        => "(+ ?b ?a)"),
        rw!("comm-mul";  "(* ?a ?b)"        => "(* ?b ?a)"),
        // associativity
        rw!("assoc-add"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
        rw!("assoc-mul"; "(* ?a (* ?b ?c))" => "(* (* ?a ?b) ?c)"),
        // distributivity
        rw!("dist-left";  "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
        rw!("dist-right"; "(+ (* ?a ?c) (* ?b ?c))" => "(* (+ ?a ?b) ?c)"),
        // square desugaring & binomial
        rw!("sq-to-mul";   "(sq ?x)"            => "(* ?x ?x)"),
        rw!("binomial";    "(sq (+ ?a ?b))"     => "(+ (+ (sq ?a) (* 2 (* ?a ?b))) (sq ?b))"),
        rw!("mul-to-sq"; "(* ?x ?x)" => "(sq ?x)"),
        rw!("mul2-binomial";"(* 2 (* ?a ?b))" => "(- (- (sq (+ ?a ?b)) (sq ?a)) (sq ?b))"),
        rw!("binomial-contract";"(+ (+ (sq ?a) (* 2 (* ?a ?b))) (sq ?b))" =>"(sq (+ ?a ?b))"),
        // pair/proj
        rw!("fst-pair";    "(fst (pair ?x ?y))" => "?x"),
        rw!("snd-pair";    "(snd (pair ?x ?y))" => "?y"),
    ]
}
