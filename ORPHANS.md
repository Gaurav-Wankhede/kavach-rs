# Orphan-code census — kavach-rs

Generated 2026-06-24. Scanned **1574 `pub` symbols across 21 crates**; **76 confirmed orphans** after adversarial per-symbol verification.

## What was checked (3 orphan classes)

| Class | Result |
|---|---|
| **1. Compiler-visible dead code** (private fn/const/import/var) | **0** — workspace enforces `dead_code=deny` + the full `unused` group (`Cargo.toml:113-130`); builds clean, so rustc already eliminates this entire class. |
| **2. Orphan `pub` exports** (the `dead_code`-blind class) | **76** — listed below. rustc treats a `pub` item as "used" the moment it's exported, so these compile clean despite zero real callers. |
| **3. Unused dependencies** | **1 fixed** — `serde` in `kavach-engine/Cargo.toml` (0 derive sites; only `serde_json` used). Removed; build exit 0; machete now clean. |

**Method:** each `pub` symbol cross-referenced workspace-wide with `rg -w`; ref_count excludes the definition; every candidate refuted-or-confirmed by an independent second agent (FP classes checked: trait-method, serde/macro, re-export-consumed, bin-entrypoint, test-only).

**Per-crate:** kavach-session 28 · kavach-types 7 · kavach-patterns 6 · kavach-config 6 · kavach-engine 5 · kavach-chain 5 · kavach-rule-storage 4 · kavach-hook 3 · kavach-dtree 3 · kavach-rule-parser 2 · kavach-rule-engine 2 · kavach-rule-ast 2 · kavach-toon 1 · kavach-surreal 1 · kavach-nlm 1.

---

## A. Truly dead — zero references anywhere (65) — SAFE DELETE candidates

No caller in any crate, test, binary, or dispatch table. Most are abandoned-feature infrastructure (a setter added, integration never wired) or refactored-away stubs whose sibling is the live one.

