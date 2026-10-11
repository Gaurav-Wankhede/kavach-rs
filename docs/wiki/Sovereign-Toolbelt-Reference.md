# Sovereign Toolbelt Reference

Kavach replaces legacy, context-polluting POSIX utilities with in-process, AST-aware equivalents designed specifically for AI agents.

---

## 1. `kavach grep` — Polyglot AST Code Search
- Tree-Sitter AST code search across 28 languages (`--def`, `--call`, `--type`).
- Ignores comments and string literals to eliminate false positives.
- Emits clean, 4-space tab-indented outputs with zero context surround bloat.

```bash
kavach grep --def EngineConfig crates/
```

---

## 2. `kavach glob` — Zero-Heap Path Locator
- Replaces `find` and `fd`.
- Gitignore-aware zero-heap search using `ignore::WalkBuilder`.

```bash
kavach glob "*.rs" crates/kavach-engine
```

---

## 3. `kavach json` — Zero-Bloat Dot-Path Projection
- Dot-path projection emitting clean TOON tabular format (`-k`, `-r`).
- Permanently replaces `jq` and `jaq`, cutting context token usage by >65%.

```bash
kavach json ".rust_toolbelt.mapping" ~/.gemini/AGENTS.json
```

---

## 4. `kavach replace` — Atomic Multi-Pattern In-Place Patching
- Replaces `sed` and `sd` using `aho_corasick` and temporary files.
- Guaranteed atomic write without file truncation bugs.

```bash
kavach replace "old_token" "new_token" src/main.rs
```

---

## 5. `kavach cargo` — Compiler Scroll Suppressor
- Automatically routes `kavach cargo test` to `cargo nextest run`.
- Suppresses multi-screen compilation noise, emitting a single summary on pass and isolated diagnostics on failure.

```bash
kavach cargo test
```

---

## 6. `kavach script` — In-Memory Pseudo-Code Engine
- Sub-millisecond Rhai script execution with zero compiler overhead.
- Includes pre-bound web discovery probes (`api_llms_txt`, `api_sitemap`, `api_openapi`, `http_get`).

```bash
kavach script -e 'let res = api_llms_txt("https://docs.anthropic.com"); print(res.len);'
```
