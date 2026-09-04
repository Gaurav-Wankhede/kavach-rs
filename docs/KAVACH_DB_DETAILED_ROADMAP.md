# Kavach Database — Detailed Roadmap with Micro Implementation

**Scope:** `crates/kavach-surreal` embedded DB layer — migrating Kavach memory from **SurrealDB `kv-rocksdb` (WS server, 32 `RELATE` edges, `search::score` BM25, 15 tables v13)** to **Turso/libSQL `kavach.db` (embedded WAL, FTS5 `porter unicode61`, `F32_BLOB(768)` vectors, recursive-CTE graph)**.
**Companion:** `PRD_SURREALDB_TO_TURSO_MIGRATION.md` (phases 0–6) · `Cargo.toml` workspace-lints · `docs/CLI.md`
**Status:** Phases 0–3 ✅ COMPLETE (2026-08-26, 202/202 tests green). This roadmap details **Phases 4–7** at micro-task granularity.
**Date:** 2026-09-01 · **DB file:** `~/.kavach/kavach.db` (`kavach.db-wal` + `backups/`) · **Rust:** 1.96+, `edition = "2024"`, `libsql = "0.9"` (`core` feature)

---

## 1. Snapshot — Where We Are vs Where We're Going

### 1.1 Current (SurrealDB)

```
surreal start ws://127.0.0.1:7710 rocksdb://<DATA_DIR>/kavach.surreal
  ├─ 15 tables (v13): project, session, decision, research, roadmap, pattern,
  │                    app_spec, mistake, entity, entity_edges(33 rels), events,
  │                    rag_trees, parts, runs, bulk_manifest + 32 RELATE edge types
  ├─ Analyzer: DEFINE ANALYZER porter/tokenizers + search::score(1.0) BM25 (10×)
  ├─ Graph: RELATE a->references->b per-type edge tables; traversals via RELATE walks
  └─ Cost: WS connect (~200ms intent-gate p50=70ms p95=898ms) + fail-closed when down
```

### 1.2 Target (Turso/libSQL) — Already Provisioned in `src/turso.rs`

```sql
-- SCHEMA_DDL (turso.rs:64) — executed statement-by-statement (libsql one-stmt rule)
CREATE TABLE memories (
  project TEXT, category TEXT CHECK(... 5 cats ...), entry_key TEXT,
  title TEXT, content TEXT, status TEXT, entry_status TEXT, tags TEXT DEFAULT '[]',
  decay_score REAL, access_count INT DEFAULT 0, priority INT, lane TEXT,
  exec_prompt TEXT, occupied_by TEXT, occupied_until TEXT,
  created_at TEXT, updated_at TEXT, embedding F32_BLOB(768),
  PRIMARY KEY (project, category, entry_key)
);
CREATE TABLE edges (src_project,src_cat,src_key, dst..., kind CHECK(5 kinds), created_at, PK 7-col);
CREATE TABLE projects (slug PK, path UNIQUE, tags);
CREATE TABLE parts (project, part, path, PK 2-col);
CREATE TABLE events (id AUTOINC PK, project, kind, payload, created_at);
CREATE VIRTUAL TABLE memories_fts USING fts5(entry_key UNINDEXED, project UNINDEXED, category UNINDEXED, title, content, tokenize='porter unicode61');
-- + turso/ entries DDL idx_memories_project, graph DDL entities+entity_edges, search DDL
PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=512; PRAGMA busy_timeout=5000;
```

| Surface | SurrealDB | Turso parity | File | Tests |
|---|---|---|---|---|
| Entry CRUD | `read.rs`/`write/*` 5 tables | `turso/entries.rs:EntryStore` 1 table+category | `turso/entries.rs` | 12 incl 2 proptests ✅ |
| Search | `brain_query::fts_rank_one` 5× | `turso/search.rs:SearchStore` 5× filtered `bm25(memories_fts)` + `rrf_fuse` | `turso/search.rs` | 5 green ✅ |
| Graph | `graph/dynamic.rs` 33 RELATE | `turso/graph.rs:GraphStore` 33 `ALLOWED_RELS`, `entity_edges` + recursive CTE | `turso/graph.rs` | 10 green ✅ |
| Migration | `surreal export` manual | `kavach db migrate --from --to` (Phase 4) | — | — |
| Vectors | none | `F32_BLOB(768)` + `vector_top_k()` later (Phase 5) | `turso.rs:83` | — |

**Invariants locked:** `entry_status ∈ {todo,in_progress,done,verified}` per-category table; `category ∈ {decision,research,roadmap,pattern,app_spec}`; every event has `session_id`; every domain row has `project_id`; `kanban_cards.entry_key NOT NULL`; `CATEGORIES = 5` singleton; ONE inherent `impl TursoStore` (repo lint).

---

## 2. Engineering Principles (Non-Negotiable)

1. **One-statement rule:** `libsql::Connection::execute()` runs exactly ONE statement — `SCHEMA_DDL` is an array iterated statement-by-statement; `PRAGMA journal_mode=WAL` needs `exec_drain` row-draining helper.
2. **Single inherent impl:** `TursoStore` has ONE `impl` block in `turso/connection.rs`; domains are extension traits (`EntryStore`, `SearchStore`, `GraphStore`) — repo lint `multiple_inherent_impl = "deny"`.
3. **Fail-closed → degrade gracefully:** SurrealDB gates denied when WS down; Turso is embedded — no daemon to fail. Reads tolerate missing FTS row (rebuild on demand).
4. **WAL hygiene:** `wal_autocheckpoint=512`, explicit `kavach db checkpoint` verb, `doctor` warns WAL>64MB (lesson: tfdb 207MB incident).
5. **3-witness per micro-task:** W1 artifact exists (no `todo!`), W2 `cargo clippy -- -D warnings` pedantic+nursery clean, W3 `cargo test` + golden-master diff passes.

