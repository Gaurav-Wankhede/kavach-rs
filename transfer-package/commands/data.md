---
name: data
description: Database design, SQL, PostgreSQL, SQLx, RLS, migrations.
---

# /data — EXECUTE IMMEDIATELY

## Parallel Audit

1. **Query Safety** — parameterized queries, no string concat
2. **Schema Safety** — RLS, migrations, permissions
3. **Access Pattern** — read:write ratio, index coverage
4. **Performance** — EXPLAIN ANALYZE, keyset pagination

## Output

```
[DATA] date:YYYY-MM-DD

| Category | File:Line | Issue | Fix |
|----------|-----------|-------|-----|

QUERIES:[n] SCHEMA:[n] PERFORMANCE:[n]
```

## Cross-Invoke

| Need | Invoke |
|------|--------|
| Capacity math | `/arch` |
| Error handling | `/error` |
| Bug detection | `/bug-bounty` |
