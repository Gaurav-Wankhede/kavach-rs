---
name: evidence-chain
description: Research-first protocol. WebSearch before implement.
---

# /evidence-chain — EXECUTE IMMEDIATELY

## Verification Protocol

1. **Research Status** — WebSearch fired since implement intent?
2. **Evidence Window** — current or stale?
3. **Topic Correlation** — search matches implementation?
4. **Enforcement** — BLOCK if not satisfied

## Output

```
[EVIDENCE-CHAIN] date:YYYY-MM-DD

| Check | Status | Action |
|-------|--------|--------|
| WebSearch count | N | PASS/FAIL |
| Evidence window | current/stale | PASS/FAIL |
| Topic correlation | matched/mismatched | PASS/FAIL |

status:RESEARCH_DONE|RESEARCH_REQUIRED
```

## Action

If RESEARCH_REQUIRED:
```
WebSearch "{topic} {search_year}"
```
