use super::model::{RoadmapDag, TopoOrder};

impl RoadmapDag {
    /// Kahn's algorithm over the dependency edges (`depends_on` ∪ `blocks`).
    /// Returns [`TopoOrder::Ordered`] (prereqs first) or [`TopoOrder::Cycle`]
    /// with the residual nodes when a cycle makes a full ordering impossible.
    ///
    /// ALGO: Kahn topological sort | `PROBLEM_CLASS`: graph DAG validation
    /// TIME: O(n + e) | SPACE: O(n + e) | YEAR: 2026
    /// SOURCE: <https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm>
    #[must_use]
    pub fn toposort_or_cycle(&self) -> TopoOrder {
        use std::collections::{HashMap, VecDeque};
        let mut in_deg: HashMap<&str, usize> =
            self.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in &self.edges {
            if e.rel != "depends_on" && e.rel != "blocks" {
                continue;
            }
            if !in_deg.contains_key(e.source.as_str()) || !in_deg.contains_key(e.target.as_str()) {
                continue;
            }
            adj.entry(e.source.as_str())
                .or_default()
                .push(e.target.as_str());
            let d = in_deg.entry(e.target.as_str()).or_insert(0);
            *d = d.saturating_add(1);
        }
        let mut queue: VecDeque<&str> = in_deg
            .iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(&k, _)| k)
            .collect();
        let mut ordered: Vec<String> = Vec::with_capacity(self.nodes.len());
        while let Some(n) = queue.pop_front() {
            ordered.push(n.to_owned());
            if let Some(succ) = adj.get(n) {
                for &m in succ {
                    if let Some(d) = in_deg.get_mut(m) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(m);
                        }
                    }
                }
            }
        }
        if ordered.len() == self.nodes.len() {
            TopoOrder::Ordered(ordered)
        } else {
            let cycle = in_deg
                .iter()
                .filter(|&(_, &d)| d > 0)
                .map(|(&k, _)| k.to_owned())
                .collect();
            TopoOrder::Cycle(cycle)
        }
    }
}
