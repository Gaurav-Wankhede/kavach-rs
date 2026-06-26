//! kavach:nano-file-exempt single clap `Subcommand` enum — the `db` command
//! surface. One enum = one cohesive CLI contract; splitting variants across
//! files fragments the command definition with no reuse gain (clap derive
//! needs them in one enum). Variant handlers live in `cmd/db/*` (already split).
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum DbAction {
    /// Register a project with absolute path
    #[command(
        after_help = "EXAMPLES:\n  kavach db register --slug my-app --path /abs/path --stack 'rust|axum'\n\nWHEN: First-time onboarding of a repo so kavach can bind sessions to it."
    )]
    Register {
        /// Project slug (unique identifier)
        #[arg(long)]
        slug: String,
        /// Absolute path to project root
        #[arg(long)]
        path: String,
        /// Tech stack (e.g. "rust|axum")
        #[arg(long)]
        stack: Option<String>,
    },
    /// Register a sub-part within a project
    #[command(
        after_help = "EXAMPLES:\n  kavach db register-part --project my-app --name backend --path /abs/backend --type backend\n\nWHEN: A monorepo has distinct parts (backend/frontend/infra) you want scoped separately."
    )]
    RegisterPart {
        /// Parent project slug
        #[arg(long)]
        project: String,
        /// Part name (e.g. "backend", "frontend")
        #[arg(long)]
        name: String,
        /// Absolute path to part root
        #[arg(long)]
        path: String,
        /// Part type (backend, frontend, database, mobile, infra, docs, shared, other)
        #[arg(long = "type")]
        part_type: String,
    },
    /// Query memory entries for a project
    #[command(
        after_help = "EXAMPLES:\n  kavach db query --project P --category roadmap\n  kavach db query --project P --category decision --depth 400 --all\n\nWHEN: Browse a category's rows; add --depth for body text, --all to include done items."
    )]
    Query {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: Option<String>,
        /// Include DONE items (filtered out by default for roadmap)
        #[arg(long)]
        all: bool,
        /// Per-row content depth: a char count (e.g. `--depth 400`) or `all` for
        /// the whole body. Omitted prints titles only (breadth). `KAVACH_NO_TRUNCATE=1`
        /// forces `all` everywhere.
        #[arg(long)]
        depth: Option<String>,
    },
    /// Search with metadata filters (`entry_status`, since, contains)
    /// SOURCE: <https://docs.rs/clap/latest/clap>/_derive/_tutorial/index.html
    #[command(
        after_help = "EXAMPLES:\n  kavach db search --project P --category roadmap --status in_progress\n  kavach db search --project P --since 7d --contains scylla --limit 50\n\nWHEN: Narrow a category by status/recency/substring instead of listing everything."
    )]
    Search {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: Option<String>,
        /// Filter by `entry_status` (planned, todo, `in_progress`, done, verified)
        #[arg(long)]
        status: Option<String>,
        /// Filter entries updated within duration (e.g. 7d, 30d, 1h)
        #[arg(long)]
        since: Option<String>,
        /// Filter by title/content containing substring
        #[arg(long)]
        contains: Option<String>,
        /// Max results (default: 20)
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Write a memory entry. Strict mode (`roadmap/decision/research/pattern/app_spec)`:
    /// pass either `--new` (create — fuzzy-checks against existing titles) or
    /// `--update-key <existing>` (update a known row). Plain `--key` without
    /// either flag is rejected to prevent stale duplicate rows.
    /// SOURCE: <https://docs.rs/clap/latest/clap>/_derive/_tutorial/index.html
    #[command(
        after_help = "EXAMPLES:\n  kavach db write --project P --category roadmap --key K --title T --new\n  kavach db write --project P --category decision --key K --title T --update-key K < body.md\n\nWHEN: Persist a decision/roadmap/research row. Always pass --new (create) or --update-key (edit)."
    )]
    Write {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Entry key (unique within project+category)
        #[arg(long)]
        key: String,
        /// Entry title
        #[arg(long)]
        title: String,
        /// Content. If omitted, the body is read from a piped/redirected stdin
        /// (e.g. `... < plan.md` or `cat plan.md | ...`); an interactive
        /// terminal with no pipe stores the title only.
        #[arg(long)]
        content: Option<String>,
        /// Intent: create a brand-new row. Gate fuzzy-matches title against
        /// existing rows in same category; refuses if similarity >= 0.85.
        /// Mutually exclusive with --update-key.
        #[arg(long, conflicts_with = "update_key")]
        new: bool,
        /// Intent: update an existing row by its key. CLI verifies the key
        /// exists in same project+category; refuses if not found.
        /// Mutually exclusive with --new.
        #[arg(long = "update-key")]
        update_key: Option<String>,
        /// Dispatch priority (roadmap/decision only). Lower number = higher
        /// rank — `--priority 1` is picked before `--priority 2`, like
        /// nice(1). Omit to leave existing value unchanged on update or
        /// NONE on insert (NONE rows sort after every prioritized row,
        /// then by `created_at` ASC).
        #[arg(long)]
        priority: Option<i64>,
        /// Declarative dependency edge(s): this card `depends_on` the given
        /// key(s). Repeatable (`--depends-on a --depends-on b`). A bare key
        /// resolves to the same project+category; a `slug/cat/key` qname is
        /// used verbatim. Feeds the DAG scheduler's topological ordering so
        /// dependents wait until each target reaches done/verified — the
        /// precise complement to the NLU prose scanner and frontmatter.
        #[arg(long = "depends-on")]
        depends_on: Vec<String>,
        /// Opus-authored executor prompt (roadmap only), served verbatim by `db next-prompt`.
        #[arg(long = "exec-prompt")]
        exec_prompt: Option<String>,
    },
    /// Set or clear the priority of an existing entry without touching title/content.
    /// Surgical rerank verb for human-in-loop focus shifts. Refuses if the row
    /// does not exist (no implicit insert). Lower number = higher rank, like nice(1).
    #[command(
        after_help = "EXAMPLES:\n  kavach db priority-set --project P --category roadmap --key K --priority 1\n  kavach db priority-set --project P --category roadmap --key K --clear\n\nWHEN: Re-rank the backlog by hand without touching a card's title or content."
    )]
    PrioritySet {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Category (roadmap or decision — only these carry priority)
        #[arg(long)]
        category: String,
        /// Entry key (must already exist)
        #[arg(long)]
        key: String,
        /// New priority (lower = higher rank). Mutually exclusive with --clear.
        #[arg(long, conflicts_with = "clear")]
        priority: Option<i64>,
        /// Clear the priority (back to NONE, FIFO tail). Mutually exclusive with --priority.
        #[arg(long, conflicts_with = "priority")]
        clear: bool,
    },
    /// Pin a roadmap card to a dispatch LANE (or clear it back to unlaned).
    /// A session running `KAVACH_LANE=<name>` dispatches its own lane first,
    /// then the unlaned backlog, never a foreign lane. Refuses if the row is
    /// absent (no implicit insert). Roadmap only.
    #[command(
        after_help = "EXAMPLES:\n  kavach db lane-set --project P --key K --lane backend\n  kavach db lane-set --project P --key K --clear\n\nWHEN: Shard a roadmap card to a named dispatch lane (or return it to the general backlog)."
    )]
    LaneSet {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Entry key (must already exist)
        #[arg(long)]
        key: String,
        /// Lane name to pin the card to. Mutually exclusive with --clear.
        #[arg(long, conflicts_with = "clear")]
        lane: Option<String>,
        /// Clear the lane (back to the unlaned general backlog). Mutually
        /// exclusive with --lane.
        #[arg(long, conflicts_with = "lane")]
        clear: bool,
    },
    /// Sync session state to database
    #[command(
        after_help = "EXAMPLES:\n  kavach db sync\n\nWHEN: Flush in-memory session state to the store after a batch of changes."
    )]
    Sync,
    /// Find project matching an absolute path
    #[command(
        after_help = "EXAMPLES:\n  kavach db find-project --path /abs/path/to/file.rs\n\nWHEN: Resolve which registered project owns an absolute path."
    )]
    FindProject {
        /// Absolute file or directory path
        #[arg(long)]
        path: String,
    },
    /// Find part matching an absolute path
    #[command(
        after_help = "EXAMPLES:\n  kavach db find-part --path /abs/path/to/backend/x.rs\n\nWHEN: Resolve which registered sub-part owns an absolute path."
    )]
    FindPart {
        /// Absolute file or directory path
        #[arg(long)]
        path: String,
    },
    /// List all registered projects
    #[command(
        after_help = "EXAMPLES:\n  kavach db list-projects\n\nWHEN: See every registered project slug + root path."
    )]
    ListProjects,
    /// Set (or clear) a project's parent, building the hierarchy
    #[command(
        after_help = "EXAMPLES:\n  kavach db set-parent --child sub-app --parent mono\n  kavach db set-parent --child sub-app          # detach to top-level\n\nWHEN: Build or unbuild the project hierarchy."
    )]
    SetParent {
        /// Child project slug
        #[arg(long)]
        child: String,
        /// Parent project slug; omit to detach to top-level
        #[arg(long)]
        parent: Option<String>,
    },
    /// Render the project hierarchy as an indented tree
    #[command(
        after_help = "EXAMPLES:\n  kavach db tree\n\nWHEN: View the whole project hierarchy as an indented tree."
    )]
    Tree,
    /// List parts for a project
    #[command(
        after_help = "EXAMPLES:\n  kavach db list-parts --project my-app\n\nWHEN: Enumerate the registered sub-parts of one project."
    )]
    ListParts {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Archive expired memory entries
    #[command(
        after_help = "EXAMPLES:\n  kavach db expire\n\nWHEN: Archive memory rows whose TTL has elapsed."
    )]
    Expire,
    /// Append an event to the log
    #[command(
        after_help = "EXAMPLES:\n  kavach db event --type file_write --payload '{\"path\":\"x.rs\"}'\n\nWHEN: Append an audit event to the log (usually hook-driven)."
    )]
    Event {
        /// Event type (e.g. "`file_write`", "`session_start`")
        #[arg(long = "type")]
        event_type: String,
        /// JSON payload
        #[arg(long)]
        payload: Option<String>,
    },
    /// Fetch a single memory entry by key
    #[command(
        after_help = "EXAMPLES:\n  kavach db get --project P --category roadmap --key roadmap.unit.foo --full\n\nWHEN: Before implementing — always `--full` for roadmap units."
    )]
    Get {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Entry key
        #[arg(long)]
        key: String,
        /// Show all metadata (tags, ttl, source, timestamps)
        #[arg(long)]
        full: bool,
        /// Return the SHORT snippet body (single-key get returns FULL content by
        /// default — naming one exact key is a depth request)
        #[arg(long)]
        snippet: bool,
    },
    /// Delete events older than N days (TIME-BASED — transitional, prefer `archive`)
    #[command(
        after_help = "EXAMPLES:\n  kavach db rotate --days 30\n\nWHEN: Drop events older than N days. Prefer `archive` (relevance-based) over this."
    )]
    Rotate {
        /// Number of days to retain
        #[arg(long)]
        days: i64,
    },
    /// Archive events with no graph anchors to active roadmap (relevance-based, audit-preserving)
    #[command(
        after_help = "EXAMPLES:\n  kavach db archive --floor-days 30 --dry-run\n  kavach db archive --floor-days 30\n\nWHEN: Relevance-archive anchor-less events while preserving audit trail."
    )]
    Archive {
        /// Floor age in days — events younger than this are never archived
        #[arg(long, default_value_t = 30)]
        floor_days: i64,
        /// Dry-run: report what would be archived without modifying state
        #[arg(long)]
        dry_run: bool,
    },
    /// Show kanban board for a project (todo / `in_progress` / done)
    #[command(
        long_about = "Direct SurrealDB read (no RPC). Primary health check for agents.\n\n\
Open lanes: todo, in_progress, done. Use --include-verified for terminal verified rows.",
        after_help = "EXAMPLES:\n  kavach db kanban --project nicole-carpenter --limit 10\n  \
kavach db kanban --project P --status in_progress --key backend --json\n\n\
WHEN: Session start and after every card close — prefer over stop-hook pipes."
    )]
    Kanban {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Max items to show (default: 15, 0 = unlimited)
        #[arg(long, default_value_t = 15)]
        limit: usize,
        /// Filter by status (todo, `in_progress`, done) — open lanes only;
        /// verified/planned rows are terminal/backlog and not shown here
        /// unless --include-verified is passed.
        #[arg(long)]
        status: Option<String>,
        /// Show `in_progress` items first
        #[arg(long)]
        active_first: bool,
        /// Filter by key substring (e.g. "backend" matches "backend.crate.foo")
        #[arg(long)]
        key: Option<String>,
        /// Filter by dispatch lane (exact match). Shows only cards pinned to
        /// this lane — the board lens for affinity-sharded sessions.
        #[arg(long)]
        lane: Option<String>,
        /// Also render terminal `verified` rows in a [VERIFIED] lens. Off by
        /// default — the board shows OPEN work; this surfaces closed items
        /// (e.g. to confirm a unit reached `verified`, not just `done`).
        #[arg(long)]
        include_verified: bool,
        /// Output as JSON for agent parsing
        #[arg(long)]
        json: bool,
        /// Override the always-on dependency-DAG view.
        ///
        /// The default (no flag) is the topo-tiered text DAG, where the first
        /// tier is ready now and deeper tiers unlock as prerequisites close, with
        /// READY/BLOCKED/CYCLE markers, inline depends-on, and per-card status —
        /// the agent's always-on task awareness. Passing `mermaid` emits a
        /// `flowchart TD` for human/GUI viewing, and `--json` gives the
        /// machine-parseable card list. Reads the same declared deps the
        /// scheduler dispatches on.
        #[arg(long, value_parser = ["dag", "mermaid"])]
        format: Option<String>,
    },
    /// Close a kanban roadmap item by key (marks status=verified)
    #[command(
        after_help = "EXAMPLES:\n  kavach db kanban-close --project P --key roadmap.unit.foo\n\nWHEN: Mark a roadmap card verified once `kavach verify` has passed."
    )]
    KanbanClose {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Roadmap entry key to close (e.g. phase-1-auth)
        #[arg(long)]
        key: String,
    },
    /// Update the strict status of a memory entry (`todo|in_progress|done|verified`)
    #[command(
        after_help = "EXAMPLES:\n  kavach db status-update --project P --category roadmap --key K --status in_progress\n  kavach db status-update --project P --category roadmap --key K --status done\n\nWHEN: Claim card (todo→in_progress), finish work (→done), then run `kavach verify` (→verified)."
    )]
    StatusUpdate {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Entry key
        #[arg(long)]
        key: String,
        /// New status: planned, todo, `in_progress`, done, verified
        #[arg(long)]
        status: String,
    },
    /// Populate the knowledge graph from existing relational data
    #[command(
        after_help = "EXAMPLES:\n  kavach db populate-graph\n\nWHEN: One-time/after-import build of the knowledge graph from relational rows."
    )]
    PopulateGraph,
    /// Backfill typed inter-entry edges (`depends_on/blocks/supersedes/references/mentions`)
    /// by re-extracting frontmatter + wikilinks from every existing memory row.
    /// Safe to re-run; UPSERT-based.
    #[command(
        after_help = "EXAMPLES:\n  kavach db backfill-relationships --project P --dry-run\n  kavach db backfill-relationships\n\nWHEN: Rebuild typed inter-entry edges from frontmatter + wikilinks. Safe to re-run."
    )]
    BackfillRelationships {
        /// Optional project slug filter (default: all projects)
        #[arg(long)]
        project: Option<String>,
        /// Print planned edges without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Query the knowledge graph — list entities and their edges
    #[command(
        after_help = "EXAMPLES:\n  kavach db graph-query --entity-type concept --limit 20\n  kavach db graph-query --name paseto_v4\n\nWHEN: Inspect graph entities and their edges by type or name."
    )]
    GraphQuery {
        /// Filter by entity type (skill, rule, gate, project, file, memory, session, `event_type`)
        #[arg(long)]
        entity_type: Option<String>,
        /// Show edges for a specific entity name
        #[arg(long)]
        name: Option<String>,
        /// Max entities to show (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Introspect a local `PostgreSQL` database — list tables, columns, FKs
    #[command(
        after_help = "EXAMPLES:\n  kavach db pg-introspect --dsn postgres://localhost/mydb\n\nWHEN: List a live Postgres schema's tables, columns, and FKs."
    )]
    PgIntrospect {
        /// Postgres DSN (e.g. <postgres://localhost/mydb>)
        #[arg(long)]
        dsn: String,
    },
    /// Find isolated tables (zero incoming + outgoing FK edges)
    #[command(
        after_help = "EXAMPLES:\n  kavach db pg-isolation --dsn postgres://localhost/mydb\n\nWHEN: Find tables with no FK edges (often a modeling smell)."
    )]
    PgIsolation {
        /// Postgres DSN
        #[arg(long)]
        dsn: String,
    },
    /// Emit ER diagram in Mermaid format from the live schema
    #[command(
        after_help = "EXAMPLES:\n  kavach db pg-er --dsn postgres://localhost/mydb\n\nWHEN: Emit a Mermaid ER diagram from the live schema."
    )]
    PgEr {
        /// Postgres DSN
        #[arg(long)]
        dsn: String,
    },
    /// Detect likely missing FKs — columns named `<table>_id` with no declared FK
    #[command(
        after_help = "EXAMPLES:\n  kavach db pg-drift --dsn postgres://localhost/mydb\n\nWHEN: Detect `<table>_id` columns missing a declared FK."
    )]
    PgDrift {
        /// Postgres DSN
        #[arg(long)]
        dsn: String,
    },
    /// Delete specific record(s) by key or category (granular, preferred over wipe-project)
    #[command(
        after_help = "EXAMPLES:\n  kavach db delete --project P --category roadmap --key K --confirm\n  kavach db delete --project P --category research --all --dry-run\n\nWHEN: Granular removal of one row or a whole category. Prefer over wipe-project."
    )]
    Delete {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Specific key to delete (omit with --all for category-wide)
        #[arg(long)]
        key: Option<String>,
        /// Delete all records in category (requires --confirm)
        #[arg(long)]
        all: bool,
        /// Confirm deletion
        #[arg(long)]
        confirm: bool,
        /// Dry-run: show what would be deleted
        #[arg(long)]
        dry_run: bool,
    },
    /// Bulk-purge records in a category whose key starts with a prefix (requires --confirm).
    /// E.g. clear every `heal.incident.loophole-*` roadmap card in one pass.
    #[command(
        after_help = "EXAMPLES:\n  kavach db delete-prefix --project P --category roadmap --prefix heal.incident.loophole- --dry-run\n  kavach db delete-prefix --project P --category roadmap --prefix heal.incident.loophole- --confirm\n\nWHEN: Bulk-purge a key-prefix family (e.g. stale heal-incident cards) in one pass."
    )]
    DeletePrefix {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Key prefix to match (e.g. "heal.incident.loophole-")
        #[arg(long)]
        prefix: String,
        /// Confirm the bulk purge (skipped under --dry-run)
        #[arg(long)]
        confirm: bool,
        /// Dry-run: count what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Wipe ALL data for a project (DANGEROUS — use `delete` for granular removal)
    #[command(
        after_help = "EXAMPLES:\n  kavach db wipe-project --project P --dry-run\n  kavach db wipe-project --project P --confirm\n\nWHEN: DANGEROUS — destroy ALL data for a project. Prefer `delete` for anything granular."
    )]
    WipeProject {
        /// Project slug to wipe
        #[arg(long)]
        project: String,
        /// Confirm destructive operation (DANGEROUS: deletes ALL project data)
        #[arg(long)]
        confirm: bool,
        /// Dry-run: show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Insert-or-update a global L0 concept (cross-project knowledge graph node)
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-add --name paseto_v4 --display 'PASETO v4' --desc '...' --tags token,crypto --sources https://...\n\nWHEN: Add/update a global L0 concept node in the cross-project graph."
    )]
    ConceptAdd {
        /// Canonical concept id (`snake_case`, e.g. `paseto_v4`)
        #[arg(long)]
        name: String,
        /// Display label (e.g. "PASETO v4")
        #[arg(long)]
        display: String,
        /// Description / prose body
        #[arg(long)]
        desc: String,
        /// Comma-separated tags (e.g. token,crypto,stateless)
        #[arg(long)]
        tags: Option<String>,
        /// Comma-separated source URLs
        #[arg(long)]
        sources: Option<String>,
    },
    /// RELATE two concepts via an ontology edge
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-link --from paseto_v4 --edge alternative_to --to jwt\n\nWHEN: Relate two concepts via an ontology edge."
    )]
    ConceptLink {
        /// Source concept name
        #[arg(long)]
        from: String,
        /// Ontology edge (`is_a`, `part_of`, `prerequisite_of`, `alternative_to`,
        /// composes, mitigates, `instance_of`, subsumes)
        #[arg(long)]
        edge: String,
        /// Target concept name
        #[arg(long)]
        to: String,
    },
    /// Full-text search concepts by description (BM25)
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-search --query 'stateless token' --limit 20\n\nWHEN: Full-text (BM25) lookup of concepts by description."
    )]
    ConceptSearch {
        /// Search terms
        #[arg(long)]
        query: String,
        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// List every concept
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-list --limit 50\n\nWHEN: Enumerate every concept node."
    )]
    ConceptList {
        /// Max results
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Remove a single concept row by canonical name
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-delete --name paseto_v4\n\nWHEN: Remove one concept row by canonical name."
    )]
    ConceptDelete {
        /// Concept name (`snake_case`)
        #[arg(long)]
        name: String,
    },
    /// Bulk-purge concept rows by name prefix (requires --confirm)
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-delete-prefix --prefix keyword: --confirm\n\nWHEN: Bulk-purge harvest-noise concept rows by name prefix."
    )]
    ConceptDeletePrefix {
        /// Name prefix (e.g. "keyword:" to purge harvest noise)
        #[arg(long)]
        prefix: String,
        /// Required to actually run the bulk purge
        #[arg(long)]
        confirm: bool,
    },
    /// Add or refresh a citation (official-docs context) keyed by (project, entry-key)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-add --project P --entry-key surreal --name SurrealDB --slug records --url https://surrealdb.com/docs\n\nWHEN: Pin an official-docs citation to a (project, entry-key)."
    )]
    CitationAdd {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Stable entry key (`snake_case`)
        #[arg(long = "entry-key")]
        entry_key: String,
        /// Display name (e.g. `SurrealDB`)
        #[arg(long)]
        name: String,
        /// One metadata slug (e.g. "records")
        #[arg(long)]
        slug: String,
        /// Official-docs URL (non-empty)
        #[arg(long)]
        url: String,
    },
    /// Fetch one citation (bumps `access_count`)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-get --project P --entry-key surreal\n\nWHEN: Fetch one citation (bumps its access_count)."
    )]
    CitationGet {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Entry key
        #[arg(long = "entry-key")]
        entry_key: String,
    },
    /// List every citation for a project (newest-updated first)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-list --project P\n\nWHEN: List a project's citations, newest-updated first."
    )]
    CitationList {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Merge a node into a citation via a `->cite->` edge (`table:key` ids)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-link --node decision:foo --citation citation:surreal\n\nWHEN: Attach a graph node to a citation via a ->cite-> edge."
    )]
    CitationLink {
        /// Source node record id (`decision:foo`, `entity:bar`)
        #[arg(long)]
        node: String,
        /// Target citation record id (`citation:baz`)
        #[arg(long)]
        citation: String,
    },
    /// List the nodes that cite a citation (single `<-cite` walk)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-traverse --citation citation:surreal\n\nWHEN: List every node that cites a given citation."
    )]
    CitationTraverse {
        /// Citation record id (`citation:baz`)
        #[arg(long)]
        citation: String,
    },
    /// Flow RLAIF reward along a citation's `cite` edges (bumps edge weight)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-refresh --citation citation:surreal --delta 1.0\n\nWHEN: Flow RLAIF reward along a citation's cite edges."
    )]
    CitationRefresh {
        /// Citation record id (`citation:baz`)
        #[arg(long)]
        citation: String,
        /// Reward delta (negative = penalty)
        #[arg(long, default_value_t = 1.0)]
        delta: f64,
    },
    /// Resolve a gate-config override (project-then-global), or report the miss
    #[command(
        after_help = "EXAMPLES:\n  kavach db gate-config-get --project P --gate-key dup.block\n  kavach db gate-config-get --project '*' --gate-key dup.block   # global\n\nWHEN: Resolve a gate override (project then global), or confirm the miss."
    )]
    GateConfigGet {
        /// Project slug (`*` for the global row)
        #[arg(long)]
        project: String,
        /// Gate-config key (e.g. `dup.block`, `session.autonomy_contract`)
        #[arg(long = "gate-key")]
        gate_key: String,
    },
    /// Set a gate-config override (exactly one value flag per `--kind`)
    #[command(
        after_help = "EXAMPLES:\n  kavach db gate-config-set --project P --gate-key dup.block --kind enabled --boolean true\n  kavach db gate-config-set --project '*' --gate-key dup.threshold --kind threshold --num 0.85\n\nWHEN: Override one gate's config; pass exactly one value flag matching --kind."
    )]
    GateConfigSet {
        /// Project slug (`*` for the global row)
        #[arg(long)]
        project: String,
        /// Gate-config key
        #[arg(long = "gate-key")]
        gate_key: String,
        /// Value kind: `threshold` | `pattern_list` | `enabled` | `severity` | `text`
        #[arg(long)]
        kind: String,
        /// Numeric value (for `--kind threshold`)
        #[arg(long)]
        num: Option<f64>,
        /// Boolean value (for `--kind enabled`)
        #[arg(long)]
        boolean: Option<bool>,
        /// Comma-separated list (for `--kind pattern_list`)
        #[arg(long)]
        list: Option<String>,
        /// Text value (for `--kind severity` | `--kind text`)
        #[arg(long)]
        text: Option<String>,
    },
    /// Delete a gate-config override, reverting to the compiled default
    #[command(
        after_help = "EXAMPLES:\n  kavach db gate-config-delete --project P --gate-key dup.block\n\nWHEN: Revert a gate override back to its compiled default."
    )]
    GateConfigDelete {
        /// Project slug (`*` for the global row)
        #[arg(long)]
        project: String,
        /// Gate-config key
        #[arg(long = "gate-key")]
        gate_key: String,
    },
    /// List every gate-config override for a project (`*` for the globals)
    #[command(
        after_help = "EXAMPLES:\n  kavach db gate-config-list --project P\n  kavach db gate-config-list --project '*'    # globals\n\nWHEN: See every gate override for a project (or the global row)."
    )]
    GateConfigList {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Bridge a project-scoped L1 entity to a global L0 concept
    #[command(
        after_help = "EXAMPLES:\n  kavach db bridge-create --src-table decision --src-key foo --edge references_concept --concept paseto_v4\n\nWHEN: Link a project-scoped L1 row to a global L0 concept."
    )]
    BridgeCreate {
        /// Source table (roadmap | decision | research | pattern | `app_spec`)
        #[arg(long = "src-table")]
        src_table: String,
        /// Source `entry_key`
        #[arg(long = "src-key")]
        src_key: String,
        /// Bridge edge (implements | discusses | `references_concept` | violates)
        #[arg(long)]
        edge: String,
        /// Target concept name
        #[arg(long)]
        concept: String,
    },
    /// List every concept that any L1 entity in this project bridges to
    #[command(
        after_help = "EXAMPLES:\n  kavach db bridge-concepts-for --project P\n\nWHEN: List every concept this project's entities bridge to."
    )]
    BridgeConceptsFor {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// List every project (via L1 entity) that bridges to this concept
    #[command(
        after_help = "EXAMPLES:\n  kavach db bridge-projects-for --concept paseto_v4\n\nWHEN: List every project that bridges to a given concept."
    )]
    BridgeProjectsFor {
        /// Concept name
        #[arg(long)]
        concept: String,
    },
    /// Count inbound `instance_of` edges on an `anti_pattern` (mistake recurrence)
    #[command(
        after_help = "EXAMPLES:\n  kavach db mistake-hit-count --name anti.self_imposed_limit.abc12345\n\nWHEN: See how many times an anti-pattern has recurred (inbound instance_of edges)."
    )]
    MistakeHitCount {
        /// Anti-pattern canonical name (e.g. `anti.self_imposed_limit.abc12345`)
        #[arg(long)]
        name: String,
    },
    /// Off-policy-evaluate a candidate policy against logged bandit rows (LCB +
    /// coverage). Read-only inspection of the Layer-B RL gate.
    #[command(
        after_help = "EXAMPLES:\n  kavach db ope-evaluate --allow 0.7 --ask 0.2 --block 0.1 --limit 500\n\nWHEN: Off-policy-evaluate a candidate Allow/Ask/Block policy against logged bandit rows."
    )]
    OpeEvaluate {
        /// Candidate P(Allow).
        #[arg(long)]
        allow: f64,
        /// Candidate P(Ask).
        #[arg(long)]
        ask: f64,
        /// Candidate P(Block).
        #[arg(long)]
        block: f64,
        /// Max bandit rows to load (newest first).
        #[arg(long, default_value_t = 500)]
        limit: u32,
        /// z-score for the lower confidence bound (1.96 ≈ 95%).
        #[arg(long, default_value_t = 1.96)]
        z: f64,
        /// Coverage floor in [0,1]; below it the estimate is flagged untrustworthy.
        #[arg(long, default_value_t = 0.2)]
        min_coverage_ratio: f64,
    },
    /// Reward-hacking audit: SOFT held-out value vs HARD witness value drift +
    /// C2 floor. Read-only inspection of the Layer-P5 promotion gate.
    #[command(
        after_help = "EXAMPLES:\n  kavach db ope-audit --limit 500 --drift-tolerance 0.05\n\nWHEN: Reward-hacking audit — SOFT vs HARD value drift on the Layer-P5 promotion gate."
    )]
    OpeAudit {
        /// Max bandit rows to load (newest first).
        #[arg(long, default_value_t = 500)]
        limit: u32,
        /// Drift slack before SOFT-vs-HARD divergence counts as hacking.
        #[arg(long, default_value_t = 0.05)]
        drift_tolerance: f64,
    },
    /// Record a harness execution run row (status lifecycle start).
    #[command(
        after_help = "EXAMPLES:\n  kavach db run-record --project P --entry-key roadmap.unit.foo --status running --pid 12345\n\nWHEN: Open a harness-execution run row at start of a dispatched task."
    )]
    RunRecord {
        /// Project slug.
        #[arg(long)]
        project: String,
        /// Roadmap entry key this run executes.
        #[arg(long)]
        entry_key: String,
        /// Git branch the run targets.
        #[arg(long)]
        branch: Option<String>,
        /// Initial status (e.g. `running`).
        #[arg(long)]
        status: String,
        /// OS process id, when known.
        #[arg(long)]
        pid: Option<i64>,
    },
    /// Update a run row's terminal status + exit code.
    #[command(
        after_help = "EXAMPLES:\n  kavach db run-update-status --id run:abc --status done --exit-code 0\n\nWHEN: Close a run row with its terminal status + exit code."
    )]
    RunUpdateStatus {
        /// Run row id.
        #[arg(long)]
        id: String,
        /// New status (e.g. `done`, `failed`).
        #[arg(long)]
        status: String,
        /// Process exit code, when terminal.
        #[arg(long)]
        exit_code: Option<i64>,
    },
    /// Purge an `anti_pattern` cluster + its `mistake_event`s by the gate that
    /// recorded them — removes a stale `correct_action` from `PRACTICE_DELTA` (requires `--confirm`).
    #[command(
        after_help = "EXAMPLES:\n  kavach db mistake-purge --gate capture_finding_unpersisted --confirm\n\nWHEN: Remove a stale anti-pattern cluster recorded by a gate (clears it from PRACTICE_DELTA)."
    )]
    MistakePurge {
        /// Gate whose `anti_pattern` cluster(s) to delete (e.g. `capture_finding_unpersisted`)
        #[arg(long)]
        gate: String,
        /// Confirm the destructive purge.
        #[arg(long)]
        confirm: bool,
    },
    /// Run a read-only `SurrealQL` query against the store (`SELECT`/`INFO` only).
    /// Ad-hoc graph inspection; mutations go through typed verbs (write, mistake-purge, …).
    #[command(
        after_help = "EXAMPLES:\n  kavach db query-raw --query 'SELECT name FROM entity LIMIT 5'\n\nWHEN: Ad-hoc read-only SurrealQL (SELECT/INFO only); mutations go through typed verbs."
    )]
    QueryRaw {
        /// The read-only `SurrealQL` to execute, e.g. `SELECT name FROM entity LIMIT 5`.
        #[arg(long)]
        query: String,
    },
    /// Store an implementation-flow DAG (structured JSON ingest, render-on-read)
    #[command(
        long_about = "Persist an implementation flow as a DAG in the entity graph (store-as-DAG, \
render-on-read). The flow is native graph: a `flow` anchor + `flow_step` nodes joined by \
`contains` and `depends_on` edges — traversable, cycle-checked (a `depends_on` cycle is \
rejected), and embeddable for awareness. Mermaid is a VIEW, not the store: ingest is structured \
JSON `{steps:[{id,label,shape?}],edges:[{from,to}]}`; the optional `--mermaid` source is cached \
for round-trip fidelity but the DAG is the source of truth. Idempotent on (project, key).",
        after_help = "EXAMPLES:\n  \
kavach db flow-add --project P --key build-flow --title 'Build Flow' --steps-json steps.json\n  \
echo '{\"steps\":[{\"id\":\"a\",\"label\":\"compile\"},{\"id\":\"b\",\"label\":\"test\"}],\"edges\":[{\"from\":\"a\",\"to\":\"b\"}]}' | kavach db flow-add --project P --key ci --title CI\n\n\
The DAG renders back to a Mermaid `flowchart TD` via `kavach db flow-show`; project flows are \
injected into session-start context as [FLOW] Mermaid for implementation-order awareness.\n\n\
WHEN: Capture how an implementation HAS to be ordered, so future sessions recall the plan as a graph."
    )]
    FlowAdd {
        /// Project slug the flow belongs to
        #[arg(long)]
        project: String,
        /// Flow key, unique per project (e.g. `auth-flow`)
        #[arg(long)]
        key: String,
        /// Display title
        #[arg(long)]
        title: String,
        /// Path to JSON `{steps:[...],edges:[...]}` (reads stdin if omitted)
        #[arg(long = "steps-json")]
        steps_json: Option<String>,
        /// Optional raw Mermaid source cached for round-trip
        #[arg(long)]
        mermaid: Option<String>,
    },
    /// Render a stored implementation-flow DAG as Mermaid (default) or JSON
    #[command(
        long_about = "Render a stored flow DAG. `--format mermaid` (default) emits a \
`flowchart TD` regenerated from the graph (cached `--mermaid` source is reused only when its \
topology matches, else regenerated — the DAG is authoritative); `--format json` emits the raw \
{steps,edges}. Read-only.",
        after_help = "EXAMPLES:\n  \
kavach db flow-show --project P --key build-flow           # Mermaid flowchart TD\n  \
kavach db flow-show --project P --key build-flow --format json\n\n\
WHEN: Recall the intended implementation order; paste the Mermaid into any renderer to view the graph."
    )]
    FlowShow {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Flow key
        #[arg(long)]
        key: String,
        /// Output format: `mermaid` (default) or `json`
        #[arg(long, default_value = "mermaid")]
        format: String,
    },
    /// Infer `depends_on` edges from card-key sequence tokens (tier backfill)
    #[command(
        long_about = "Derive kanban `depends_on` edges from card-key naming. Cards authored as a \
dotted namespace with a trailing sequence token (`unit.harness-rl.p7`, `...loop-eng.f4`, \
`...phase2`, `...step3`, or a bare trailing number) imply ordering: token N depends on the \
same-namespace sibling with the matching token N-1. HEURISTIC — DRY RUN by default (prints the \
proposal only); pass --apply to write the edges through the daemon. After --apply, re-deploy so \
the tier GUI segregates the cards.",
        after_help = "EXAMPLES:\n  \
kavach db infer-deps --project kavach-rs            # dry run — review proposed edges\n  \
kavach db infer-deps --project kavach-rs --apply    # write the edges\n\n\
WHEN: The DAG tier view collapses every card into TIER 0 because no card declares prerequisites."
    )]
    InferDeps {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Write the inferred edges (default is a dry run that only prints them)
        #[arg(long)]
        apply: bool,
    },
    /// Print the top-priority todo card's exec_prompt to stdout (pipe to an executor model).
    #[command(
        after_help = "EXAMPLES:\n  kavach db next-prompt --project P\n  kavach db next-prompt --project P | <executor-model>\n\nWHEN: Serve the top-priority todo card's exec_prompt to a cheaper executor model."
    )]
    NextPrompt {
        /// Project slug
        #[arg(long)]
        project: String,
    },
}
