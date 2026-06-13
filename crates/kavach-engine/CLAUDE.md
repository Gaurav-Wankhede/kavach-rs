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
| `loophole_guard` (in-engine; `post_write_checks.rs`) | PostWrite content scan | advisory — injects `[LOOPHOLE_CHECK]` when a completion claim touches a risk-bearing path (auth/lease/lock/money/persist/concurrency/state). IMPERATIVE/fix-first: each lens must be CLOSED at file:line this turn or FILED as a card; the satisfying marker is `Loopholes closed:` (a passive `considered:` no longer satisfies the Stop variant) |
| `db_security_guard`, `owasp_guard`, `crypto_guard`, `gnap_guard`, `frontend_security_guard`, `pre_write_sql_guard` | PreWrite (`pre_write_guards::check`) | block on hit |
| `solid_guard`, `dsa_guard`, `system_design_guard`, `atomic_ui_guard`, `rust_196_guard`, `dioxus_guard`, `axum_guard`, `api_management_guard`, `complexity_guard`, `algo_complexity_guard`, `secrecy_guard`, `alloc_guard`, `a11y_guard`, `banned_css_guard`, `ux_guard`, `api_gateway_guard`, `infra_guard`, `response_guard`, `microservice_guard`, `rust_guard`, `ts_guard` | PreWrite | advisory (or microservice → block) |
| `database_ops_guard` | PreWrite | P0→block, P1/P2→advisory |
| `rust_guard` env_var arm (§CENTRALIZED_CONFIG) | PreWrite | **P0→block** — raw `env::var` in governed `crates/{core,api,services}` (exempt: config_fragments/dotenvy/`main.rs`/startup); P0 routed via `pre_write_rust_guard::block`, P1 via `…::advisory`. FP-bound proven in `env_var_test.rs::false_positive_set_is_empty`. |
| `dedup_guard` (§DEDUP recall-not-redefine) | PreWrite | **P0→block** — a governed file redefines a name it already imports (shadow of a central export); routed via `guards2026::dedup` (after `migration`, before `webhook`). No P1 tier. FP-bound proven in `dedup_guard_test.rs`; engine-entry test `guards2026_test.rs`. |
| `production_patterns` (`pre_write_guards::production_audit`) | PreWrite | advisory — compact `[PRODUCTION_AUDIT_P1]` rollup naming hit count + codes; NOT a block (overlaps P0 security guards, so blocking risks FP storm) |
| `shallow_verdict_guard` (`stop.rs` advisory path) | Stop | advisory + mistake-ledger — `clean/wired/no-defect/safe` verdict with no `file:line` + no `[RCA]`; ride-along on clean_exit, recorded at the computation site so it fires on every stop. NO HALT (no-block policy) |
| `rust_guard` index 78 (`let _name = …`, in `multiline_core/discard_race.rs`) | PreWrite | P1 advisory — general named-underscore discard; RAII names filtered per-match (engine has no regex lookaround) |
| `algo_selection` | n/a — pure data lookup | not a gate |

### Defined-but-never-enforced audit (loophole class)

`pub mod` detectors with ZERO gate call sites are a recurring loophole: the
pattern exists but nothing invokes it. As of the last audit, ALL detectors in
`kavach-patterns` have ≥1 engine reference. The two that were dark
(`production_patterns`, `shallow_verdict_guard`) are now wired (rows above). When
adding a detector, wire its call site in the SAME change and add its Wiring Map
row — an unwired detector is dead code (`dead_code = deny` won't catch it because
the `pub` export keeps it "used").

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