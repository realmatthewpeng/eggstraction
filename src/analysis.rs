use crate::language::Math;
use egg::{Analysis, DidMerge, EGraph, Id};
use std::collections::HashMap;
use serde::{Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum FieldType {
    /// Base field (degree 1)
    Fp,
    /// Extension field of degree n over Fp
    /// Fp2, Fp4, Fp8, etc.
    FpExt(u32),
}

impl FieldType {
    /// Get the degree of the field extension
    pub fn degree(&self) -> u32 {
        match self {
            FieldType::Fp => 1,
            FieldType::FpExt(n) => *n,
        }
    }

    /// Check if this field can contain elements from another field
    pub fn contains(&self, other: &FieldType) -> bool {
        match (self, other) {
            (FieldType::Fp, FieldType::Fp) => true,
            (FieldType::FpExt(_), FieldType::Fp) => true,
            (FieldType::FpExt(n1), FieldType::FpExt(n2)) => (*n1 >= *n2) && (n1 % n2 == 0),
            _ => false,
        }
    }

    /// Get the least common extension that contains both fields
    pub fn lcm_extension(&self, other: &FieldType) -> FieldType {
        match (self, other) {
            (FieldType::Fp, FieldType::Fp) => FieldType::Fp,
            (FieldType::Fp, FieldType::FpExt(n)) | (FieldType::FpExt(n), FieldType::Fp) => {
                FieldType::FpExt(*n)
            }
            (FieldType::FpExt(n1), FieldType::FpExt(n2)) => {
                let raw = lcm(*n1, *n2);
                FieldType::FpExt(raw)
            }
        }
    }

    /// Parse from string (e.g., "fp", "fp2", "fp4", etc.)
    pub fn from_str(s: &str) -> Result<Self, String> {
        if s == "fp" {
            Ok(FieldType::Fp)
        } else if s.starts_with("fp") {
            let degree_str = &s[2..];
            if degree_str.is_empty() {
                return Err(format!("Invalid field type: {}", s));
            }
            let degree: u32 = degree_str.parse()
                .map_err(|_| format!("Invalid degree in field type: {}", s))?;
            if degree == 1 {
                Ok(FieldType::Fp)
            } else if degree.is_power_of_two() {
                Ok(FieldType::FpExt(degree))
            } else {
                Err(format!("Field degree must be a power of 2: {}", degree))
            }
        } else {
            Err(format!("Unknown field type: {}", s))
        }
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            FieldType::Fp => "fp".to_string(),
            FieldType::FpExt(n) => format!("fp{}", n),
        }
    }
}

// Helper function to compute LCM
fn lcm(a: u32, b: u32) -> u32 {
    a * b / gcd(a, b)
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

// Custom Deserialize implementation for backward compatibility
impl<'de> serde::Deserialize<'de> for FieldType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FieldType::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone)]
pub struct TypeAnalysis {
    /// User‐provided map: symbol → its field type
    pub symbol_types: HashMap<String, FieldType>,
    /// Upper bound on extension degree (optional clamp)
    pub max_degree: u32,
}

impl TypeAnalysis {
    pub fn new(symbol_types: HashMap<String, FieldType>) -> Self {
        let max_degree = symbol_types
            .values()
            .map(|t| t.degree())
            .max()
            .unwrap_or(256);
        TypeAnalysis {
            symbol_types,
            max_degree,
        }
    }

    /// If you explicitly want to clamp all LCMs at a certain maximum:
    pub fn with_max_degree(mut self, max_degree: u32) -> Self {
        self.max_degree = max_degree;
        self
    }

    /// Determine the result type of a pair operation
    fn pair_result_type(&self, a: &FieldType, b: &FieldType) -> FieldType {
        // For now, pair(Fp, Fp) -> Fp2, pair(Fp2, Fp2) -> Fp4, etc.
        // This assumes we're constructing extension fields by pairing
        match (a, b) {
            (FieldType::Fp, FieldType::Fp) => FieldType::FpExt(2),
            (FieldType::FpExt(n1), FieldType::FpExt(n2)) if n1 == n2 => {
                let new_degree = n1 * 2;
                if new_degree <= self.max_degree {
                    FieldType::FpExt(new_degree)
                } else {
                    // If we exceed max degree, stay in the LCM extension
                    a.lcm_extension(b)
                }
            }
            _ => a.lcm_extension(b),
        }
    }

