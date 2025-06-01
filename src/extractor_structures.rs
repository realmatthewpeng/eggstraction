#![allow(dead_code)]

use egraph_serialize::*;
use indexmap::IndexMap;
use ordered_float::NotNan;

use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashMap;
pub const INFINITY: Cost = unsafe { NotNan::new_unchecked(std::f64::INFINITY) };

/** A data structure to maintain a queue of unique elements.

Notably, insert/pop operations have O(1) expected amortized runtime complexity.

Thanks @Bastacyclop for the implementation!
*/
#[derive(Clone)]
pub(crate) struct UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    set: FxHashSet<T>, // hashbrown::
    queue: std::collections::VecDeque<T>,
}

impl<T> Default for UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    fn default() -> Self {
        UniqueQueue {
            set: Default::default(),
            queue: std::collections::VecDeque::new(),
        }
    }
}

impl<T> UniqueQueue<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    pub fn insert(&mut self, t: T) {
        if self.set.insert(t.clone()) {
            self.queue.push_back(t);
        }
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for t in iter.into_iter() {
            self.insert(t);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let res = self.queue.pop_front();
        res.as_ref().map(|t| self.set.remove(t));
        res
    }
    
    pub fn is_empty(&self) -> bool {
        let r = self.queue.is_empty();
        debug_assert_eq!(r, self.set.is_empty());
        r
    }
}

pub trait Extractor: Sync {
    fn extract(&self, egraph: &EGraph, roots: &[ClassId]) -> ExtractionResult;

    fn boxed(self) -> Box<dyn Extractor>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

pub trait MapGet<K, V> {
    fn get(&self, key: &K) -> Option<&V>;
}

impl<K, V> MapGet<K, V> for HashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        HashMap::get(self, key)
    }
}

impl<K, V> MapGet<K, V> for FxHashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        FxHashMap::get(self, key)
    }
}

impl<K, V> MapGet<K, V> for IndexMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    fn get(&self, key: &K) -> Option<&V> {
        IndexMap::get(self, key)
    }
}

#[derive(Default, Clone)]
pub struct ExtractionResult {
    pub choices: IndexMap<ClassId, NodeId>,
}

#[derive(Clone, Copy)]
enum Status {
    Doing,
    Done,
}

impl ExtractionResult {
    pub fn check(&self, egraph: &EGraph) {
        // should be a root
        assert!(!egraph.root_eclasses.is_empty());

        // All roots should be selected.
        for cid in egraph.root_eclasses.iter() {
            assert!(self.choices.contains_key(cid));
        }

        // No cycles
        assert!(self.find_cycles(&egraph, &egraph.root_eclasses).is_empty());

        // Nodes should match the class they are selected into.
        for (cid, nid) in &self.choices {
            let node = &egraph[nid];
            assert!(node.eclass == *cid);
        }

        // All the nodes the roots depend upon should be selected.
        let mut todo: Vec<ClassId> = egraph.root_eclasses.to_vec();
        let mut visited: FxHashSet<ClassId> = Default::default();
        while let Some(cid) = todo.pop() {
            if !visited.insert(cid.clone()) {
                continue;
            }
            assert!(self.choices.contains_key(&cid));

            for child in &egraph[&self.choices[&cid]].children {
                todo.push(egraph.nid_to_cid(child).clone());
            }
        }
    }

    pub fn choose(&mut self, class_id: ClassId, node_id: NodeId) {
        self.choices.insert(class_id, node_id);
    }

    pub fn find_cycles(&self, egraph: &EGraph, roots: &[ClassId]) -> Vec<ClassId> {
        // let mut status = vec![Status::Todo; egraph.classes().len()];
        let mut status = IndexMap::<ClassId, Status>::default();
        let mut cycles = vec![];
        for root in roots {
            // let root_index = egraph.classes().get_index_of(root).unwrap();
            self.cycle_dfs(egraph, root, &mut status, &mut cycles)
        }
        cycles
    }

