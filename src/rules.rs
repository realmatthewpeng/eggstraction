use egg::{rewrite as rw};
use crate::language::Math;
pub type Rewrite = egg::Rewrite<Math, ()>;

pub fn rules() -> Vec<Rewrite> {
    vec![
        rw!("comm-add";  "(+ ?a ?b)" => "(+ ?b ?a)"),
        rw!("comm-mul";  "(* ?a ?b)" => "(* ?b ?a)"),
        rw!("mul-2";     "(* 2 ?a)"  => "(+ ?a ?a)"),
    ]
}