- **get_fork_agent** (fn) — kavach-patterns — skill_keyword_router.rs:280; fork routing defined, never integrated (`should_fork` also unused).
- **has_blocking_violations** (fn) — kavach-patterns — production_patterns.rs:46; module used but only `scan()`/`count_critical()` called.
- **scan_category** (fn) — kavach-patterns — production_patterns.rs:31; never invoked.
- **skills_for_keywords** (fn) — kavach-patterns — skill_manifest.rs:275; zero refs across all dependents.
- **load_skill_on_demand** (fn) — kavach-patterns — skill_manifest.rs:281; wrapper, never called.
- **W_KANBAN_PHASE** (const) — kavach-patterns — k_pri.rs:103; sibling weights used, this 0.
- **accumulate_subagent_blast** (fn) — kavach-session — subagent.rs:113; fields populated via serde only.
- **add_test_pending** (fn) — kavach-session — enforcement.rs:99; field mutated directly elsewhere.
- **clear_post_compact** (fn) — kavach-session — markers.rs:64; only `mark_post_compact` is live.
- **clear_task** (fn) — kavach-session — task.rs:28; sibling task methods used, this never.
- **deny_tool_for_subagents** (fn) — kavach-session — subagent.rs:141; setter never called.
- **increment_failure_blocks** (fn) — kavach-session — markers.rs:173; doc claims stop-gate calls it; 0 calls, field read for display only.
- **is_transient_failure** (fn) — kavach-session — markers.rs:119; engine uses raw string compare.
- **mark_turn_shadow_pending** (fn) — kavach-session — turn_shadow.rs:81; never called.
- **memory_dir** (fn) — kavach-session — paths.rs:135; re-exported, never consumed.
- **record_failure** (fn) — kavach-session — markers.rs:96; `record_failure_typed` is live.
- **reset_all_circuits** (fn) — kavach-session — enforcement.rs:270; zero call sites.
- **reset_files_read** (fn) — kavach-session — markers.rs:160; field reset by direct assignment in 3 places.
- **reset_gate_circuit** (fn) — kavach-session — enforcement.rs:263; reset path never wired.
- **review_covers_current_diff** (fn) — kavach-session — markers.rs:44; caller removed under "kill blocking, keep auto-continue".
- **scope_narrowing_hint** (fn) — kavach-session — enforcement.rs:284; unfinished gate-feedback.
- **SessionFlags** (struct) — kavach-session — subset.rs:15; `.flags()` never called.
- **set_current_task** (fn) — kavach-session — task.rs:4; never invoked.
- **should_suggest_narrowing** (fn) — kavach-session — enforcement.rs:309; leftover from refactor.
- **SubagentBlast** (struct) — kavach-session — subagent.rs:12; only used by the unused `accumulate_subagent_blast`.
- **TEST_BLOCK_THRESHOLD** (const) — kavach-session — enforcement.rs:96; feature disabled, legacy dead.
- **project_fields** (fn) — kavach-surreal — projects.rs:239; never integrated.
- **run_query** (fn) — kavach-nlm — store.rs:61; crate excluded from workspace (Cargo.toml), unreachable.
- **exit_block** (fn) — kavach-hook — lib.rs:315; siblings `exit_silent/approve/modify` used, this 0.
- **exit_modify** (fn) — kavach-hook — lib.rs:321; dead export.
- **must_read_hook_input** (fn) — kavach-hook — lib.rs:201; never called by any consuming crate.
- **ast_search** (fn) — kavach-engine — toolbelt/search.rs:53; re-exported only, 0 calls.
- **find_files** (fn) — kavach-engine — toolbelt/search.rs:32; exported, unused.
- **http_get** (fn) — kavach-engine — toolbelt/net.rs:31; sibling `verify_url_reachable` used, this 0.
- **json_query** (fn) — kavach-engine — toolbelt/net.rs:10; re-export never consumed.
- **process_list** (fn) — kavach-engine — toolbelt/proc.rs:10; never referenced.
- **find_skill_by_trigger** (fn) — kavach-chain — loader.rs:121; companion skill infra all unused.
- **loaded_skills** (fn) — kavach-chain — loader.rs:133; `loaded_agents` tested, this 0.
- **get_skill_for_agent** (fn) — kavach-chain — router/skill_first.rs:87; backing field never populated.
- **register_agent_skills** (fn) — kavach-chain — router/skill_first.rs:26; never called → field always empty.
- **should_prefer_skill** (fn) — kavach-chain — router/skill_first.rs:101; only `route()` used.
- **get_bool** (fn) — kavach-types — lib.rs:274; `get_string`/`get_int` used, this 0 prod.
- **HookResponse::new_modify_input** (fn) — kavach-types — lib.rs:413; 0 refs.
- **new_permission_allow_with_input** (fn) — kavach-types — lib.rs:573; non-`with_input` variants used.
- **new_pre_tool_use_modify_input** (fn) — kavach-types — lib.rs:481; other `new_pre_tool_use_*` used, this 0.
- **new_setup_context** (fn) — kavach-types — lib.rs:613; sibling context ctors used, this 0.
- **get_skills_for_intent** (fn) — kavach-config — blocklist.rs:203; duplicate of `requires_research` logic, 0 callers.
- **get_skill_names** (fn) — kavach-config — skills.rs:83; `get_skills_by_priority` used, this 0.
- **get_skill_keywords** (fn) — kavach-config — skills.rs:91; re-exported, never called.
- **reload_gates_config** (fn) — kavach-config — gates_loader.rs:79; never called.
- **get_nlu_patterns** (fn) — kavach-config — loaders.rs:4; exported, unused across 6 dependents.
- **is_engineer** (fn) — kavach-config — agents.rs:31; 0 refs across all dependents.
- **RuleIndex::trigger_count** (fn) — kavach-rule-storage — index.rs:55; RuleStore never calls it.
- **RuleIndex::category_count** (fn) — kavach-rule-storage — index.rs:61; same.
- **RuleVersion::file_modified_time** (fn) — kavach-rule-storage — version.rs:49; private `file_modified_iso` is live.
- **RuleVersion::next_version** (fn) — kavach-rule-storage — version.rs:37; test-only; prod uses compute_hash/has_file_changed.
- **with_environment** (fn) — kavach-rule-engine — context.rs:107; builder chain never calls it.
- **to_json** (fn) — kavach-dtree — tree.rs:46; companion `from_json` also unused.
- **from_json** (fn) — kavach-dtree — tree.rs:54; serialization pair never invoked.
- **with_categorical** (fn) — kavach-dtree — feature.rs:83; `with_bool/with_numeric` used 28×, this 0.
- **validate_skill** (fn) — kavach-rule-parser — validation.rs:24; `ValidationError` type also unused.
- **Sections::is_empty** (fn) — kavach-rule-parser — sections.rs:25; the only `is_empty()` call is on String.
- **Trigger** (struct) — kavach-rule-ast — trigger.rs:10; `SkillMetadata.triggers` is `Vec<String>`, not `Vec<Trigger>`.
- **TriggerCategory** (enum) — kavach-rule-ast — trigger.rs:21; only used by the dead `Trigger` struct.
- **Document::get** (fn) — kavach-toon — lib.rs:53; sole dependent iterates `doc.blocks` directly.

