# Anti-Patterns — Tester Detection Guide (Hinglish)

## Code Review Mein Dekho

| Pattern | Issue | Report Kaise Karo |
|---------|-------|-------------------|
| `.unwrap()`/`.expect()` | Panic risk | "Line X pe unwrap hai, error handle nahi ho raha" |
| `todo!()`/`unimplemented!()` | Incomplete code | "Line X pe stub hai, implement nahi hua" |
| Empty `catch {}`/`except: pass` | Swallowed error | "Line X pe error ignore ho raha" |
| Magic numbers | Unclear intent | "Line X pe hardcoded value hai, constant hona chahiye" |
| `_ =>` catch-all | Hidden cases | "Line X pe catch-all hai, cases enumerate nahi hue" |

## Testing Mein Verify Karo

| Check | Kaise Karo |
|-------|------------|
| Happy path | Normal input se expected output |
| Error path | Invalid input se graceful error |
| Edge cases | Boundaries pe behavior |
| Null/empty | Null aur empty values handle ho rahe |
| Concurrency | Race conditions check karo |

## Bug Report Format

```
[BUG] file:line
Issue: Kya galat hai
Expected: Kya hona chahiye tha
Actual: Kya ho raha hai
Steps: Reproduce kaise karo
```

## Database Queries Review

| Check | Issue |
|-------|-------|
| String concat in SQL | SQL injection risk |
| Missing WHERE | Full table scan |
| SELECT * | Unnecessary data fetch |
| No LIMIT | Unbounded results |
| No index hint | Potential slow query |

## Communication — Tester Role

| ALLOWED | NOT ALLOWED |
|---------|-------------|
| "Bug mila line X pe" | "Main fix kar deta hoon" |
| "Test fail ho raha hai" | "Code change karo" |
| "Query slow hai" | "Main optimize kar deta hoon" |
| "Pattern violation hai" | "Edit kar diya" |

Tester role = Identify + Report. Developer fixes.