---

## 3. Phase Overview

| Phase | Goal | Effort | Gate |
|---|---|---|---|
| **4 — Migration Command** | `kavach db migrate --from <export.ndjson> --to ~/.kavach/kavach.db` non-destructive, validated, resumable | 1.5 days | 471 fixtures byte-match |
| **5 — Vector Upgrade** | Offline `fastembed-rs` BGE-small 384 (config), `F32_BLOB`, hybrid `FTS ∪ vector ∪ graph → RRF`, mistake clustering | 2–3 days | intent p95 ≤ 400ms w/ 3rd signal |
| **6 — Cutover & Cleanup** | Default `turso` (`KAVACH_STORE=surreal` escape 1 release), `doctor` integrity, docs | 1 day | soak 200-event transcript identical verdicts |
| **7 — Hardening** | WAL/backup/rotate/checkpoint auto-policy, parity-soak CI, fixture capture re-run | 0.5 day | `cargo test` 260+ green |

Dependencies: 4 → 5 (vector needs stable schema) → 6 (cutover needs migration validated) → 7 (hardening after default flip).

---

## 4. Phase 4 — Migration Command (1.5 days, 8 micro-tasks)

### 4.1 Contract

```bash
kavach db migrate --from <export.ndjson> --to ~/.kavach/kavach.db [--validate] [--dry-run] [--overwrite]
# --from: surreal export NDJSON (or direct WS dump via existing client)
# --to:   libSQL file path (default: open_default() path)
# --validate: post-import checks (dangling edges, FTS rebuild, counts) — refuse to mark complete on any mismatch
# --dry-run: parse + validate only, no writes
# --overwrite: allow writing into existing DB (default: refuse if memories non-empty)
```

Non-destructive: never writes to SurrealDB source. Idempotent: running twice changes nothing (ON CONFLICT upsert). Progress: streaming NDJSON (1 JSON object per line), batched transactions.

### 4.2 Micro-Tasks

#### KAV-DB-MIG-01 — NDJSON Export Normalizer (4h)

* **Files:** `crates/kavach-surreal/src/turso/migrate/export.rs` (new) + `src/turso/migrate/mod.rs`
* **Input formats:**
  ```json
  {"table":"decision","record":{"project":"myapp","entry_key":"k1","title":"T","content":"C","entry_status":"verified","tags":["a"]}}
  {"table":"entity_edges","record":{"from_id":"e1","rel_type":"references","to_id":"e2","weight":1.0}}
  ```
* **Logic:**
  - Support both `surreal export` (SurrealQL dump) and direct WS dump (`SELECT * FROM decision`) — detect by first 1KB sniff (`DEFINE` vs `{`).
  - Normalize: `project` as slug string (strip `project:` record prefix), timestamps → RFC3339, `RecordId` → `(project,category,entry_key)` triple, `tags` array → JSON string.
  - Validation: every `entry_key` non-empty, `category` in `CATEGORIES`, `entry_status` in allowed set.
* **Acceptance:** `tests/fixtures/turso-parity/*.ndjson` round-trips through `normalize()` byte-stable; `cargo test migrate_export` 3 cases + 1 proptest (arbitrary tags array).
* **Snippet:**
  ```rust
  pub fn normalize_record(raw: &serde_json::Value) -> Result<NormalizedRecord, MigrateError>;
  pub enum NormalizedRecord { Memory{project:String,category:String,key:String,...}, Edge{..}, Project{..}, Event{..} }
  ```

#### KAV-DB-MIG-02 — Importer: NDJSON → TursoStore (4h)

* **Files:** `crates/kavach-surreal/src/turso/migrate/import.rs`
* **Logic:**
  - Open target `TursoStore::open(to)` (creates if needed) — refuse if `SELECT count(*) FROM memories > 0` and `!overwrite`.
  - Stream NDJSON line-by-line (not loading full file), batch 500 rows per transaction (`BEGIN`/`COMMIT`), `PRAGMA synchronous=NORMAL` inside transaction only.
  - Per record: dispatch to `EntryStore::upsert_entry` (FTS sync already in that method), `GraphStore::upsert_entity`/`relate_dynamic`, `projects/parts/events` direct INSERT.
  - Edge import: `validate_rel()` before insert — unknown rel type → collect as warning, not fatal.
* **Acceptance:** Import `471 fixtures` → `turso.db` then `list_all()` counts match `SELECT COUNT` from WS ground truth per project/category; `cargo test migrate_import_idempotent` (import twice → counts identical).
* **Snippet:**
  ```rust
  pub async fn import_ndjson(store: &TursoStore, reader: impl AsyncRead, opts: ImportOpts) -> MigrateReport;
  pub struct MigrateReport { memories:usize, entities:usize, edges:usize, warnings:Vec<String> }
  ```

#### KAV-DB-MIG-03 — Validation Pass (3h)

* **Files:** `crates/kavach-surreal/src/turso/migrate/validate.rs`
* **Checks (all must pass; report mismatches, refuse to mark complete on any):**
  1. **Counts:** per `(project,category)` counts vs source NDJSON line counts.
  2. **Dangling edges:** `SELECT from_id,to_id FROM entity_edges WHERE to_id NOT IN (SELECT id FROM entities)` → must be empty.
  3. **FTS rebuild:** `DELETE FROM memories_fts; INSERT INTO memories_fts SELECT ... FROM memories` then `SELECT count(*) FROM memories_fts` == `memories` count.
  4. **Entry integrity:** every `memories` row has non-empty `project/category/entry_key/title`.
  5. **Ordering:** `list_by_project` ordering (`priority NULLS LAST, created_at ASC`) byte-matches WS fixture ordering.
