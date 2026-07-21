use super::format::{dm_sanitize, dm_truncate, status_class, status_rank};
use super::model::{DagNode, RoadmapDag, TopoOrder};

impl RoadmapDag {
    /// Kahn topological sort over `depends_on`∪`blocks` — `Ordered` (prereqs first) or `Cycle` with residual nodes.
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

    /// Decision-spine Mermaid `graph TD`: decision/roadmap nodes status-styled, depends_on solid + supersedes dotted; focus+max_nodes cap; None when empty.
    #[must_use]
    pub fn decision_mermaid(&self, focus: &[String], max_nodes: usize) -> Option<String> {
        let in_focus = |id: &str| {
            focus.is_empty()
                || focus
                    .iter()
                    .any(|f| id == f || id.rsplit('/').next() == Some(f.as_str()))
        };
        let mut kept: Vec<&DagNode> = self
            .nodes
            .iter()
            .filter(|n| n.category == "decision" || n.category == "roadmap")
            .filter(|n| in_focus(&n.id))
            .collect();
        if kept.is_empty() {
            return None;
        }
        kept.sort_by_key(|n| status_rank(&n.entry_status));
        kept.truncate(max_nodes);
        let keep_ids: std::collections::HashSet<&str> =
            kept.iter().map(|n| n.id.as_str()).collect();

        let mut out = String::from("graph TD\n");
        for n in &kept {
            let id = dm_sanitize(&n.id);
            let label = dm_truncate(&n.title);
            let st = n.entry_status.to_uppercase();
            let cls = status_class(&n.entry_status);
            out.push_str("  ");
            out.push_str(&id);
            out.push_str("[\"");
            out.push_str(&label);
            out.push_str("<br/>");
            out.push_str(&st);
            out.push_str("\"]:::");
            out.push_str(cls);
            out.push('\n');
        }
        for e in &self.edges {
            if !keep_ids.contains(e.source.as_str()) || !keep_ids.contains(e.target.as_str()) {
                continue;
            }
            let arrow = match e.rel.as_str() {
                "depends_on" => Some(" --> "),
                "supersedes" => Some(" -.retires.-> "),
                _ => None,
            };
            if let Some(a) = arrow {
                out.push_str("  ");
                out.push_str(&dm_sanitize(&e.source));
                out.push_str(a);
                out.push_str(&dm_sanitize(&e.target));
                out.push('\n');
            }
        }
        out.push_str("  classDef done fill:#1a7f37,color:#fff;\n");
        out.push_str("  classDef open fill:#9a3412,color:#fff;\n");
        out.push_str("  classDef draft fill:#854d0e,color:#fff;\n");
        Some(out)
    }

    /// The retired patterns: every `pattern` node that is the TARGET of a
    /// `supersedes` edge (something replaced it). Returns `(title, retired_by)`
    /// pairs — `retired_by` is the superseding node's title — so a gate can cite
    /// what the codebase moved to. Empty when nothing is retired.
    #[must_use]
    pub fn retired_patterns(&self) -> Vec<(String, String)> {
        let title_of = |id: &str| {
            self.nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| n.title.clone())
        };
        self.edges
            .iter()
            .filter(|e| e.rel == "supersedes")
            .filter_map(|e| {
                let retired = title_of(&e.target)?;
                let replacement = title_of(&e.source)?;
                Some((retired, replacement))
            })
            .collect()
    }

    /// Render the `pattern`-category slice as a Mermaid `graph TD` supersession
    /// DAG: each pattern node, status-styled by trust (`verified` = soaked,
    /// `todo` = fresh-but-unsoaked), and `supersedes` edges showing which pattern
    /// RETIRED which (`-.retires.->`). This is the research-refreshed pattern
    /// layer's read-side VIEW — the model sees which detector the codebase has
    /// already replaced, so it adopts the current one. `focus` (qnames or bare
    /// keys, same match rule as [`Self::decision_mermaid`]) restricts to the
    /// relevant pattern neighbourhood for token discipline; an empty `focus`
    /// renders the whole pattern layer. `None` when no pattern node survives the
    /// filter. SOURCE: roadmap.unit.mermaid-decision-architecture.
    #[must_use]
    pub fn pattern_dag_mermaid(&self, focus: &[String], max_nodes: usize) -> Option<String> {
        let in_focus = |id: &str| {
            focus.is_empty()
                || focus
                    .iter()
                    .any(|f| id == f || id.rsplit('/').next() == Some(f.as_str()))
        };
        let mut kept: Vec<&DagNode> = self
            .nodes
            .iter()
            .filter(|n| n.category == "pattern")
            .filter(|n| in_focus(&n.id))
            .collect();
        if kept.is_empty() {
            return None;
        }
        kept.sort_by_key(|n| status_rank(&n.entry_status));
        kept.truncate(max_nodes);
        let keep_ids: std::collections::HashSet<&str> =
            kept.iter().map(|n| n.id.as_str()).collect();

        let mut out = String::from("graph TD\n");
        for n in &kept {
            let trust = if n.entry_status == "todo" {
                " (fresh)"
            } else {
                ""
            };
            out.push_str("  ");
            out.push_str(&dm_sanitize(&n.id));
            out.push_str("[\"");
            out.push_str(&dm_truncate(&n.title));
            out.push_str(trust);
            out.push_str("\"]:::");
            out.push_str(status_class(&n.entry_status));
            out.push('\n');
        }
        for e in &self.edges {
            if e.rel != "supersedes"
                || !keep_ids.contains(e.source.as_str())
                || !keep_ids.contains(e.target.as_str())
            {
                continue;
            }
            out.push_str("  ");
            out.push_str(&dm_sanitize(&e.source));
            out.push_str(" -.retires.-> ");
            out.push_str(&dm_sanitize(&e.target));
            out.push('\n');
        }
        out.push_str("  classDef done fill:#1a7f37,color:#fff;\n");
        out.push_str("  classDef open fill:#854d0e,color:#fff;\n");
        out.push_str("  classDef draft fill:#9a3412,color:#fff;\n");
        Some(out)
    }
}
