// Iterative Tarjan SCC — avoids stack overflow on deep dependency graphs.
// ALGO: Tarjan SCC (iterative)
// PROBLEM_CLASS: graph_traversal
// TIME: O(V+E) | SPACE: O(V+E)
// YEAR: 1972 | SEARCHED: 2026-04

const UNVISITED: usize = usize::MAX;

pub struct TarjanState {
    index: Vec<usize>,
    lowlink: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    counter: usize,
}

impl TarjanState {
    fn new(n: usize) -> Self {
        Self {
            index: (0..n).map(|_| UNVISITED).collect(),
            lowlink: (0..n).map(|_| 0usize).collect(),
            on_stack: (0..n).map(|_| false).collect(),
            stack: Vec::with_capacity(n),
            counter: 0,
        }
    }

    fn visit(&mut self, v: usize) {
        if let Some(idx) = self.index.get_mut(v) {
            *idx = self.counter;
        }
        if let Some(ll) = self.lowlink.get_mut(v) {
            *ll = self.counter;
        }
        self.counter += 1;
        if let Some(os) = self.on_stack.get_mut(v) {
            *os = true;
        }
        self.stack.push(v);
    }

    fn is_unvisited(&self, v: usize) -> bool {
        match self.index.get(v) {
            Some(&i) => i == UNVISITED,
            None => true,
        }
    }

    fn is_on_stack(&self, v: usize) -> bool {
        match self.on_stack.get(v) {
            Some(&b) => b,
            None => false,
        }
    }

    fn index_of(&self, v: usize) -> usize {
        match self.index.get(v) {
            Some(&i) => i,
            None => 0,
        }
    }

    fn lowlink_of(&self, v: usize) -> usize {
        match self.lowlink.get(v) {
            Some(&ll) => ll,
            None => 0,
        }
    }

    fn update_lowlink(&mut self, v: usize, val: usize) {
        if let Some(ll) = self.lowlink.get_mut(v) {
            *ll = (*ll).min(val);
        }
    }

    fn pop_scc(&mut self, root: usize) -> Vec<usize> {
        let mut scc = Vec::new();
        while let Some(w) = self.stack.pop() {
            if let Some(os) = self.on_stack.get_mut(w) {
                *os = false;
            }
            scc.push(w);
            if w == root {
                break;
            }
        }
        scc
    }
}

/// Find all strongly-connected components with at least 2 nodes (circular dependency clusters).
/// Parameter `adj` is a slice where each entry lists the dependency indices of that node.
pub fn find_cycles(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let mut state = TarjanState::new(n);
    let mut sccs: Vec<Vec<usize>> = Vec::new();
    let mut call_stack: Vec<(usize, usize)> = Vec::with_capacity(n);

    for start in 0..n {
        if !state.is_unvisited(start) {
            continue;
        }
        call_stack.push((start, 0));
        while let Some(frame) = call_stack.last_mut() {
            let v = frame.0;
            if state.is_unvisited(v) {
                state.visit(v);
            }
            let neighbors: &[usize] = match adj.get(v) {
                Some(nb) => nb.as_slice(),
                None => &[],
            };
            let wi = frame.1;
            if wi < neighbors.len() {
                frame.1 += 1;
                let w = match neighbors.get(wi) {
                    Some(&w) => w,
                    None => continue,
                };
                if state.is_unvisited(w) {
                    call_stack.push((w, 0));
                } else if state.is_on_stack(w) {
                    state.update_lowlink(v, state.index_of(w));
                }
            } else {
                call_stack.pop();
                if let Some(parent_frame) = call_stack.last() {
                    let parent = parent_frame.0;
                    state.update_lowlink(parent, state.lowlink_of(v));
                }
                if state.lowlink_of(v) == state.index_of(v) {
                    let scc = state.pop_scc(v);
                    if scc.len() >= 2 {
                        sccs.push(scc);
                    }
                }
            }
        }
    }
    sccs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_find_cycle_when_two_nodes_mutually_depend() {
        // 0 → 1 → 0
        let mut adj: Vec<Vec<usize>> = (0..2).map(|_| Vec::new()).collect();
        if let Some(v) = adj.get_mut(0) { v.push(1); }
        if let Some(v) = adj.get_mut(1) { v.push(0); }
        let sccs = find_cycles(&adj);
        assert_eq!(sccs.len(), 1);
        if let Some(scc) = sccs.into_iter().next() {
            assert_eq!(scc.len(), 2);
            assert!(scc.contains(&0));
            assert!(scc.contains(&1));
        }
    }

    #[test]
    fn should_return_empty_when_no_cycles() {
        // 0 → 1 → 2 (DAG)
        let mut adj: Vec<Vec<usize>> = (0..3).map(|_| Vec::new()).collect();
        if let Some(v) = adj.get_mut(0) { v.push(1); }
        if let Some(v) = adj.get_mut(1) { v.push(2); }
        let sccs = find_cycles(&adj);
        assert!(sccs.is_empty());
    }

    #[test]
    fn should_find_three_node_cycle() {
        // 0 → 1 → 2 → 0
        let mut adj: Vec<Vec<usize>> = (0..3).map(|_| Vec::new()).collect();
        if let Some(v) = adj.get_mut(0) { v.push(1); }
        if let Some(v) = adj.get_mut(1) { v.push(2); }
        if let Some(v) = adj.get_mut(2) { v.push(0); }
        let sccs = find_cycles(&adj);
        assert_eq!(sccs.len(), 1);
        if let Some(scc) = sccs.into_iter().next() {
            assert_eq!(scc.len(), 3);
        }
    }

    #[test]
    fn should_handle_empty_graph() {
        let adj: Vec<Vec<usize>> = Vec::new();
        let sccs = find_cycles(&adj);
        assert!(sccs.is_empty());
    }
}