* **Acceptance:** `validate()` returns `Ok(Report)` with zero failures on golden import; inject dangling edge → reports `DANGLING_EDGES` and returns `Err`.
* **Snippet:**
  ```rust
  pub async fn validate_store(store: &TursoStore, expected_counts: &Counts) -> Result<ValidateReport, ValidateError>;
  ```

#### KAV-DB-MIG-04 — CLI Wiring `kavach db migrate` (2h)

* **Files:** `crates/kavach-cli/src/commands/db/migrate.rs` (new) + `crates/kavach-cli/src/commands/db/mod.rs` + `docs/CLI.md` (auto-generated via `kavach commands --markdown`, not hand-edited)
* **Clap derive:**
  ```rust
  #[derive(Parser)] struct MigrateArgs {
    #[arg(long)] from: PathBuf,
    #[arg(long)] to: Option<PathBuf>,
    #[arg(long, default_value_t=false)] validate: bool,
    #[arg(long, default_value_t=false)] dry_run: bool,
    #[arg(long, default_value_t=false)] overwrite: bool,
  }
  ```
* **Flow:** `from` existence check → `normalize_record` streaming → `import_ndjson` (or dry-run) → optional `validate_store` → print `MigrateReport` as table/`--json` → exit `0` on success, `1` on validation failure, `2` usage.
* **Acceptance:** `kavach db migrate --help` lists all flags; `kavach db migrate --from fixtures/sample.ndjson --to /tmp/t.db --dry-run --json` emits valid JSON envelope; `--from` missing → exit 2.
* **Register:** add `Migrate` variant to `DbCommand` enum in `crates/kavach-cli/src/commands/db/mod.rs`.

#### KAV-DB-MIG-05 — Progress & Resume (1h)

* **Files:** `turso/migrate/import.rs` (extend)
* **Logic:** Emit progress to stderr `imported 1234/5678 (21%)` every 500 rows; on Ctrl-C (SIGINT) commit current batch then exit with partial report (resume = re-run with same `--from`; ON CONFLICT makes it safe).
* **Acceptance:** Kill mid-import, re-run → final counts correct (idempotent).

#### KAV-DB-MIG-06 — Source-Side `surreal export` Helper (1h)

* **Files:** `scripts/migration/capture-fixtures.sh` (extend, already exists from Phase 0)
* **Logic:** `kavach db migrate --export --to export.ndjson` convenience: shells `surreal export --endpoint ws://127.0.0.1:7710 --user root --pass root` or uses existing WS client (`connection.rs`) to `SELECT *` per table → NDJSON. Documented as optional — direct WS dump is the primary path.
* **Acceptance:** `scripts/migration/capture-fixtures.sh --export` produces NDJSON readable by `migrate --from`.

#### KAV-DB-MIG-07 — Tests: Golden & Property (2h)

* **Files:** `crates/kavach-surreal/src/turso/migrate/tests.rs` (new) + `tests/fixtures/turso-parity/` (reuse 471)
* **Cases:**
  - `golden_import_parity`: import `tests/fixtures/turso-parity/*.ndjson` → `list_by_project`/`get_by_key`/`all_edges_raw` byte-match WS fixtures (timestamps normalized to `<TS>`).
  - `migration_idempotence`: import twice → `SELECT count(*)` unchanged, `updated_at` bumped only once.
  - `dangling_edge_reported`: inject edge to missing entity → `validate` returns `DANGLING_EDGES`.
  - `fts_survives_migrate`: `search_corpus("auth token")` before/after ordering identical.
  - `quote_injection_safe`: NDJSON with `title: '"; DROP TABLE memories; --'` imports safely.
* **Acceptance:** 6 new tests green, `proptest` arbitrary NDJSON (100 cases) no panic.

#### KAV-DB-MIG-08 — Docs & Error Taxonomy (30m)

* **Files:** `crates/kavach-surreal/src/error.rs` (extend `TursoError::Migrate(String)`), `PRD_SURREALDB_TO_TURSO_MIGRATION.md` §4 Phase 4 marked ✅.
* **Errors:** `Migrate(DanglingEdges)`, `Migrate(FtsMismatch)`, `Migrate(CountsMismatch)` map to exit 1; `Usage` to exit 2.
* **Docs:** `README.md` Architecture section updated via next `kavach commands --markdown` pass.

**Phase 4 Exit Criteria:** `kavach db migrate --from export.ndjson --to ~/.kavach/kavach.db --validate` succeeds on golden fixtures, `cargo test --lib kavach-surreal` 208+ green (202→208), `cargo clippy -D warnings` clean, golden diff script `scripts/migration/diff-fixtures.sh` reports 0 mismatches (except documented `bm25()` absolute-score tolerance).

---

## 5. Phase 5 — Vector Upgrade (2–3 days, 8 micro-tasks)

### 5.1 Decisions Locked Before Coding