    /// Determine if a field operation should promote to a larger field
    fn operation_result_type(&self, args: &[&FieldType]) -> FieldType {
        if args.is_empty() {
            return FieldType::Fp;
        }

        // Start by folding all children’s types via lcm_extension
        let mut acc = args[0].clone().clone();
        for t in args.iter().skip(1) {
            acc = acc.lcm_extension(t);
        }

        // Optionally clamp to max_degree:
        match &acc {
            FieldType::Fp => FieldType::Fp,
            FieldType::FpExt(d) => {
                let raw = *d;
                let clamped = u32::min(raw, self.max_degree);
                // If you want the “next power‐of‐two below max_degree,” you'd need
                // additional logic here. In this template we do a hard clamp.
                if clamped == 1 {
                    FieldType::Fp
                } else {
                    FieldType::FpExt(clamped)
                }
            }
        }
    }
}

impl Analysis<Math> for TypeAnalysis {
    type Data = FieldType;

    fn make(egraph: &mut EGraph<Math, TypeAnalysis>, enode: &Math) -> FieldType {
        match enode {

            // Pair operations create extension fields
            Math::Pair([a, b]) => {
                let type_a = &egraph[*a].data;
                let type_b = &egraph[*b].data;
                egraph.analysis.pair_result_type(type_a, type_b)
            }

            // First/second projections from extension fields
            Math::Fst(id) | Math::Snd(id) => {
                let input_type = &egraph[*id].data;
                match input_type {
                    FieldType::FpExt(n) if *n > 1 => {
                        // Project to a smaller field (half the degree)
                        let new_degree = n / 2;
                        if new_degree == 1 {
                            FieldType::Fp
                        } else {
                            FieldType::FpExt(new_degree)
                        }
                    }
                    _ => FieldType::Fp, // Default projection
                }
            }
            
            // Binary ops: take the LCM of operand types
            Math::Add([a, b]) | Math::Sub([a, b]) | Math::Mul([a, b]) => {
                let t1 = &egraph[*a].data;
                let t2 = &egraph[*b].data;
                egraph.analysis.operation_result_type(&[t1, t2])
            }

            // Unary ops: preserve the child’s type
            Math::Inv(x) | Math::Sq(x) => egraph[*x].data.clone(),

            // Constants always live in base field
            Math::Constant(_) => FieldType::Fp,

            // A symbol’s type comes from user‐provided “symbol_types”
            Math::Symbol(sym) => {
                let name = sym.as_str().to_string();
                egraph.analysis
                    .symbol_types
                    .get(&name)
                    .cloned()
                    .unwrap_or(FieldType::Fp)
            }
        }
    }

    fn merge(&mut self, to: &mut FieldType, from: FieldType) -> DidMerge {
        if *to != from {
            // take LCM
            let mut new_ty = to.lcm_extension(&from);
            // clamp if needed:
            if let FieldType::FpExt(d) = &new_ty {
                if *d > self.max_degree {
                    new_ty = FieldType::FpExt(self.max_degree);
                }
            }
            if new_ty != *to {
                *to = new_ty;
                DidMerge(true, true)
            } else {
                DidMerge(false, false)
            }
        } else {
            DidMerge(false, false)
        }
    }

    fn modify(_egraph: &mut EGraph<Math, TypeAnalysis>, _id: Id) {
        // no extra “post‐merge” rewriting
    }
}

impl Default for TypeAnalysis {
    fn default() -> Self {
        TypeAnalysis {
            symbol_types: HashMap::new(),
            max_degree: 8, // default upper bound
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_parsing() {
        assert_eq!(FieldType::from_str("fp").unwrap(), FieldType::Fp);
        assert_eq!(FieldType::from_str("fp2").unwrap(), FieldType::FpExt(2));
        assert_eq!(FieldType::from_str("fp4").unwrap(), FieldType::FpExt(4));
        assert_eq!(FieldType::from_str("fp8").unwrap(), FieldType::FpExt(8));

        assert!(FieldType::from_str("fp3").is_err()); // Not power of 2
        assert!(FieldType::from_str("invalid").is_err());
    }

    #[test]
    fn test_field_containment() {
        let fp = FieldType::Fp;
        let fp2 = FieldType::FpExt(2);
        let fp4 = FieldType::FpExt(4);
        let fp8 = FieldType::FpExt(8);

        assert!(fp2.contains(&fp));
        assert!(fp4.contains(&fp));
        assert!(fp4.contains(&fp2));
        assert!(fp8.contains(&fp4));
        assert!(!fp2.contains(&fp4));
    }

    #[test]
    fn test_lcm_extension() {
        let fp = FieldType::Fp;
        let fp2 = FieldType::FpExt(2);
        let fp4 = FieldType::FpExt(4);

        assert_eq!(fp.lcm_extension(&fp2), fp2);
        assert_eq!(fp2.lcm_extension(&fp4), fp4);
        assert_eq!(fp2.lcm_extension(&FieldType::FpExt(8)), FieldType::FpExt(8));
    }
}