    fn cycle_dfs(
        &self,
        egraph: &EGraph,
        class_id: &ClassId,
        status: &mut IndexMap<ClassId, Status>,
        cycles: &mut Vec<ClassId>,
    ) {
        match status.get(class_id).cloned() {
            Some(Status::Done) => (),
            Some(Status::Doing) => cycles.push(class_id.clone()),
            None => {
                status.insert(class_id.clone(), Status::Doing);
                let node_id = &self.choices[class_id];
                let node = &egraph[node_id];
                for child in &node.children {
                    let child_cid = egraph.nid_to_cid(child);
                    self.cycle_dfs(egraph, child_cid, status, cycles)
                }
                status.insert(class_id.clone(), Status::Done);
            }
        }
    }

    pub fn tree_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> Cost {
        let node_roots = roots
            .iter()
            .map(|cid| self.choices[cid].clone())
            .collect::<Vec<NodeId>>();
        self.tree_cost_rec(egraph, &node_roots, &mut HashMap::new())
    }

    fn tree_cost_rec(
        &self,
        egraph: &EGraph,
        roots: &[NodeId],
        memo: &mut HashMap<NodeId, Cost>,
    ) -> Cost {
        let mut cost = Cost::default();
        for root in roots {
            if let Some(c) = memo.get(root) {
                cost += *c;
                continue;
            }
            let class = egraph.nid_to_cid(root);
            let node = &egraph[&self.choices[class]];
            let inner = node.cost + self.tree_cost_rec(egraph, &node.children, memo);
            memo.insert(root.clone(), inner);
            cost += inner;
        }
        cost
    }

    // this will loop if there are cycles
    pub fn dag_cost(&self, egraph: &EGraph, roots: &[ClassId]) -> Cost {
        let mut costs: IndexMap<ClassId, Cost> = IndexMap::new();
        let mut todo: Vec<ClassId> = roots.to_vec();
        while let Some(cid) = todo.pop() {
            let node_id = &self.choices[&cid];
            let node = &egraph[node_id];
            if costs.insert(cid.clone(), node.cost).is_some() {
                continue;
            }
            for child in &node.children {
                todo.push(egraph.nid_to_cid(child).clone());
            }
        }
        costs.values().sum()
    }

    pub fn dag_extracted_exprs(&self, egraph: &EGraph, roots: &[ClassId]) -> Vec<String> {
        use std::collections::HashSet;
        let mut exprs = Vec::new();

        fn build_expr(
            cid: &ClassId,
            egraph: &EGraph,
            result: &ExtractionResult,
            seen: &mut HashSet<ClassId>,
        ) -> String {
            // Check if we have already seen this class ID to avoid infinite recursion
            // if !seen.insert(cid.clone()) {
            //     // Already visited this class (shared subexpr), just use its symbol
            //     return format!("${}", cid);
            // }
            let node_id = &result.choices[cid];
            let node = &egraph[node_id];
            if node.children.is_empty() {
                // Leaf node: print its value
                format!("{}", node.op)
            } else {
                let children_exprs: Vec<String> = node
                    .children
                    .iter()
                    .map(|child| {
                        let child_cid = egraph.nid_to_cid(child);
                        build_expr(child_cid, egraph, result, seen)
                    })
                    .collect();
                // Print as S-expression: (op child1 child2 ...)
                format!("({} {})", node.op, children_exprs.join(" "))
            }
        }

        for root in roots {
            let mut seen = HashSet::new();
            exprs.push(build_expr(root, egraph, self, &mut seen));
        }
        exprs
    }

    pub fn node_sum_cost<M>(&self, egraph: &EGraph, node: &Node, costs: &M) -> Cost
    where
        M: MapGet<ClassId, Cost>,
    {
        node.cost
            + node
                .children
                .iter()
                .map(|n| {
                    let cid = egraph.nid_to_cid(n);
                    costs.get(cid).unwrap_or(&INFINITY)
                })
                .sum::<Cost>()
    }
}
