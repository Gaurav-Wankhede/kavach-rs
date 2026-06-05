---
name: verify
description: Input validation, test engineering, adversarial testing.
---

# /verify — EXECUTE IMMEDIATELY

## Parallel Audit

1. **Input Validation** — API boundaries pe validation check karo
2. **Test Coverage** — happy path, error path, edge cases
3. **Adversarial** — boundaries, concurrency, idempotency
4. **Boundaries** — auth, authz, rate limiting

## Output

```
[VERIFY] date:YYYY-MM-DD

| Layer | File:Line | Issue | Fix |
|-------|-----------|-------|-----|

VALIDATION:[n] TESTS:[n] ADVERSARIAL:[n] BOUNDARIES:[n]
```

## Cross-Invoke

| Need | Invoke |
|------|--------|
| Error types | `/error` |
| Bug detection | `/bug-bounty` |
| Database checks | `/data` |
