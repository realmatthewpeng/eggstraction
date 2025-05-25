use egg::{Analysis, EGraph, Id, DidMerge};
use std::collections::HashMap;
use crate::language::Math;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Fp,
    Ext2,
}

// Manual Deserialize so that JSON keys "fp" and "ext2" map to our Type
impl<'de> serde::Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "fp"   => Ok(Type::Fp),
            "ext2" => Ok(Type::Ext2),
            other  => Err(serde::de::Error::unknown_variant(other, &["fp","ext2"])),
        }
    }
}

#[derive(Clone)]
pub struct TypeAnalysis {
    /// user-provided map from symbol name → its field type
    symbol_types: HashMap<String, Type>,
}

impl TypeAnalysis {
    pub fn new(symbol_types: HashMap<String, Type>) -> Self {
        TypeAnalysis { symbol_types }
    }
}

impl Analysis<Math> for TypeAnalysis {
    type Data = Type;

    fn make(egraph: &mut EGraph<Math, TypeAnalysis>, enode: &Math) -> Type {
        // Pair(fp, fp) → Ext2
        if let Math::Pair([a, b]) = enode {
            if egraph[*a].data == Type::Fp && egraph[*b].data == Type::Fp {
                return Type::Ext2;
            }
        }
        // fst/snd(Ext2) → fp
        if let Math::Fst(id) | Math::Snd(id) = enode {
            if egraph[*id].data == Type::Ext2 {
                return Type::Fp;
            }
        }
        // ring ops / sq / inv just propagate the type of their first child
        match enode {
            Math::Add([a,b])
            | Math::Sub([a,b])
            | Math::Mul([a,b]) => {
                let ta = egraph[*a].data.clone();
                let tb = egraph[*b].data.clone();
                if ta == Type::Ext2 || tb == Type::Ext2 {
                    return Type::Ext2;
                } else {
                    return Type::Fp;
                }
            }

            Math::Inv(x) | Math::Sq(x) => {
                return egraph[*x].data.clone();
            }

            Math::Constant(_) => return Type::Fp,

            Math::Symbol(sym) => {
                let name = sym.as_str().to_string();
                return egraph
                    .analysis
                    .symbol_types
                    .get(&name)
                    .cloned()
                    .unwrap_or(Type::Fp);
            }

            _ => {}
        }
        // fallback
        Type::Fp
    }

    // must return DidMerge, not bool
    fn merge(&mut self, to: &mut Type, from: Type) -> DidMerge {
        if *to != from {
            *to = if *to == Type::Ext2 || from == Type::Ext2 {
                Type::Ext2
            } else {
                Type::Fp
            };
            DidMerge(true, true)
        } else {
            DidMerge(false, false)
        }
    }

    // signature has no &self
    fn modify(_egraph: &mut EGraph<Math, TypeAnalysis>, _id: Id) {
        // no changes needed
    }
}


impl Default for TypeAnalysis {
    fn default() -> Self {
        TypeAnalysis {
            symbol_types: std::collections::HashMap::new(),
        }
    }
}