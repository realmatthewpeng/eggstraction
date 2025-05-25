use egg::{rewrite as rw, *};
use ordered_float::NotNan;

pub type EGraph = egg::EGraph<Math, ()>;
pub type Rewrite = egg::Rewrite<Math, ()>;

pub type Constant = NotNan<f64>;

define_language! {
    pub enum Math {
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "inv" = Inv(Id),

        Constant(Constant),
        Symbol(Symbol),
    }
}

pub struct MathCostFn<'a> {
    egraph: &'a egg::EGraph<Math, ()>,
}

impl<'a> egg::CostFunction<Math> for MathCostFn<'a> {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &Math, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            Math::Inv(..) => 80,
            Math::Mul([a,b]) => {
                if id_is_sym(self.egraph, *a) && id_is_sym(self.egraph, *b) {
                    10
                } else if id_is_const(self.egraph, *a) && id_is_sym(self.egraph, *b) {
                    4
                } else {
                    panic!("Unexpected case: Mul with non-symbol and non-constant operands");
                }
            }
            Math::Add(..) => 1,
            Math::Sub(..) => 1,
            _ => 0,
        };
        enode.fold(op_cost, |sum, i| sum + costs(i))
    }

}

fn id_is_const(egraph: &EGraph, id: Id) -> bool {
    egraph[id]
        .nodes
        .iter()
        .any(|n| matches!(n, Math::Constant(..)))
}

fn id_is_sym(egraph: &EGraph, id: Id) -> bool {
    egraph[id]
        .nodes
        .iter()
        .any(|n| matches!(n, Math::Symbol(..)))
}

fn is_const(var: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    let var = var.parse().unwrap();
    move |egraph, _, subst| {
        egraph[subst[var]]
            .nodes
            .iter()
            .any(|n| matches!(n, Math::Constant(..)))
    }
}

fn is_sym(var: &str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    let var = var.parse().unwrap();
    move |egraph, _, subst| {
        egraph[subst[var]]
            .nodes
            .iter()
            .any(|n| matches!(n, Math::Symbol(..)))
    }
}

pub fn rules() -> Vec<Rewrite> { vec![
    rw!("comm-add";  "(+ ?a ?b)"        => "(+ ?b ?a)"),
    rw!("comm-mul";  "(* ?a ?b)"        => "(* ?b ?a)"),
    rw!("mul-2";     "(* 2 ?a)"         => "(+ ?a ?a)"),
]}

fn test1() {
    let expr: RecExpr<Math> = "(* x x)".parse().unwrap();
    let mut runner: Runner<Math, ()> = Runner::default()
        .with_expr(&expr)
        .run(&rules());
    let costfn = MathCostFn { egraph: &runner.egraph };
    let extractor = Extractor::new(&runner.egraph, costfn);
    let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);
    println!("Orig expr    : {}", expr);
    println!("Best cost    : {}", best_cost);
    println!("Best expr    : {}", best_expr);
}

fn main() {
    test1();
}