* **Model:** `fastembed-rs` 0.4 + `BGE-small-en` (384-dim, ~120MB, ONNX, offline, MIT). Alternative `all-MiniLM-L6-v2` is 384-dim too; keep dimension as `const EMBEDDING_DIMS = 384` in `turso.rs` — **changing dims after init requires full reindex** (document, enforce).
* **Schema already provisioned:** `embedding F32_BLOB(768)` in DDL uses 768 placeholder; **fix to 384** before Phase 5 ships (1-line edit in `turso.rs:83` + `F32_BLOB(384)`). Keep `CREATE INDEX ... libsql_vector_idx(embedding, 'metric=cosine')` behind feature flag until backfill done.
* **ANN:** Full-scan `vector_distance_cos()` is fine below ~10K rows (kavach corpus ~1–3K). Add `DiskANN` `CREATE INDEX` only after backfill; ANN query via `vector_top_k()`.

### 5.2 Micro-Tasks

#### KAV-DB-VEC-01 — Embedding Config & Model Cache (3h)

* **Files:** `crates/kavach-surreal/src/turso/vector.rs` (new) + `Cargo.toml` add `fastembed = "4"` (feature `onnx`)
* **Logic:** `VectorConfig { model: "BGE-small-en", dims: 384, cache_dir: dirs::cache_dir()/kavach/embeddings }`. Download on first use, verify SHA256, lazy singleton `TextEmbedding`.
* **Snippet:**
  ```rust
  pub const EMBEDDING_DIMS: usize = 384;
  pub async fn embed(texts: &[String]) -> Result<Vec<Vec<f32>>, VectorError>;
  ```
* **Acceptance:** `fastembed` downloads once, `embed(["hello world"])` returns 384 floats, deterministic.

#### KAV-DB-VEC-02 — Schema Fix & Index DDL (1h)

* **Files:** `crates/kavach-surreal/src/turso.rs:83` + `crates/kavach-surreal/src/turso/vector.rs` DDL
* **Change:**
  ```sql
  -- was: embedding F32_BLOB(768)
  embedding F32_BLOB(384),
  -- new index (after backfill):
  CREATE INDEX IF NOT EXISTS idx_memories_embedding
    ON memories(libsql_vector_idx(embedding, 'metric=cosine'));
  ```
* **Acceptance:** `cargo test` still green, `PRAGMA table_info(memories)` shows `embedding` type `F32_BLOB(384)`.

#### KAV-DB-VEC-03 — Write-Time Embedding (3h)

* **Files:** `crates/kavach-surreal/src/turso/entries.rs:upsert_entry`
* **Logic:** After `INSERT INTO memories` + `memories_fts` sync, if `dims==384`, compute `embedding = embed(&[format!("{} {}", e.title, e.content)])` via `tokio::task::spawn_blocking` (ONNX is CPU-bound) then `UPDATE memories SET embedding=? WHERE project=? AND category=? AND entry_key=?`. Batch path embeds 32 rows at a time.
* **Snippet stays inside existing `upsert_entry` — no new trait.**
* **Acceptance:** `upsert_entry` then `SELECT embedding IS NOT NULL` == true; `cargo test vector_write_time` green.

#### KAV-DB-VEC-04 — Backfill Command (3h)

* **Files:** `crates/kavach-cli/src/commands/db/backfill-embeddings.rs` + `turso/vector.rs:backfill()`
* **CLI:**
  ```bash
  kavach db backfill-embeddings [--project myapp] [--batch 64] [--dry-run] [--json]
  ```
* **Logic:** `SELECT project,category,entry_key,title,content FROM memories WHERE embedding IS NULL LIMIT ?`, embed batch, `UPDATE` per row, progress to stderr. Idempotent (skip non-null).
* **Acceptance:** Fresh DB with 1000 rows → `backfill-embeddings` fills 1000 embeddings, second run fills 0.

#### KAV-DB-VEC-05 — Vector Search API (2h)

* **Files:** `crates/kavach-surreal/src/turso/vector.rs:VectorStore`
* **API:**
  ```rust
  pub trait VectorStore {
    fn vector_search(&self, project: &str, query: &str, limit: usize) -> impl Future<Output=TursoResult<Vec<(String,f32)>>>+Send;
  }
  // SQL: SELECT entry_key, vector_distance_cos(embedding, ?1) AS dist FROM memories WHERE project=?2 ORDER BY dist LIMIT ?3
  ```
* **Acceptance:** `vector_search("how to handle auth token refresh?")` hits `decision.auth.token-refresh` top-3; `cargo test vector_search_parity`.

#### KAV-DB-VEC-06 — Hybrid Recall `FTS ∪ Vector ∪ Graph → RRF` (4h)

* **Files:** `crates/kavach-surreal/src/brain.rs` (extend `brain.think` hybrid)
* **Logic (port existing `brain_query::search_corpus` + new signal):**
  ```rust
  // 3 rank lists:
  let fts_lists: Vec<Vec<String>> = corpus_categories.map(|c| fts_rank_category(project,c,query,limit));
  let vec_list: Vec<String> = vector_search(project, query, limit);
  let graph_list: Vec<String> = expand_via_graph(top_hits, 1_hop); // existing concept-graph expansion
  let fused: Vec<BrainHit> = rrf_fuse([fts_lists, vec![vec_list], vec![graph_list]].concat(), RRF_K=60);
  ```
* **Gap filing:** When `fused.len() < 3`, write `research.gap.*` card (existing `brain.rs` gap-filer) — now triggered by 3-signal consensus, not just FTS.
* **Acceptance:** `kavach think --project myapp "auth token refresh"` returns 3-signal hits with `gap_filed=false` when corpus covers; with unknown query returns `gap_filed=true` + `research.gap.*` card created.

#### KAV-DB-VEC-07 — Mistake-Ledger Clustering via kNN (2h)

