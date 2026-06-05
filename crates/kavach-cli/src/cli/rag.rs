use clap::Subcommand;

/// Actions for `kavach rag <action>`. Builds and queries vectorless
/// reasoning-based RAG trees used by the intent and pre-write gates.
#[derive(Subcommand)]
pub(crate) enum RagAction {
    /// Scan a directory for markdown files and emit skeleton trees as NDJSON.
    ///
    /// Output is one JSON tree per line. Pipe into an LLM summarizer to fill
    /// in summary/keywords, then feed the result to `kavach rag apply`.
    /// With `--persist`, trees are written directly into `SurrealDB`'s
    /// `rag_trees` table instead of stdout, keyed by `--label`.
    Build {
        /// Directory to scan recursively
        #[arg(long)]
        source: String,
        /// Label to embed in each tree's `source` field (also db primary key)
        #[arg(long, default_value = "user")]
        label: String,
        /// Persist directly to `SurrealDB` instead of writing NDJSON to stdout
        #[arg(long)]
        persist: bool,
    },
    /// List persisted trees in `SurrealDB`.
    List,
    /// Build and persist a skills tree with deterministic metadata enrichment.
    ///
    /// One-shot convenience: scans `source` for SKILL.md files, builds the
    /// tree skeleton, then fills each root node's `summary`, `keywords`, and
    /// `file_patterns` from the YAML frontmatter (description, metadata.triggers,
    /// `file_patterns`). No LLM call — pure deterministic extraction. Persists
    /// the enriched trees to `SurrealDB` under `label` so gates can query them.
    EnrichSkills {
        /// Directory containing skills (e.g. ~/.claude/skills)
        #[arg(long)]
        source: String,
        /// Label to store under in `SurrealDB`
        #[arg(long, default_value = "skills")]
        label: String,
    },
    /// Rebuild and persist only if the source directory has changed.
    ///
    /// Computes the prospective tree hash from the current contents of
    /// `source`, compares against the stored `source_hash` for `label`, and
    /// writes the new tree only when they differ. Safe to call from a
    /// session-start hook: near-zero cost when source files are stable, one
    /// rebuild when a SKILL.md changes.
    RefreshIfStale {
        /// Directory containing skills (e.g. ~/.claude/skills)
        #[arg(long)]
        source: String,
        /// Label to store under in `SurrealDB`
        #[arg(long, default_value = "skills")]
        label: String,
    },
    /// Build and persist a generic markdown tree with frontmatter enrichment.
    ///
    /// Scans `source` for markdown files, builds tree skeletons, enriches
    /// root nodes from YAML frontmatter (description, keywords, `file_patterns`),
    /// and persists to `SurrealDB`. Works with any markdown directory — rules,
    /// agents, commands, docs, or custom knowledge bases.
    Enrich {
        /// Directory to scan recursively
        #[arg(long)]
        source: String,
        /// Label to store under in `SurrealDB`
        #[arg(long)]
        label: String,
    },
    /// Match a query against a tree JSON file and print top-k hits.
    Query {
        /// Path to a tree JSON file emitted by `build`
        #[arg(long)]
        tree: String,
        /// Target file path (used for `file_patterns` scoring)
        #[arg(long)]
        file: String,
        /// Raw query text (tokenized for keyword + summary scoring)
        #[arg(long, default_value = "")]
        text: String,
        /// Optional intent hint for title-match bonus
        #[arg(long, default_value = "")]
        intent: String,
        /// Max results to return
        #[arg(long, default_value_t = 5)]
        top_k: usize,
    },
    /// Apply NDJSON summary responses to a tree and print the enriched JSON.
    ///
    /// `responses` is a path to a file containing one JSON `SummaryResponse`
    /// per line — the output of an external summarizer that consumed
    /// `kavach rag pending`. Writes the updated tree to stdout.
    Apply {
        /// Path to a tree JSON file
        #[arg(long)]
        tree: String,
        /// Path to NDJSON responses (one `SummaryResponse` per line)
        #[arg(long)]
        responses: String,
    },
    /// Print the list of pending summary requests for a tree JSON file.
    ///
    /// One request per line as NDJSON, ready to pipe into an external
    /// summarizer. The summarizer reads requests, returns responses in the
    /// same format, and those are applied via `kavach rag apply`.
    Pending {
        /// Path to a tree JSON file
        #[arg(long)]
        tree: String,
    },
}
