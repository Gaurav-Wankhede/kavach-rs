---
name: bug-bounty
description: Offensive bug hunting. Taint analysis, silent failures, attack surface.
---

# /bug-bounty — EXECUTE IMMEDIATELY

## Hunt Protocol

1. **Scope** — `git diff --name-only HEAD~5` or all source files
2. **Taint Analysis** — trace user input → sinks (SQL, shell, HTML)
3. **Error Propagation** — trace Result/Option to handlers
4. **Resource Lifecycle** — acquisition → release paths
5. **Attack Surface** — unbounded ops, TOCTOU, fail-open
6. **Report** — every bug documented with file:line

## Output

```
[BOUNTY] date:YYYY-MM-DD scope:[N files]

| # | Type | File:Line | Issue | Severity | 
|---|------|-----------|-------|----------|

P0:[n] P1:[n] TAINT:[n] RESOURCE:[n]
```

## Cross-Invoke

| Need | Invoke |
|------|--------|
| SQL patterns | `/data` |
| Error types | `/error` |
| Test coverage | `/verify` |
| Architecture | `/arch` |
