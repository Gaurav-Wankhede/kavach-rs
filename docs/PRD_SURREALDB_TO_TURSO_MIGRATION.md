# PRD: Migrating Kavach Memory from SurrealDB to Turso (libSQL)

> **From:** Kavach with a standalone SurrealDB server (`kavach servers`; every gate/RPC process is a thin WebSocket client; fail-closed when the server is unreachable).
> **To:** Kavach 2.0 — an **embedded Turso/libSQL** database (`~/.kavach/kavach.db`). No daemon, no WebSocket hop, no connection state. Native FTS5 (BM25), typed-graph edges via recursive CTEs, and — new capability — native vector search for semantic memory.

**Status:** ACTIVE — Phase 0 complete (2026-08-26); Phase 1 starting
**Author:** ox-alpha (Hermes)
**Date:** 2026-08-26
**Precedent:** This is the second SurrealDB→Turso migration in this ecosystem. WebFind completed the identical move (`docs/PRD_SURREALDB_TO_TURSO_MIGRATION.md`); its tactics are reused here with kavach-specific adaptations. Additionally, git history shows kavach itself previously migrated SQLite→SurrealDB (the removed `dual_write.rs` bridge, 2026-05) — the round-trip validates embedded-SQL ergonomics for this workload.

---

## 1. Why migrate

### 1.1 What SurrealDB gives kavach today (measured, not assumed)

Grep audit of `crates/kavach-surreal/src/`:

| Feature | Usage | Where |
|---|---|---|
| Typed graph edges | 32× `RELATE` statements | `backfill-relationships`, graph layer: `depends_on`, `blocks`, `supersedes`, `references`, `mentions` |
| BM25 full-text relevance | 10× `search::score(...)`, 1× `bm25`, 1× `DEFINE ANALYZER` | Brain-OS hybrid retrieval |
| Relational memory rows | projects / parts / memories / events / kanban status / priorities / lanes | all of `kavach db *` |
| Server topology | Standalone WS server; every gate invocation opens a client connection | `connection.rs`, gates fail-closed when unreachable |

**No vector indexes are used today.** Mistake-ledger "embedding clustering" and Brain-OS recall are two-signal (BM25 + concept-graph RRF). Vectors are a *new* capability this migration unlocks.

### 1.2 Pain points being removed

1. **Fail-closed fragility** — if the SurrealDB server is down, gates deny by default. A background guardian should degrade gracefully, not wedge the agent.
2. **IPC cost on every gate call** — each of the ~200ms intent-gate invocations pays a WS connect + query round-trip. Embedded storage removes the hop entirely.
3. **Operational surface** — a second long-running service to start/stop/monitor/upgrade (`kavach servers`).
4. **Backup ceremony** — `surreal export` vs. a file copy.

### 1.3 What Turso/libSQL provides in exchange

| Need | libSQL primitive | Notes |
|---|---|---|
| Relational rows | Plain SQLite tables + WAL | Zero learning curve |
| Graph edges | Edge table + recursive CTEs | `WITH RECURSIVE traverse(...)` replaces `RELATE` traversals |
| BM25 | **FTS5 virtual tables** with built-in `bm25()` ranking function | Porter/Unicode tokenizers replace SurrealDB's analyzer |
| Vectors (NEW) | `F32_BLOB(768)` columns, `vector_distance_cos()`, DiskANN index via `CREATE INDEX ... ON t(libsql_vector_idx(embedding, 'metric=cosine'))`, ANN queries via `vector_top_k()` | Full-scan cosine is fine below ~10K rows; our corpus is well under that today, DiskANN covers growth |
| Embedding model | Local (fastembed/Ollama) or API | Owner prefers local-first; decision needed §8 |
| Remote sync (optional, later) | libSQL embedded replicas / `turso` cloud | Same file format, no schema change |

Scale reality: kavach's memory corpus (hundreds–low-thousands of rows across ~10 projects) sits comfortably in SQLite's sweet spot.

---

## 2. Non-goals

- No change to gate semantics (allow/block/ask verdicts, exit codes, CC wire dialect) — only where state lives.
- No change to CLI UX: `kavach db write/query/search/get/graph-query/kanban ...` keep exact flags.
- No multi-machine replication in v1 (libSQL replicas documented as a later option, not built).
- No removal of SurrealDB code in the migration release itself — it goes behind a feature flag, removed one release later after a clean cycle.