---

## B. Test-only — exist solely to be tested, dead in production (9)

Referenced ONLY by their own crate's `#[cfg(test)]`. No production caller; kept (or written) just so a test could assert on it.

- **all_skills_satisfied** (fn) — kavach-session — enforcement.rs:70; prod uses `missing_skills()`.
- **mark_spec_injected** (fn) — kavach-session — intent.rs:29; only its own `test_spec_injected`.
- **reset_enforcement** (fn) — kavach-session — enforcement.rs:129; 1 test ref only.
- **reset_stale_subagents** (fn) — kavach-session — subagent.rs; SessionStart resets fields directly instead.
- **should_continue_loop** (fn) — kavach-session — enforcement.rs:388; 5 test refs, 0 prod.
- **track_teammate_start** (fn) — kavach-session — team_tracking.rs:12; whole module private + test-only.
- **was_spec_injected** (fn) — kavach-session — intent.rs:37; test-only infra.
- **get_int** (fn) — kavach-types — lib.rs:284; `get_string` used 20×, this test-only.
- **is_subagent_event** (fn) — kavach-types — lib.rs:312; prod checks the string literals directly.

---

## C. Re-export-laundered + inherent-method — also genuine orphans (2)

- **SessionTracking** (struct) — kavach-session — subset.rs:27; re-exported in lib.rs:53 but **never imported anywhere**; `.tracking()` never called. The `pub use` is what hides it from `dead_code`.
- **in_loop** (fn) — kavach-rule-engine — context.rs:128; inherent method (NOT a trait method); only called in its own tests. Trivial pass-through to `loop_active`.

---

## Removal plan (NOT yet executed)

Deleting a `pub` API is harder to reverse than a dep bump, so removal is a separate, confirmed step:

1. **One script, not 76 edits** (BULK_VIA_SCRIPT law): author `scripts/prune-orphans.sh` (driven by `rg`/`sd`/`ast-grep`) + a `just prune-orphans` recipe — re-runnable, reviewable, git-tracked.
2. Delete §A + §B + §C symbols + their now-empty `pub use` re-export lines + siblings left dangling (e.g. `SubagentBlast` once `accumulate_subagent_blast` goes).
3. For test-only (§B): delete the method AND the test that only existed to exercise it.
4. Verify: `cargo build --workspace -D warnings` + `cargo nextest run` + `cargo machete` + `kavach deploy`.
5. Future safety net: add `unreachable_pub` to `[workspace.lints]` so internal-only items must be `pub(crate)` — that re-arms `dead_code` for them and prevents new orphan-pub accumulation.