* **Files:** `crates/kavach-surreal/src/turso/vector.rs:cluster_mistakes()`
* **Logic:** `SELECT entry_key, embedding FROM memories WHERE category='mistake'` then cosine kNN (k=5) grouping; replace heuristic `mistake_group` with `mistake_cluster_id` edge kind `clustered_with`.
* **Acceptance:** `mistakes.rs` test shows 3 similar mistakes clustered together, distinct mistake isolated.

#### KAV-DB-VEC-08 — Latency Budget & Bench (2h)

* **Files:** `benches/vector_recall.rs` (criterion) + `tests/vector_latency.rs`
* **Budget:** Intent-gate p95 ≤ 400ms with 3rd signal (baseline p95=898ms WS hop removed, so budget is generous — full-scan over <10K rows is <50ms). Assert in CI: `hyperfine "kavach think --project myapp 'test'"` N=50 p95 check.
* **Acceptance:** `cargo bench --bench vector_recall` reports p50 < 100ms, p95 < 250ms on M4; CI gate fails if p95 > 400ms.

**Phase 5 Exit Criteria:** `cargo test` 230+ green (8 vector tests + 12 hybrid), `cargo clippy` clean, `kavach db backfill-embeddings --json` fills all nulls, `kavach think` returns 3-signal RRF hits with explainable `entry_key`s.

---

## 6. Phase 6 — Cutover & Cleanup (1 day, 6 micro-tasks)

#### KAV-DB-CUT-01 — Config `store.backend` (2h)

* **Files:** `crates/kavach-config/src/lib.rs` + `crates/kavach-surreal/src/lib.rs` (dispatch)
* **Logic:**
  ```rust
  // KAVACH_STORE env: "turso" | "surreal" | "auto" (default)
  // auto: if ~/.kavach/kavach.db exists → turso else surreal
  pub fn resolve_store() -> StoreBackend { ... }
  ```
* **Acceptance:** `KAVACH_STORE=surreal kavach db kanban` hits WS; `KAVACH_STORE=turso ...` hits file; `auto` with DB present → turso.

#### KAV-DB-CUT-02 — Default Flip + Escape Hatch (1h)

* **Files:** `crates/kavach-surreal/src/lib.rs`, `justfile` install target, `README.md`
* **Logic:** `just install` switches default to `turso`; keep `surreal` behind flag for one release (next minor removes it). Log `store=turso` vs `store=surreal` at startup trace.
* **Acceptance:** Fresh install with no `kavach.db` + first `kavach db write` creates `kavach.db` and uses it thereafter.

#### KAV-DB-CUT-03 — Doctor Integrity (2h)

* **Files:** `crates/kavach-cli/src/commands/doctor.rs` (extend)
* **Checks:**
  - `PRAGMA integrity_check` → must be `ok`.
  - `SELECT count(*) FROM memories_fts` vs `memories` → must match (or advise `kavach db reindex-fts`).
  - `PRAGMA wal_checkpoint(TRUNCATE)` → WAL size warning if >64MB.
  - `fixtures re-run shortcut`: `kavach doctor --fixtures` re-runs golden diff non-interactively.
* **Acceptance:** `kavach doctor` green on clean DB, reports WAL warning on inflated DB.

#### KAV-DB-CUT-04 — Docs (1h)

* **Files:** `README.md` Architecture diagram (replace Surreal WS box with `kavach.db` WAL), `PRD_SURREALDB_TO_TURSO_MIGRATION.md` Phase 6 ✅, `docs/CLI.md` regenerated.
* **Acceptance:** `kavach commands --markdown > docs/CLI.md` includes `migrate` + `backfill-embeddings`; README diagram shows single file, no daemon.

#### KAV-DB-CUT-05 — Surreal Feature-Gate (1h)

* **Files:** `Cargo.toml` add `features = ["surreal-backend"]` default-on for one release; `#[cfg(feature="surreal-backend")]` guards `kavach-surreal/src/{connection,surreal}` WS code.
* **Acceptance:** `cargo build --no-default-features --features turso-only` (future) still compiles; current `cargo build` with default features compiles both.

#### KAV-DB-CUT-06 — Soak Gate (2h, parallel)

* **Script:** `scripts/soak/gate-soak.sh` — replays 200-event transcript (intent → post_tool → stop gate) against both stores; verdict streams must be byte-identical (allow/block/ask + exit codes + CC wire dialect).
* **Acceptance:** Soak passes, `events` table row-for-row identical.

**Phase 6 Exit Criteria:** Default store is `turso`, `KAVACH_STORE=surreal` escape works, `kavach doctor` green, soak identical, docs shipped.

---

## 7. Phase 7 — Hardening & Ops (0.5 day, 5 micro-tasks)

| ID | Task | File | Acceptance |
|---|---|---|---|
| KAV-DB-OPS-01 | **WAL auto-policy** — `wal_autocheckpoint=512` already set; add `kavach db checkpoint` verb (`PRAGMA wal_checkpoint(TRUNCATE)`) | `turso/connection.rs` + `cli/commands/db/checkpoint.rs` | `kavach db checkpoint --json` truncates WAL, `wal` file 0 bytes after |
| KAV-DB-OPS-02 | **Backup:** file-copy `kavach.db` → `backups/kavach-YYYYMMDD-HHMMSS.db` (sidestep `VACUUM INTO`) + `PRAGMA integrity_check` on copy, keep 10, atomic via `rename` | `crates/kavach-surreal/src/turso/backup.rs` + `cli/commands/db/backup.rs` | `backup → restore → reindex → counts match` |
| KAV-DB-OPS-03 | **Rotate:** `kavach db rotate --days 30` (already `rotate_events`) + `kavach db archive --floor-days 30` alias | `turso/entries.rs` + `cli` | `rotate` deletes rows older than cutoff, `--dry-run` reports count |
| KAV-DB-OPS-04 | **Fixture re-capture:** `scripts/migration/capture-fixtures.sh --turso` dumps Turso golden for future diffs | `scripts/migration/capture-fixtures.sh` | Turso fixtures diff against next release to catch schema drift |
| KAV-DB-OPS-05 | **CI parity gate:** `cargo test --lib -- --ignored perf_gate` + `cargo clippy --all-targets -- -D warnings` must pass on fresh `migrate`+`backfill` DB | `.github/workflows/ci.yml` | CI green on `kavach.db` path |

