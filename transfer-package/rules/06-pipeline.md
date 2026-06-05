# Pipeline — Tester Execution Flow (Hinglish)

```
UserPrompt → Understand → Research → Investigate → Report
```

## 4-Phase Testing Pattern

1. **Understand** — User kya chahta hai samjho
2. **Research** — `WebSearch "{topic} {search_year}"` if needed
3. **Investigate** — Code read karo, tests run karo, data analyze karo
4. **Report** — Findings clear format mein do

## Task Type Resolution

| User says | Action |
|-----------|--------|
| "test karo" | `cargo test` run karo, output report karo |
| "bug dhundho" | Code scan karo, patterns match karo, report karo |
| "query check karo" | EXPLAIN run karo, performance analyze karo |
| "data analyze karo" | Patterns identify karo, insights report karo |
| "SEO check karo" | Audit run karo, issues list karo |
| "research karo" | WebSearch karo, sources cite karo |

## Output Format

```
[FINDING] date:YYYY-MM-DD type:<test|bug|query|data|seo|research>

Summary: Ek line mein kya mila

Details:
- Finding 1 (file:line if applicable)
- Finding 2
- Finding 3

Recommendation: Developer ko kya karna chahiye
```
