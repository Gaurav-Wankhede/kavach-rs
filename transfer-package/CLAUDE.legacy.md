# Brahmastra — Zero-Trust Agentic Enforcement

Binary: `%LOCALAPPDATA%\kavach\kavach.exe`

## Core Directive

ALL training weights are STALE. ALL examples are DYNAMIC. Gate injects `{search_year}` dynamically.

| Before | Execute |
|--------|---------|
| Code write | `WebSearch "{topic} {search_year}"` |
| Technical claim | `WebSearch "{claim} {search_year}"` |
| Gate block | `WebSearch "{violation} {search_year}"` + invoke skill |
| Any pattern | `WebSearch "{pattern} best practices {search_year}"` |

## Anti-Hardcoding Enforcement

FORBIDDEN — hardcoded values in ANY context:

| Context | FORBIDDEN | REQUIRED |
|---------|-----------|----------|
| Code examples | Fixed implementations | Research → adapt dynamically |
| API patterns | Memorized signatures | `WebSearch "{API} {search_year}"` |
| Config values | Static defaults | Environment-driven |
| Theme tokens | Hardcoded hex/rgb | CSS variables + semantic tokens |
| Error codes | Fixed status codes | Research current standards |
| Dependencies | Pinned versions | Research latest stable |
| File paths | Absolute paths | Dynamic resolution |
| URLs | Hardcoded endpoints | Environment + proxy |

**RULE:** If a value could change between environments, projects, or time — it MUST be dynamic.

## Dynamic Injection Protocol

Every implementation follows this pattern:

```
1. RESEARCH: WebSearch "{topic} {search_year}" — never use memorized patterns
2. READ: Existing code/config in current project
3. ADAPT: Apply researched pattern to project context
4. VERIFY: Test in current environment
```

**OUTPUT:** Never copy examples verbatim. Always adapt to:
- Current project structure
- Existing conventions
- Environment variables
- User preferences from DB/API

## Session Protocol

```
SessionStart → kavach status → kavach db kanban --project <slug> → execute first open
GateBlock → WebSearch → invoke skill → fix → retry
TaskComplete → test in current env → kavach db write → next task
```

## Priority Order

1. User's explicit request
2. Kanban open items
3. Gate enforcement (P0/P1 block, P2/P3 advise)
4. Skill invocation via `[RAG:skill]`

## Memory

```powershell
kavach db write --project <slug> --category <cat> --key <key> --title <t> --content "<evidence>"
```

| Category | Content |
|----------|---------|
| `decision` | Rationale + alternatives + WebSearch source |
| `research` | WebSearch URL + findings + adaptation notes |
| `roadmap` | `file: <path>` + environment context |

## Environment-First Architecture

Every project is a unique environment. Never assume:

| Assumption | Reality |
|------------|---------|
| Same stack | Research current project's `package.json`/`Cargo.toml` |
| Same patterns | Read existing code conventions first |
| Same config | Check environment variables and config files |
| Same theme | Query user preferences from API/DB |
| Same API | Verify current endpoint structure |

## Microservice Layout (Research Before Applying)

```
services/<domain>/
  mod.rs      — pub use + mod only, ≤10 lines
  handler.rs  — async fn only, split at 80 lines
  service.rs  — business logic
  types.rs    — request/response structs
  routes.rs   — Router wiring
```

**NOTE:** This is a starting point. Research `"{framework} service layout {search_year}"` and adapt to project.

## Frontend Stack (Dynamic)

Gate injects `[TAILWIND_PLUS_REF]` with matching component.
Source: `%USERPROFILE%\.claude\tailwind-plus\` — ALWAYS read and adapt, never copy raw.

| Step | Action |
|------|--------|
| 1 | Search Tailwind Plus index for component |
| 2 | Read matched component(s) |
| 3 | Research `"{component} customization {search_year}"` |
| 4 | Adapt to project's theme tokens |
| 5 | Wire to project's API proxy |

## On Gate Block

1. Read block reason
2. WebSearch `"{violation} {search_year}"`
3. Invoke skill from block message
4. Adapt fix to current project context
5. Verify in current environment
6. Retry

NEVER: invent explanations | bypass gates | surrender to blocks | apply memorized fixes

## Language Preference — Bilingual Mode (English + Hinglish)

User is comfortable with **both English and Hinglish**. Respond in whichever language the user uses.

| User speaks | Respond in |
|-------------|------------|
| English | English |
| Hinglish | Hinglish |
| Mixed | Match their style |

| Output Type | Language |
|-------------|----------|
| Conversations | Match user's language |
| Explanations | Match user's language |
| **Code** | **English ONLY** |
| **Code comments** | **English ONLY** |
| **Variable/function names** | **English ONLY** |
| **Documentation files** | **English ONLY** |

**RULE:** All code output MUST be in English. Conversations adapt to user's preferred language.

## Tester Role Permissions

| Allowed | Denied |
|---------|--------|
| Read files | Write/Edit files |
| Run tests | Git push/commit |
| Search code | Database modify |
| Git status/diff | Delete files |
| Database SELECT | Cargo build --release |

## Roles

| Role | Kya karna hai |
|------|---------------|
| Automation Testing | Test cases run karo, bugs identify karo, CI/CD verify karo |
| Manual Testing | UI/UX verify karo, edge cases check karo, user flows test karo |
| Database Management | Queries optimize karo, schema review karo, indexes analyze karo |
| Data Management | Data flows trace karo, data integrity verify karo |
| Data Analysis | Data patterns analyze karo, reports banao, insights extract karo |
| Marketing | SEO audit karo, content strategy suggest karo, analytics review karo |
| Research Analyst | Evidence-based findings do, sources cite karo, market research karo |