---

---

## 8. Phase 8 — Token Optimization & Native In-Process RTK Subsystem (2 days, 8 micro-tasks)

### 8.1 Architecture & Strategy

Tool outputs (test suites, build logs, git diffs, JSON arrays, scraped web pages) consume 70–90% of the token context in real agent workflows. Furthermore, raw tool outputs frequently leak sensitive secrets (`.env` values, API tokens, private keys) and flood the context window with compiler/test noise.

Rather than running a fragile 24/7 background daemon, Kavach implements a **dual-choke in-process architecture (Model A: Always-On Zero-Daemon Core)** backed by an **optional ephemeral proxy (Model B: On-Demand with auto-idle shutdown)**:

```
  Agent (Claude Code / Antigravity / Cursor)
      │
      ├─ [1. PreToolUse Gate] ─────────► Transparent Command Rewrite via `rtk rewrite`
      │                                     (e.g., `cargo test` → `rtk cargo test`, `env` → `rtk env`)
      │
      ├─ [2. PostToolUse Gate] ────────► In-Process Secret Redaction + RTK Pipe Filter
      │                                     (Preserves .env keys, masks values; strips log noise)
      │
      ├─ [3. File-Based CCR Engine] ───► Payloads > 1.5K tokens: cache full blob to `~/.kavach/cache/ccr/`
      │                                     Inject compacted excerpt + `<<kavach_ccr:HASH>>`
      │                                     Direct disk recovery via `kavach retrieve <hash>` (zero server needed)
      ▼
  [Optional Ephemeral Proxy: 127.0.0.1:8787] (Starts on-demand, auto-terminates after 15m idle)
      │
      ├─ [4. Lossless Compaction] ─────► Tabular JSON → CSV/TSV schema (SmartCrusher principle)
      ├─ [5. Prefix Alignment] ─────────► Guarantees byte-stable prefix for 90%+ KV-cache hits
      ├─ [6. Output Shaper] ────────────► Clamps thinking budget on routine tool resumes
      ▼
  Upstream LLM Provider (Anthropic / OpenAI / Bedrock)
```

### 8.2 Micro-Tasks

#### KAV-PROXY-01 — PreToolUse Transparent Command Rewrite (3h)
* **Files:** `crates/kavach-hook/src/cc21.rs` + `crates/kavach-engine/src/gates/pre_tool_skill.rs`
* **Logic:** On intercepting a `Bash` / `run_command` tool call:
  1. Inspect the command string `cmd`.
  2. Invoke internal `rtk rewrite "$cmd"` (sub-millisecond Rust check).
  3. If rewritten (e.g. `cargo test` → `rtk cargo test`, `git status` → `rtk git status`, `curl` → `rtk curl`), return `updatedInput: { "command": "<rewritten>" }` in the Claude Code `PreToolUse` response envelope.
  4. Specifically map environment inspections (`env`, `printenv`) to `rtk env` to avoid raw secret dumps.
* **Acceptance:** Agent emits `cargo test`; OS executes `rtk cargo test`; prompt context receives token-optimized test output; zero prompt leakage or external tool syntax required.

#### KAV-PROXY-02 — Native RTK Output Stream Pipeline (2h)
* **Files:** `crates/kavach-engine/src/gates/post_tool_bash/handle.rs`
* **Logic:** For commands executed without `updatedInput` support (background tasks, subagent execution):
  1. Detect command signature (e.g. `cargo test`, `git log`, `pytest`, `npm test`).
  2. Pipe raw stdout through `rtk pipe -f <filter>` before storage or logging.
  3. Compiler headers and progress tickers are stripped; failures, errors, and summary verdicts are preserved byte-for-byte.
* **Acceptance:** 5,000 lines of build output compressed to ~80 lines; test failures and stack traces 100% preserved.

#### KAV-PROXY-03 — Secret & Environment Redaction Layer (info-leak closed) (3h)
* **Files:** `crates/kavach-engine/src/gates/secret_redactor.rs` (new) + `crates/kavach-patterns/src/pii_data_guard.rs`
* **Logic:**
  1. Intercept all bash stdout and file reads targeting `.env*` files or environment listings:
  2. Extract `KEY=VALUE` pairs.
  3. Detect sensitive keys matching `*(SECRET|KEY|TOKEN|PASSWORD|URL|CREDENTIAL|AUTH|PRIVATE)*`:
     - **Preserve the variable name/key** so the model knows the variable exists for code configuration.
     - **Mask the value**: `DATABASE_URL=[REDACTED: postgres://... 48 chars]`, `STRIPE_KEY=[REDACTED: sk_live_... 32 chars]`.
  4. Perform high-entropy secret scan (JWT tokens, AWS `AKIA*`, GitHub `ghp_*`, RSA/PEM private key headers) and redact values across all tool outputs before they reach the LLM context.
* **Acceptance:** Reading a `.env` file exposes all configuration keys to the LLM while redacting 100% of sensitive secret values. Zero credentials in prompt context.