---

## 3. Target architecture

```
~/.kavach/
├── kavach.db          ← single embedded libSQL file (WAL mode: memories, edges, entities, FTS5)
├── kavach.db-wal      ← checkpoint hygiene required (see §7 Ops)
├── backups/           ← file-copy snapshots, keep N
└── cache/
    └── ccr/           ← reversible CCR blob store (SHA-256 raw tool outputs, 7-day TTL)
```

```
  Agent (Claude Code / Antigravity / Cursor)
      │
      ├─ [PreToolUse Gate] ────────────► Transparent `rtk rewrite` (e.g. `cargo test` → `rtk cargo test`)
      ▼
  Kavach Local Token Proxy (`kavach proxy` on 127.0.0.1:8787)
      │
      ├─ [Lossless Compactor] ─────────► Tabular JSON array → TSV/CSV format (30–60% token reduction)
      ├─ [RTK Stream Filter] ──────────► `rtk pipe -f <filter>` on raw logs & test runs
      ├─ [CCR Cache & Retrieve] ───────► Oversized blobs (>2KB) stored in cache/ccr/, injects `kavach_retrieve`
      ├─ [CacheAligner] ───────────────► Enforces byte-stable system prompt prefix for 90%+ KV cache hit
      ▼
  Upstream LLM Provider (Anthropic / OpenAI / Bedrock)
```

```sql
-- Core memory row (replaces SurrealDB memory tables)
CREATE TABLE memories (
  project     TEXT NOT NULL,
  category    TEXT NOT NULL CHECK (category IN ('decision','research','roadmap','pattern','app_spec','mistake')),
  key         TEXT NOT NULL,
  title       TEXT NOT NULL,
  content     TEXT NOT NULL DEFAULT '',
  status      TEXT NOT NULL DEFAULT 'todo',        -- todo|in_progress|done|verified (+ archived)
  priority    INTEGER,                             -- NULL = unprioritized (sorts last)
  lane        TEXT,
  created_at  INTEGER NOT NULL,                    -- unix seconds
  updated_at  INTEGER NOT NULL,
  embedding   F32_BLOB(768),                       -- NEW: semantic layer
  PRIMARY KEY (project, category, key)
);

-- RELATE replacement: one table, five relationship kinds
CREATE TABLE edges (
  src_project TEXT NOT NULL, src_cat TEXT NOT NULL, src_key TEXT NOT NULL,
  dst_project TEXT NOT NULL, dst_cat TEXT NOT NULL, dst_key TEXT NOT NULL,
  kind        TEXT NOT NULL CHECK (kind IN ('depends_on','blocks','supersedes','references','mentions')),
  created_at  INTEGER NOT NULL,
  PRIMARY KEY (src_project, src_cat, src_key, dst_project, dst_cat, dst_key, kind)
);
CREATE INDEX idx_edges_dst ON edges(dst_project, dst_cat, dst_key);

-- BM25 replacement for the SurrealDB analyzer+search::score pair
CREATE VIRTUAL TABLE memories_fts USING fts5(
  title, content, content='memories', content_rowid='rowid',
  tokenize='porter unicode61'
);

-- Events log (was: event append + rotate/archive)
CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  project TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT,
  created_at INTEGER NOT NULL
);

-- Projects & parts registry
CREATE TABLE projects (slug TEXT PRIMARY KEY, path TEXT UNIQUE, tags TEXT);
CREATE TABLE parts (project TEXT, part TEXT, path TEXT, PRIMARY KEY(project, part));
```

