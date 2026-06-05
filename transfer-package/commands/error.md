---
name: error
description: Error handling, Result, Option, propagation patterns.
---

# /error — EXECUTE IMMEDIATELY

## Parallel Audit

1. **Propagation** — trace every Result/Option to handler
2. **Error Types** — thiserror, domain errors, categorization
3. **Recovery** — retry, fallback, circuit breaker
4. **Boundaries** — external error mapping, context preservation

## Output

```
[ERROR] date:YYYY-MM-DD

| Category | File:Line | Issue | Fix |
|----------|-----------|-------|-----|

PROPAGATION:[n] TYPES:[n] RECOVERY:[n] BOUNDARIES:[n]
```

## Cross-Invoke

| Need | Invoke |
|------|--------|
| Architecture | `/arch` |
| Bug detection | `/bug-bounty` |
| Database errors | `/data` |
