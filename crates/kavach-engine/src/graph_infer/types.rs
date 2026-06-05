//! Cross-crate-constructed input/output records for graph inference.

/// A memory row fed to the inferencer: its identity plus the prose to scan.
#[derive(Clone, Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate; non_exhaustive => E0639"
)]
pub struct InferRow {
    pub project_slug: String,
    pub category: String,
    pub entry_key: String,
    pub content: String,
}

/// A derived edge: `source_qname` --rel--> `target_qname`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate; non_exhaustive => E0639"
)]
pub struct InferredRel {
    pub source_qname: String,
    pub rel: String,
    pub target_qname: String,
}
