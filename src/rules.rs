use crate::analysis::TypeAnalysis;
use crate::language::Math;
use egg::{Rewrite, rewrite as rw};

// Note: we’re now returning Rewrite<Math,TypeAnalysis>
pub fn rules() -> Vec<Rewrite<Math, TypeAnalysis>> {
    vec![
        // commutativity
        rw!("comm-add";     "(+ ?a ?b)"    => "(+ ?b ?a)"),
        rw!("comm-mul";     "(* ?a ?b)"    => "(* ?b ?a)"),

        // associativity
        rw!("assoc-mul";    "(* ?a (* ?b ?c))"     => "(* (* ?a ?b) ?c)"),
        rw!("assoc-add";    "(+ ?a (+ ?b ?c))"     => "(+ (+ ?a ?b) ?c)"),

        // squaring
        rw!("mul-to-sq";    "(* ?x ?x)"     => "(sq ?x)"),
        rw!("sq-to-mul";    "(sq ?x)"       => "(* ?x ?x)"),

        // addition, subtraction
        rw!("sub";          "(- (+ ?a ?b) ?b)"      => "?a"),
        rw!("add-same";     "(+ ?a ?a)"             => "(* 2 ?a)"),

        // // distributivity
        rw!("dist-left";        "(* ?a (+ ?b ?c))"          => "(+ (* ?a ?b) (* ?a ?c))"),
        rw!("dist-right-add";   "(+ (* ?a ?c) (* ?b ?c))"   => "(* (+ ?a ?b) ?c)"),
        rw!("dist-right-sub";   "(- (* ?a ?c) (* ?b ?c))"   => "(* (- ?a ?b) ?c)"),
        
        // // binomial
        rw!("binomial";         "(sq (+ ?a ?b))"    => "(+ (+ (sq ?a) (* 2 (* ?a ?b))) (sq ?b))"),
        rw!("mul2-binomial";    "(* 2 (* ?a ?b))"   => "(- (- (sq (+ ?a ?b)) (sq ?a)) (sq ?b))"),
    ]
}
