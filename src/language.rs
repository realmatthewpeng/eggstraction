use egg::{*};
use ordered_float::NotNan;

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

pub type EGraph = egg::EGraph<Math, ()>;