#### KAV-PROXY-04 — Lossless Tabular JSON Compactor (SmartCrusher) (3h)
* **Files:** `crates/kavach-toon/src/compact/tabular.rs` (new) + `crates/kavach-toon/src/compact.rs`
* **Logic:**
  1. Detect if payload is a JSON array of uniform objects (consistent keys across ≥3 items).
  2. If tabular and token count > 100: re-encode into dense TSV / markdown table with column header.
  3. If compaction saves ≥30% of tokens: replace payload with table form.
  4. 100% lossless: no rows or fields dropped, purely syntax compaction.
* **Acceptance:** 1,000-item JSON array compressed by 45–60% with zero information loss; roundtrip deserialization preserves all values.

#### KAV-PROXY-05 — Reversible CCR (Compress-Cache-Retrieve) File Store (3h)
* **Files:** `crates/kavach-engine/src/ccr.rs` (new) + `crates/kavach-cli/src/cmd/retrieve.rs`
* **Logic:**
  1. For any tool output or content block exceeding `CCR_THRESHOLD_BYTES` (default 6KB / ~1,500 tokens):
  2. Compute SHA-256 hash `H`.
  3. Store raw original payload in `~/.kavach/cache/ccr/<H>.blob` (TTL 7 days, zero-server disk file).
  4. Truncate payload for the LLM: keep header, first 25 lines, anomaly/error lines, last 25 lines.
  5. Append marker: `\n[KAVACH CCR: Original (N tokens) cached. Retrieve if needed: <<kavach_ccr:H>>. Run: kavach retrieve <hash>]\n`.
  6. Register `kavach_retrieve` CLI / MCP tool to read `<H>.blob` directly from disk.
* **Acceptance:** Oversized tool results compressed safely; `kavach retrieve <hash>` prints the exact original bytes without requiring any background server.

#### KAV-PROXY-06 — Ephemeral HTTP Token Proxy Server (On-Demand) (3h)
* **Files:** `crates/kavach-rpc/src/proxy.rs` (new) + `crates/kavach-cli/src/cmd/proxy.rs`
* **Logic:**
  1. Lightweight Axum / Hyper HTTP proxy running on `127.0.0.1:8787`.
  2. Designed as an **ephemeral process** with `--idle-timeout 15m`: starts on demand (or via `kavach wrap`), terminates automatically when inactive. Zero 24/7 background service footprint.
  3. Intercepts `POST /v1/messages` (Anthropic) and `POST /v1/chat/completions` (OpenAI).
  4. Unpacks `messages`, walks `tool_result` content blocks, executes KAV-PROXY-03 (Redaction), KAV-PROXY-04 (Tabular), and KAV-PROXY-05 (CCR).
  5. Forwards compacted payload to upstream provider API using streaming passthrough (SSE).
* **Acceptance:** `ANTHROPIC_BASE_URL=http://127.0.0.1:8787` transparently handles agent traffic; automatically exits after 15m idle.

#### KAV-PROXY-07 — CacheAligner Prefix Stabilizer (2h)
* **Files:** `crates/kavach-rpc/src/proxy/cache_align.rs`
* **Logic:**
  1. Inspects system instructions and prompt prefix before forwarding.
  2. Validates that static prompt rules remain byte-stable across wakes.
  3. Relegates ephemeral timestamps and session counters behind `CACHE_BOUNDARY_MARKER` (`<!-- DYNAMIC_CONTENT_BELOW -->`).
  4. Injects provider-native `cache_control: {"type": "ephemeral"}` breakpoints at optimal block boundaries.
* **Acceptance:** Anthropic Prompt Caching cache-read hit rate exceeds 90% across multi-turn sessions.

#### KAV-PROXY-08 — Output Token Shaper (2h)
* **Files:** `crates/kavach-rpc/src/proxy/shaper.rs`
* **Logic:**
  1. Detects turn intent: user-prompt vs error-recovery vs routine-tool-resume.
  2. On routine tool resumes (passing tests, file reads): clamps `thinking.budget_tokens` / `reasoning_effort` down to minimal floor.
  3. On user questions or error recoveries: leaves thinking budget unrestricted.
* **Acceptance:** Eliminates 4,000-token runaway reasoning loops on routine tool acknowledgments; 25–35% output token savings.

**Phase 8 Exit Criteria:** Zero persistent background daemon required. `PreToolUse` transparently rewrites commands via `rtk`, `.env` values and high-entropy secrets are 100% redacted while preserving variable keys, CCR store caches and recovers raw payloads from disk, tabular compactor saves ≥30% on JSON arrays, prompt caching hit rate ≥90%.

---

## 9. Test Matrix (Merged)

| Layer | Suite | Cases | Gate |
|---|---|---|---|
| Unit | `turso/entries_test.rs` | upsert roundtrip, FTS sync, status CAS, priority/lane, delete, proptests | 12 green |
| Unit | `turso/search_test.rs` | title/content hits, relevance vs insertion, cross-cat fusion, empty-query no-op, quote-injection + property | 5 green |
| Unit | `turso/graph_test.rs` | entity CRUD, allowlist, relate/delete, forward/backward, closure capped, get_related JOIN, list_edges_among | 10 green |
| Unit | `turso/vector` | embed dims, write-time, backfill idempotent, vector_search ordering, hybrid RRF | 8 new |
| Unit | `toon/compact/tabular` | tabular detection, JSON to TSV roundtrip, non-tabular bypass, lossless retention | 6 new |
| Unit | `proxy/ccr` | blob storage, hash verification, retrieval tool execution, TTL expiry | 4 new |
| Integration | `migrate/tests.rs` | golden 471 parity, idempotence, dangling edge, FTS survive, quote injection | 6 new |
| Integration | `proxy/e2e` | Anthropic /v1/messages streaming passthrough, CCR injection, RTK rewrite in hook | 4 new |
| Integration | `soak/gate-soak.sh` | 200-event verdict stream identical | 1 |
| Perf | `vector_latency` | intent p95 ≤ 400ms, vector full-scan p50 <100ms | 2 benches |
| Perf | `proxy_latency` | proxy pipeline overhead p95 ≤ 5ms | 1 bench |
| Ops | `backup/rotate/checkpoint` | file-copy + integrity_check, WAL truncate, rotate cutoff | 3 |

