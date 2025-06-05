use crate::analysis::{TypeAnalysis};
use crate::language::Math;
use egg::{Rewrite, rewrite as rw, EGraph, Id, Subst};

fn is_not_same(a: &str, b: &str) -> impl Fn(&mut EGraph<Math, TypeAnalysis>, Id, &Subst) -> bool {
    let a = a.parse().unwrap();
    let b = b.parse().unwrap();
    move |egraph, _, subst| {
        egraph.find(subst[a]) != egraph.find(subst[b])
    }
}

fn is_same_field(a: &str, b: &str) -> impl Fn(&mut EGraph<Math, TypeAnalysis>, Id, &Subst) -> bool {
    let a = a.parse().unwrap();
    let b = b.parse().unwrap();
    move |egraph, _, subst| {
        let ta = &egraph[egraph.find(subst[a])].data;
        let tb = &egraph[egraph.find(subst[b])].data;
        *ta == *tb
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
        rw!("sq-to-mul";    "(sq ?x)"       => "(* ?x ?x)"),
        rw!("mul-to-sq";    "(* ?x ?x)"     => "(sq ?x)"),

        // addition, subtraction
        rw!("sub";          "(- (+ ?a ?b) ?b)"      => "?a"),
        rw!("add-same";     "(+ ?a ?a)"             => "(* 2 ?a)"),
        rw!("sub-same";     "(- ?a ?a)"             => "0"),

        // distributivity
        rw!("dist-left";        "(* ?a (+ ?b ?c))"          => "(+ (* ?a ?b) (* ?a ?c))" if is_not_same("?b", "?c")),
        rw!("dist-right-add";   "(+ (* ?a ?c) (* ?b ?c))"   => "(* (+ ?a ?b) ?c)" if is_not_same("?a", "?b")),
        rw!("dist-right-sub";   "(- (* ?a ?c) (* ?b ?c))"   => "(* (- ?a ?b) ?c)" if is_not_same("?a", "?b")),
        
        // binomial
        rw!("binomial";         "(sq (+ ?a ?b))"    => "(+ (+ (sq ?a) (* 2 (* ?a ?b))) (sq ?b))"),

        // Benchmark 1
        rw!("benchmark1";   "(+ (* ?a ?b) (* ?c ?d))"   => "(- (- (* (+ ?a ?c) (+ ?d ?b)) (* ?a ?d)) (* ?c ?b))"
                                                            if is_not_same("?a", "?b")
                                                            if is_not_same("?a", "?c")
                                                            if is_not_same("?a", "?d")
                                                            if is_not_same("?b", "?c")
                                                            if is_not_same("?b", "?d") 
                                                            if is_not_same("?c", "?d")
                                                            if is_same_field("?a", "?c")
                                                            if is_same_field("?b", "?d")),

        // // Benchmark 2
        rw!("mul2-binomial";    "(* 2 (* ?a ?b))"   => "(- (- (sq (+ ?a ?b)) (sq ?a)) (sq ?b))"),

    ]
}

pub fn pair_rules() -> Vec<Rewrite<Math, TypeAnalysis>> {
    vec![

    rw!("pair-add";         "(+ (pair ?a ?b) (pair ?c ?d))"     =>  "(pair (+ ?a ?c) (+ ?b ?d))"),
    rw!("pair-sub";         "(- (pair ?a ?b) (pair ?c ?d))"     =>  "(pair (- ?a ?c) (- ?b ?d))"),
    rw!("pair-mul-const";   "(* (pair ?a ?b) ?c)"               =>  "(pair (* ?a ?c) (* ?b ?c))" if is_same_field("?c", "?a")),
    rw!("pair-sq";          "(sq (pair ?a ?b))"                 =>  "(* (pair ?a ?b) (pair ?a ?b))"),
    // (a+bU)*(c+dU) = (a*c + a*dU + bU*c + bU*dU)
    rw!("pair-mul";         "(* (pair ?a ?b) (pair ?c ?d))"     =>  "(pair (+ (* ?a ?c) (* (* ?b ?d) xi)) (+ (* ?a ?d) (* ?b ?c)))"),

    ]
}
