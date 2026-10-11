# Cognitive Memory & Brain-OS

Kavach includes an embedded SQLite / Turso database engine running in-process. It stores cross-session knowledge scoped per repository.

---

## 1. Structured Knowledge Tables

- **`decisions`**: Invariable architectural choices. Never re-litigated across turns.
- **`mistakes`**: Anti-pattern ledger that clusters past mistakes by semantic similarity.
- **`roadmap`**: Dependency-aware task cards and execution status.
- **`patterns`**: Gate exceptions and learned false-positive resolutions.

---

## 2. Hybrid Retrieval (`kavach think`)

When an agent needs context, `kavach think <query>` executes vectorless, explainable retrieval in under 2ms:
1. **BM25 Full-Text Search**: Queries across all memory tables.
2. **Knowledge Graph Traversal**: Traverses L0 global concepts and L1 project entities.
3. **Reciprocal Rank Fusion ($k=60$)**: Fuses ranked lists into deterministic, citable row keys (e.g., `decision.storage.turso_sqlite_embedded`).

```bash
kavach think "auth token migration"
```
