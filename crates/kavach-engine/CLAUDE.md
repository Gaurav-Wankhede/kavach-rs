<claude-mem-context>

</claude-mem-context>

# kavach-engine — Gate Severity Policy

Each new gate must declare a severity tier. The host hook layer maps tier → action.

| Tier | Hook action | When to use |
|---|---|---|
| `P0Block` | `kavach_hook::exit_pre_tool_deny` (Bash) / return `block:` from `pre_write_guards::check` | Irreversible: destructive shell op, SQL injection, banned crypto, RLS bypass, secret leak |
| `P1Confirm` | `kavach_hook::exit_pre_tool_ask` (Bash only) | Reversible-but-risky: `sudo rm`, `shutdown`, `history -c`, kernel module |
| `P1Advisory` | `p1_advisories.push(...)` | Quality nudge: SOLID structure, DSA Big-O traps, microservice composition, frontend a11y |
| `P2Warning` | `p1_advisories.push(...)` (lower-prio) | Style hint: format-in-loop, no with_capacity, hash choice |

**RULE — default to advisory.** New gates default to `P1Advisory` unless the violation is irreversible AND the false-positive rate is provably <1%. Promote to `P0Block` only with a regression test demonstrating the false-positive bound.

## Wiring map (which gate fires at which lifecycle)

| Gate (in `kavach-patterns`) | Hook event | Severity → action |
|---|---|---|
| `destructive_cli_guard` | PreToolUse:Bash (`pre_tool_bash/mod.rs`) | P0→deny, P1→ask |
| `is_blocked` (literal cloud-API list) | PreToolUse:Bash | deny |
| `db_security_guard`, `owasp_guard`, `crypto_guard`, `gnap_guard`, `frontend_security_guard`, `pre_write_sql_guard` | PreWrite (`pre_write_guards::check`) | block on hit |
| `solid_guard`, `dsa_guard`, `system_design_guard`, `atomic_ui_guard`, `rust_196_guard`, `dioxus_guard`, `axum_guard`, `api_management_guard`, `complexity_guard`, `algo_complexity_guard`, `secrecy_guard`, `alloc_guard`, `a11y_guard`, `banned_css_guard`, `ux_guard`, `api_gateway_guard`, `infra_guard`, `response_guard`, `microservice_guard`, `rust_guard`, `ts_guard` | PreWrite | advisory (or microservice → block) |
| `database_ops_guard` | PreWrite | P0→block, P1/P2→advisory |
| `algo_selection` | n/a — pure data lookup | not a gate |

## How to add a new gate

1. Implement the detector in `kavach-patterns/src/<name>_guard.rs` returning `Vec<Violation>` or `Option<Hit>`.
2. Wire it into `pre_write_guards.rs` (PreWrite-time) or `pre_tool_bash/mod.rs` (Bash-time) per the table above.
3. Default severity = `P1Advisory`. Justify any P0 in a `// SAFETY:` comment with the false-positive bound.
4. Add a regression test that exercises the gate via the engine entry point — not just the pattern in isolation.
5. Update this Wiring Map block.

## Don't break the chain

- Never add a `block:` return in the advisory section without explicit user request.
- Never call `exit_pre_tool_deny` for advisory tier — use `exit_pre_tool_ask` for P1Confirm or just push to `p1_advisories`.
- The chain runner short-circuits on the first `block:` — order P0 hard-blocks before P1 advisories.