**Target:** 254+ tests green (202 → 254+), 0 clippy warnings, p95 overhead ≤ baseline+10%.

---

## 10. CLI Surface (Final)

```bash
# Existing (unchanged)
kavach db kanban --project myapp --limit 10
kavach db get --project myapp --category roadmap --key K --full
kavach db write --project myapp --category roadmap --key K --title T --new

# New in Phase 4–5
kavach db migrate --from export.ndjson --to ~/.kavach/kavach.db [--validate] [--dry-run] [--overwrite]
kavach db backfill-embeddings [--project myapp] [--batch 64] [--dry-run]
kavach db checkpoint [--truncate]
kavach db reindex-fts  # rebuild memories_fts from memories (idempotent recovery)

# Token Proxy & RTK (Phase 8)
kavach proxy start [--port 8787] [--daemon]  # start local token optimization proxy
kavach proxy status                          # check proxy health, CCR cache size, savings
kavach proxy stop                            # shutdown proxy
kavach proxy ccr retrieve <hash>             # inspect cached original blob manually

# Ops (Phase 7)
kavach db backup [--to backups/] [--keep 10]
kavach db rotate --days 30 [--dry-run]
kavach doctor [--fixtures]  # integrity_check + wal size + fixture diff + proxy health
kavach think --project myapp "how did we handle auth token refresh?"  # now 3-signal BM25+vector+graph RRF
```

---

## 11. Risks & Mitigations

| # | Risk | Mitigation |
|---|---|---|
| R1 | `F32_BLOB(768)` vs 384 dim mismatch | Lock `EMBEDDING_DIMS=384` before Phase 5, doc "changing dims requires reindex" |
| R2 | `fts5_query` injection via user tokens | Per-token quoting `fts_escape` already proptest-covered (Phase 2) |
| R3 | WAL bloat (tfdb 207MB) | `wal_autocheckpoint=512` + `kavach db checkpoint` + `doctor` >64MB warning |
| R4 | NDJSON format drift (Surreal export vs WS dump) | Sniff + support both, normalize timestamps/RecordId early |
| R5 | Long `migrate` without progress | Batched TX (500) + stderr progress every 500 lines + Ctrl-C graceful commit |
| R6 | Embedding cost on write path | `spawn_blocking` batch 32, debounce on hot path, backfill offline |
| R7 | Proxy latency overhead | Zero-copy SSE streaming; compaction runs in Tokio async worker; bypass when payload <100 tokens |
| R8 | Lossy truncation breaks LLM reasoning | CCR guarantee: every truncated blob is cached with hash; `kavach_retrieve` restores exact original on demand |

---

## 12. Rollout & Rollback

* Ship storage behind `KAVACH_STORE=auto` for one release (turso if `kavach.db` exists else surreal).
* Rollback storage = set `KAVACH_STORE=surreal` — SurrealDB server untouched throughout (migrate reads only).
* Ship proxy behind optional `kavach proxy start` / `ANTHROPIC_BASE_URL` routing — unset env var instantly falls back to direct provider connection.
* `.db` is the entire state: backup = file copy into `~/.kavach/backups/` (keep 10).
* CCR cache cleanable via `kavach proxy ccr prune --days 7`.

---

## 13. Effort Summary

| Phase | Days | Engineer | Parallelizable |
|---|---|---|---|
| 4 Migration | 1.5 | 1 | 4 & 5 model cache prep |
| 5 Vector | 2.5 | 1 | 5.01 + 5.02 ahead |
| 6 Cutover | 1.0 | 1 | soak parallel |
| 7 Hardening | 0.5 | 1 | after 6 |
| 8 Proxy & RTK | 2.0 | 1 | parallel with 6 & 7 |
| **Total** | **7.5 days** | 1 | seq 13 days if time-boxed, 7.5 days focused |

---

## 14. Next Actions (Owner Review)

1. **Confirm** model lock: `BGE-small-en 384` via `fastembed-rs` (offline, MIT) or alternative? Decision gates Phase 5 start.
2. **Confirm** `EMBEDDING_DIMS` freeze: 384 vs 768 — bigger corpus later → consider 1024+ with Matryoshka truncation (post-v1).
3. **Approve** Phase 4 start (`kavach db migrate` — non-destructive, no Surreal writes).
4. **Approve** Phase 8 inclusion: Local HTTP Token Proxy (`kavach proxy`) + native RTK internal command rewriting & CCR pipeline.
5. **Fixture scope:** re-capture goldens for all 10 projects or active subset? (Current: 9 + `_global`).

---

*Generated from `PRD_SURREALDB_TO_TURSO_MIGRATION.md` Phase 0–3 actuals + `crates/kavach-surreal/src/turso{/,/entries,graph,search}.rs` ship state — every micro-task is ready to dispatch as a `kavach db write --category roadmap` card.*