**Graph traversal pattern** (replaces `RELATE` walks):
```sql
-- Transitive depends_on closure, depth-limited:
WITH RECURSIVE deps(key) AS (
  SELECT dst_key FROM edges
   WHERE src_project=? AND src_cat=? AND src_key=? AND kind='depends_on'
  UNION ALL
  SELECT e.dst_key FROM edges e JOIN deps d
    ON e.src_key=d.key AND e.kind='depends_on' AND e.src_project=?
   WHERE depth < 16
) SELECT DISTINCT key FROM deps;
```
Centrality/graph scores become a **periodic recompute job** (on-write debounce, like WebFind's PageRank cache) rather than live traversal.

---

## 4. Phased execution plan

### Phase 0 — Parity fixtures (golden master) — ✅ COMPLETE 2026-08-26
- [x] `scripts/migration/capture-fixtures.sh`: dumps deterministic outputs of `db query`, `db search`, `db get --full`, `db graph-query`, `db kanban` per project.
- [x] **471 fixtures** under `tests/fixtures/turso-parity/` (9 projects + `_global` graph; throwaway `kavach-test-fix` excluded). Normalization: timestamps → `<TS>`/`<EPOCH>`, relative ages → `<AGO>` — so cross-engine diffs are byte-stable.
- [x] Latency baseline (`_latency-baseline.txt`, N=50): `pre-tool` p50=36ms p95=55ms p99=199ms · `intent` p50=70ms **p95=898ms** p99=1125ms. The intent tail is the SurrealDB WS hop + recall fan-out; Phase 5's target is intent p95 ≤ 400ms with the third (vector) signal added.

### Phase 0b — Findings feeding Phase 1
- [x] Source audit: ~30 storage modules / ~110 public fns in `kavach-surreal`; the RPC/gate layers consume a narrow slice through `read.rs`/`write.rs`/`delete.rs`/`projects.rs`/`parts.rs`.
- [x] Real `MemoryEntry` shape (from `dual_write.rs`) carries MORE columns than first sketched: `entry_key`, `entry_status` + `status`, `tags: Vec<String>`, `decay_score`, `access_count`, `priority:i64`, timestamps. Schema DDL in §3 must be regenerated from this struct, not from the sketch.
- [x] `MemoryEntry.id` uses `surrealdb_types::RecordId` — the Turso DTO must replace it with a `(project, category, key)` composite; keep the old type behind the SurrealStore only.
- [x] Prior art in-tree: removed 2026-05 SQLite→Surreal dual-write bridge lived in `dual_write.rs` (git history has its shape).

### Phase 1 — Schema & store trait — ✅ COMPLETE 2026-08-26
- [x] `libsql = { version = "0.9", features = ["core"] }` added to `kavach-surreal` (crate rename deferred — cosmetic, no behavior gain).
- [x] `MemoryStore` trait deferred in favor of the repo's single-inherent-impl lint: `TursoStore` lives in `src/turso.rs` + `src/turso/ops.rs` with ONE inherent impl; SurrealStore untouched. Trait extraction re-scoped to Phase 3 when both stores must sit behind common call sites.
- [x] Schema DDL finalized from the real `MemoryEntry` struct (17 columns incl. tags/decay/access_count/priority/lane/exec_prompt/lease fields) + `edges`, `projects`, `parts`, `events`, and the Phase-5 `memories_fts` + `embedding F32_BLOB(768)` provisioned up front.
- [x] Ops implemented: `upsert_entry` (ON CONFLICT + FTS sync), `get_by_key`, `list_by_project/all/by_category/with_filter`, `update_status(+CAS)`, `set_priority/lane`, `append_event`, `rotate_events`.
- [x] **189/189 tests green** (6 new turso tests incl. 2 proptests); clippy pedantic+nursery clean on the module.

### Phase 1 lessons (apply going forward)
- libsql `execute()` runs exactly one statement per call → DDL is a `SCHEMA_DDL` array executed statement-by-statement.
- `PRAGMA journal_mode=WAL` returns a row → needs a drain helper (`exec_drain`), not `execute()`.
- Toolchain drift hazard: an unpinned `nightly` channel in `rust-toolchain.toml` broke dependency C++ builds (`diskann-wide`, RocksDB). Restored the committed `1.97.1` pin — keep it pinned.

### Phase 2 — Search parity — ✅ COMPLETE 2026-08-26
- [x] FTS5 sync: `upsert_entry`/`delete_by_key` maintain `memories_fts` (contentless-external approach replaced by explicit dual-write; simpler, transactional within one connection).
- [x] `fts_rank_category()` = parity of `brain_query::fts_rank_one`: one BM25 rank list per category (`ORDER BY bm25(memories_fts)` ascending), injection-safe MATCH construction (per-token quoting, proptest-covered).
- [x] `search_corpus()` = parity of `brain_query::search_corpus`: 5 category rank lists → existing `rrf_fuse` → `BrainHit`s. RRF core reused unchanged — engine-agnostic as designed.
- [x] **12/12 turso tests green** (5 new: title/content hits, relevance-vs-insertion order, cross-category fusion, empty-query no-op, quote-injection safety + property test); clippy pedantic+nursery clean.

### Phase 3 — Graph parity — ✅ COMPLETE 2026-08-26
- [x] `GraphStore` trait (`turso/graph.rs`): entity CRUD, allowlist-guarded `relate_dynamic` (all 33 ALLOWED_RELS mirrored), edge delete, forward/backward hops, depth-capped recursive-CTE closure, `get_related` (single JOIN replaces SurrealDB's per-rel query fan-out), `list_edges_among` (double-IN set filter).
- [x] Renderer parity with the RPC handlers: `render_entity_line` (graph-query list), `render_named_lines` (`──rel──►` shape + `(no edges)` sentinel), byte-asserted in tests against real fixture lines.
- [x] Backfill note: `edges` memory-row table + `upsert_relationships` projection ported as part of EntryStore/GraphStore surfaces; bulk backfill CLI lands with Phase 4's `db migrate` (same import path).
- [x] **202/202 crate tests green** (10 graph tests); clippy pedantic+nursery clean.

### Phase 4 — Migration command `[1.5 days]` — ✅ COMPLETE 2026-09-04
- [x] `kavach db migrate --verb <export|import|verify> --from <export.ndjson> --to ~/.kavach/kavach.db`
- [x] Source side: live SurrealDB dump via WebSocket client → normalized NDJSON (3,634 memories, 15,365 entities, 18,322 edges, 12 projects, 1 part).
- [x] Validation pass post-import: in-memory multi-table endpoint resolution (memories, entities, citations, projects), 0 dangling edges, exact count parity, FTS5 sync.
- [x] Non-destructive guarantee: read-only against SurrealDB source.

### Phase 5 — Vector upgrade (the payoff) ✅ COMPLETE

- [x] Embedding pipeline: `embed_one(title, content)` called in `upsert_entry` via `spawn_blocking`; `kavach db embed-backfill` command for existing rows. (`vector.rs`, `entries.rs`, `embed_backfill.rs`)
- [x] Model: `fastembed::EmbeddingModel::BGESmallENV15` (384-dim, MIT). Schema locked to `F32_BLOB(384)`. Offline fallback: blake3-seeded deterministic unit vector (CI-safe, no download). Decision row: `phase5.vector.dimension-384-bge-small-en-v1.5`.
- [x] Hybrid recall: FTS5-BM25 ∪ brute-cosine-ANN ∪ graph-expansion → RRF(k=60) implemented in `VectorStore::search_hybrid`. ANN path tries `libsql_vector_idx` opportunistically; falls back to O(N) scan below 10K corpus.
- [x] 14 new vector tests green: serialization, cosine, fallback, embed_texts, write-time embedding, vector_rank, search_hybrid, backfill.
- [ ] Mistake-ledger clustering: kNN over mistake embeddings replaces heuristic grouping. (deferred to Phase 7/8)
- [x] Latency budget: vector ops run in `spawn_blocking` (ONNX is synchronous); intent gate async path unblocked.

### Phase 6 — Cutover & cleanup ✅ COMPLETE
 
- [x] `open_turso_resilient()` added to `rpc_client/resilience.rs` — libSQL direct-open helper (no lock-contention retry needed).
- [x] `kavach db query` fallback path → TursoStore (`EntryStore::list_by_project` + `list_by_category`). Live smoke-tested against `~/.kavach/kavach.db`.
- [x] Remaining `db/*` commands (`get`, `event`, `kanban`, `register`, `write`, `next_prompt`, `wipe_project`, `delete`) → TursoStore fallback.
- [x] `cmd/think.rs`, `cmd/context.rs`, `cmd/verify.rs`, `cmd/phase/*` → TursoStore.
- [x] `kavach doctor` extended: DB integrity check (`PRAGMA quick_check`), WAL size warning (>64MB → advise checkpoint).
- [x] Docs updated (README architecture section).
 
### Phase 7 — Hardening & Ops `[0.5 day]` ✅ COMPLETE
- [x] WAL auto-checkpointing (`PRAGMA wal_autocheckpoint=512`) + `kavach db checkpoint --truncate`.
- [x] Backup & rotate commands: file-copy snapshots with integrity check (`~/.kavach/backups/`, keep 10).
- [x] Fixture re-capture & CI parity gates.

### Phase 8 — Token Optimization & Native In-Process RTK Subsystem `[2 days]`
- [ ] `PreToolUse` transparent command rewrite via `rtk rewrite` (e.g. `cargo test` → `rtk cargo test`, `env` → `rtk env`).
- [ ] Secret & `.env` redaction gate: preserves variable keys while redacting sensitive values and scanning for high-entropy tokens.
- [ ] Native RTK output pipe integration (`rtk pipe -f <filter>`) for streaming log noise compaction (strips compilation loops, preserves errors/verdicts).
- [ ] Lossless Tabular JSON Compactor in `kavach-toon` (re-encodes uniform JSON arrays to TSV, 30–60% token reduction).
- [ ] Reversible CCR (Compress-Cache-Retrieve) disk store under `~/.kavach/cache/ccr/` with `kavach retrieve <hash>` (zero server needed).
- [ ] Ephemeral HTTP Token Proxy (`kavach proxy` on `127.0.0.1:8787` with `--idle-timeout 15m` auto-shutdown).
- [ ] CacheAligner prefix stabilization (guaranteeing 90%+ KV prompt cache hit rate).
- [ ] Output token shaping (dynamic thinking budget downscaling on tool resumes).

---

## 5. Verification strategy

1. **Golden-master diff**: every fixture captured in Phase 0 must byte-match TursoStore output (except documented bm25 absolute-score differences; ordering equality enforced).
2. **Property tests** (repo law): roundtrip (write→get→update→get), edge-integrity invariant (no dangling dst), FTS-content sync invariant, migration idempotence (running twice changes nothing).
3. **Gate soak**: run the full hook suite against a scripted 200-event session transcript on both stores; verdict streams must be identical.
4. **Latency gates**: CI assert p95 gate overhead ≤ baseline +10%; proxy pipeline overhead ≤ 5ms p95.

## 6. Rollout & rollback

- Ship behind `store.backend` config (`auto` → turso if `kavach.db` exists, else surreal) for one release.
- Rollback = set backend to surreal; SurrealDB server untouched throughout (migration reads only).
- The `.db` file is the entire state: backup = stop-free `sqlite3 .backup` / file copy into `~/.kavach/backups/` (keep 10, mirroring tubeforge convention).
- Token proxy is opt-in via `kavach proxy start` / `ANTHROPIC_BASE_URL` routing — unsetting the base URL restores direct provider routing with zero disruption.

## 7. Ops discipline

- **WAL checkpointing**: auto-checkpoint on size threshold (PRAGMA wal_autocheckpoint=512 pages) + explicit `kavach db checkpoint` verb. Lesson learned from tfdb's 207MB-WAL incident.
- **Integrity**: `PRAGMA integrity_check` wired into `kavach doctor`.
- **Single-writer discipline**: gates may run concurrently; use libSQL connection pool + busy_timeout; writes serialized by WAL.

## 8. Open questions for owner review

1. **Embedding model**: local fastembed/BGE (private, offline, recommended) vs API embeddings (higher quality, network dependency)?
2. **Dimension**: standardize 384 (BGE-small) vs 768 (BGE-base)? Bigger corpus → consider 1024+ w/ Matryoshka truncation.
3. **Keep `kavach-rpc` daemon?** **RESOLVED: YES.** Elevate `kavach-rpc` to host the local Token Optimization Proxy (`kavach proxy` on `127.0.0.1:8787`), CCR blob cache coordinator, and single-writer DB manager.
4. **Timeline**: sequential phases (~13 dev-days total) vs. parallelize Phase 1–3 and Phase 7–8?
5. **Fixture scope**: capture goldens for all 10 registered projects or just active ones (gaurav-portfolio, youtube, webfind, kavach-rs)?

## Appendix A — SurrealDB → libSQL syntax cheat sheet

| SurrealDB | libSQL equivalent |
|---|---|
| `DEFINE TABLE memories` | `CREATE TABLE memories (...)` |
| `RELATE a->references->b` | `INSERT INTO edges(src...,dst...,kind='references')` |
| `<-references<-` traversal | recursive CTE over `edges` |
| `DEFINE ANALYZER ... TOKENIZERS` | FTS5 `tokenize='porter unicode61'` |
| `search::score(1.0)` / `bm25` | `bm25(memories_fts)` rank column |
| `SELECT ... WHERE project = $p` | same (positional `?` params) |
| `surreal export` | file copy / `VACUUM INTO` |
