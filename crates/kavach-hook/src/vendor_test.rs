//! Native-edge proofs: vendor detection (hybrid), input lowering per harness, and
//! native output rendering incl. each vendor's failure policy.

use super::{Vendor, cursor};
use kavach_types::HookResponse;

// --- detection (hybrid: flag > env > sniff > default) ---

#[test]
fn detects_cursor_from_its_signature_fields() {
    let p = r#"{"conversation_id":"c1","workspace_roots":["/r"],"prompt":"x"}"#;
    assert_eq!(Vendor::detect(p), Vendor::Cursor);
}

#[test]
fn detects_codex_from_turn_id() {
    let p = r#"{"session_id":"s1","turn_id":"t1","hook_event_name":"PreToolUse"}"#;
    assert_eq!(Vendor::detect(p), Vendor::Codex);
}

#[test]
fn detects_cursor_from_camelcase_event_when_id_fields_absent() {
    // workspaceOpen omits conversation_id/generation_id/model — the camelCase
    // event name is the ONLY tell. Before the fix this fell through to CC.
    let p = r#"{"hook_event_name":"workspaceOpen","workspace_roots":["/r"]}"#;
    assert_eq!(Vendor::detect(p), Vendor::Cursor);
    // Even with NO workspace_roots, the event vocabulary alone is decisive.
    let bare = r#"{"hook_event_name":"beforeSubmitPrompt","prompt":"hi"}"#;
    assert_eq!(Vendor::detect(bare), Vendor::Cursor);
}

#[test]
fn detects_cursor_from_cursor_version_field() {
    let p = r#"{"cursor_version":"1.2.3","hook_event_name":"PreToolUse"}"#;
    assert_eq!(Vendor::detect(p), Vendor::Cursor);
}

#[test]
fn pascalcase_event_is_not_mistaken_for_cursor() {
    // CC/Codex PascalCase events must NOT trip the Cursor camelCase matcher.
    // Assert the PAYLOAD-SHAPE seam directly: `detect` falls back to env markers
    // (e.g. CURSOR_AGENT set under the Cursor IDE) when the shape is inconclusive,
    // so testing via detect() would flake on the parent process. The contract
    // under test is "this shape carries no vendor signal" => None => CC default.
    let p = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash"}"#;
    assert_eq!(Vendor::detect_from_payload(p), None);
}

#[test]
fn unknown_or_plain_payload_defaults_to_claude_code() {
    // A bare Claude-Code-shaped object and any non-object carry NO vendor signal.
    // Assert the payload-shape seam directly (None) rather than detect(), which
    // would consult env markers a parent harness sets (CURSOR_AGENT under the
    // Cursor IDE) and flake. `None` here is what makes detect() default to CC.
    assert_eq!(
        Vendor::detect_from_payload(r#"{"session_id":"s1","tool_name":"Bash"}"#),
        None
    );
    assert_eq!(
        Vendor::detect_from_payload("not json"),
        None,
        "unparseable => no payload signal => CC default"
    );
}

#[test]
fn an_explicit_flag_overrides_the_payload_sniff() {
    // Payload looks like Cursor, but the flag forces Codex (hybrid: flag wins).
    let cursor_shaped = r#"{"conversation_id":"c1"}"#;
    assert_eq!(Vendor::resolve(Some("codex"), cursor_shaped), Vendor::Codex);
    assert_eq!(Vendor::resolve(Some("cursor"), "{}"), Vendor::Cursor);
}

#[test]
fn an_unknown_flag_falls_through_to_detect() {
    let p = r#"{"conversation_id":"c1"}"#;
    assert_eq!(
        Vendor::resolve(Some("nonsense"), p),
        Vendor::Cursor,
        "bad flag => sniff"
    );
}

// --- Cursor native input lowering ---

#[test]
fn cursor_input_maps_native_names_to_the_pivot() {
    let p = r#"{
        "conversation_id":"conv-9","prompt":"do it","workspace_roots":["/repo","/other"],
        "metadata":{"tool_name":"Bash"},"hook_event_name":"beforeShellExecution"
    }"#;
    let input = cursor::lower(p).expect("cursor lowers");
    assert_eq!(input.session_id, "conv-9", "conversation_id -> session_id");
    assert_eq!(input.prompt, "do it");
    assert_eq!(input.cwd, "/repo", "first workspace_root -> cwd");
    assert_eq!(input.tool_name, "Bash", "metadata.tool_name -> tool_name");
    assert_eq!(
        input.hook_event_name, "PreToolUse",
        "beforeShellExecution -> PreToolUse"
    );
}

#[test]
fn cursor_pretooluse_event_maps_to_canonical_pretooluse() {
    let p = r#"{
        "conversation_id":"c1","workspace_roots":["/repo"],
        "tool_name":"Write","hook_event_name":"preToolUse",
        "tool_input":{"file_path":"src/lib.rs","content":"fn main(){}"}
    }"#;
    let input = cursor::lower(p).expect("cursor lowers");
    assert_eq!(input.hook_event_name, "PreToolUse", "preToolUse -> PreToolUse");
    assert_eq!(input.tool_name, "Write");
    assert_eq!(input.get_string("file_path"), "src/lib.rs");
}

#[test]
fn cursor_shell_command_reaches_canonical_tool_input() {
    // Regression: beforeShellExecution must carry `command` into tool_input so the
    // destructive blocklist can see it (else rm -rf / is silently allowed).
    let p = r#"{
        "conversation_id":"c","metadata":{"tool_name":"Bash"},
        "hook_event_name":"beforeShellExecution","command":"rm -rf /tmp/x"
    }"#;
    let input = cursor::lower(p).expect("cursor lowers");
    assert_eq!(
        input.get_string("command"),
        "rm -rf /tmp/x",
        "cursor command must reach tool_input[command]"
    );
}

#[test]
fn cursor_pre_tool_deny_renders_permission_deny() {
    // Regression: a PreToolUse deny sets only hook_specific_output.permission_decision
    // (not top-level `decision`). Cursor's render must still emit permission:"deny",
    // else the destructive blocklist's deny is silently downgraded to allow.
    let resp = HookResponse::new_pre_tool_use_deny("BLOCKED [test]: nope");
    let out = cursor::render(&resp, "PreToolUse");
    assert!(out.contains(r#""permission":"deny""#), "must deny: {out}");
    // SPEC (cursor.com/docs/hooks): PreToolUse honors NO `continue` field — only
    // {permission, user_message, agent_message}. Emitting `continue` here is the
    // imprecision this fix removes.
    assert!(!out.contains(r#""continue""#), "pre-tool has no continue field: {out}");
    assert!(out.contains("BLOCKED [test]"), "must carry reason: {out}");
}

#[test]
fn cursor_input_tolerates_nulls_and_missing_fields() {
    let p = r#"{"conversation_id":null,"prompt":"hi","workspace_roots":null,"metadata":null}"#;
    let input = cursor::lower(p).expect("nulls must not block");
    assert_eq!(input.prompt, "hi");
    assert_eq!(input.session_id, "");
    assert_eq!(input.cwd, "");
}

#[test]
fn cursor_loop_count_maps_to_stop_hook_active() {
    // SPEC (cursor.com/docs/hooks): the stop hook carries `loop_count` (auto-
    // follow-ups already fired, starts at 0). The stop gate's dispatch tiers key
    // on the canonical `stop_hook_active` (first_pass when false, retry/verify
    // when true). Without this map Cursor was forever first_pass-only and never
    // ran the done->verified promotion path, stalling the loop after a verify.
    // loop_count == 0 -> initial stop -> false.
    let initial = cursor::lower(r#"{"hook_event_name":"stop","loop_count":0}"#)
        .expect("cursor lowers");
    assert!(!initial.stop_hook_active, "loop_count 0 is the initial stop");
    // loop_count > 0 -> already in a follow-up loop -> true (re-entry path).
    let reentry = cursor::lower(r#"{"hook_event_name":"stop","loop_count":3}"#)
        .expect("cursor lowers");
    assert!(reentry.stop_hook_active, "loop_count>0 is a re-entry stop");
    // Absent / malformed loop_count fails safe to 0 -> false.
    let absent = cursor::lower(r#"{"hook_event_name":"stop"}"#).expect("cursor lowers");
    assert!(!absent.stop_hook_active, "absent loop_count defaults to initial");
}

// --- Codex native input lowering (CC-compatible) ---

#[test]
fn codex_input_is_claude_code_compatible_with_extras_ignored() {
    let p = r#"{"session_id":"s1","turn_id":"t1","permission_mode":"plan",
                "tool_name":"Write","hook_event_name":"PreToolUse"}"#;
    let input = super::codex::lower(p).expect("codex lowers");
    assert_eq!(input.session_id, "s1");
    assert_eq!(input.tool_name, "Write", "CC field names pass through");
}

// --- native output rendering + failure policy ---

#[test]
fn cursor_block_renders_the_native_deny_contract() {
    let json = cursor::render(&HookResponse::new_block("nope"), "PreToolUse");
    assert!(!json.contains(r#""continue""#), "pre-tool has no continue: {json}");
    assert!(json.contains(r#""permission":"deny""#), "got {json}");
    assert!(
        json.contains("nope"),
        "reason carried as user/agent message: {json}"
    );
    assert!(
        !json.contains(r#""decision""#),
        "must NOT emit Claude-Code shape"
    );
}

#[test]
fn cursor_approve_renders_allow() {
    let json = cursor::render(&HookResponse::new_approve("ok"), "PreToolUse");
    assert!(!json.contains(r#""continue""#), "pre-tool has no continue: {json}");
    assert!(json.contains(r#""permission":"allow""#), "got {json}");
}

#[test]
fn cursor_fails_open_on_error() {
    let json = cursor::fail_open();
    assert!(
        json.contains(r#""continue":true"#),
        "Cursor's native default is allow"
    );
    assert!(json.contains(r#""permission":"allow""#));
}

#[test]
fn codex_blocks_via_exit_code_two_not_the_body() {
    assert_eq!(
        Vendor::Codex.block_exit_code(),
        2,
        "Codex hard-block = exit 2"
    );
    assert_eq!(Vendor::ClaudeCode.block_exit_code(), 0);
    assert_eq!(Vendor::Cursor.block_exit_code(), 0);
}

#[test]
fn claude_code_render_is_the_canonical_json_unchanged() {
    let json = Vendor::ClaudeCode.render(&HookResponse::new_block("x"));
    assert!(
        json.contains(r#""decision":"block""#),
        "CC keeps canonical shape: {json}"
    );
}

// --- thread-local output sink (the happy-path native translation) ---

#[test]
fn output_sink_defaults_to_claude_code_then_tracks_set_vendor() {
    // The sink is what makes a gate's SELF-EMITTED verdict native: the edge arms
    // it once, every `output(&resp)` then renders in that dialect. Proven here on
    // the selector; the render mapping itself is covered above.
    assert_eq!(
        crate::output_vendor(),
        Vendor::ClaudeCode,
        "unset => canonical default"
    );
    crate::set_output_context(Vendor::Cursor, "Stop");
    assert_eq!(crate::output_vendor(), Vendor::Cursor);
    assert_eq!(
        crate::output_event(),
        "Stop",
        "the answered event is recorded too"
    );
    // Restore so we don't leak the dialect into sibling tests on this thread.
    crate::set_output_context(Vendor::ClaudeCode, "");
}

#[test]
fn cursor_session_start_injects_context_as_additional_context() {
    // SPEC (cursor.com/docs/hooks): `sessionStart` is the ONLY hook whose output
    // reaches the model, via `additional_context`. The mistake ledger / global
    // rules / kanban boot context land here (once per conversation).
    let mut resp = HookResponse::new_approve("");
    // The real session-start gate prepends an [AUTONOMY_CONTRACT] block ahead of
    // the mistake ledger; prove the renderer carries BOTH through the only
    // agent-readable door so the autonomy contract reaches the model on boot.
    resp.system_message =
        "[AUTONOMY_CONTRACT] claim -> implement -> 3-witness -> close\n[MISTAKE_LEDGER] do not X"
            .to_owned();
    let json = cursor::render(&resp, "SessionStart");
    assert!(
        json.contains(r#""additional_context""#),
        "session-start injects via additional_context: {json}"
    );
    assert!(json.contains("MISTAKE_LEDGER"), "context must be carried: {json}");
    assert!(
        json.contains("AUTONOMY_CONTRACT"),
        "the autonomy contract must reach the model via additional_context: {json}"
    );
}

#[test]
fn cursor_submit_emits_continue_only_no_agent_message() {
    // SPEC: beforeSubmitPrompt honors ONLY {continue, user_message}; user_message
    // is user-facing (never reaches the model) and agent_message is NOT honored.
    // A clean allow is a bare {continue:true} — no message popup every turn. This
    // is the bug the old test encoded: routing agent context through submit, which
    // Cursor drops.
    let mut resp = HookResponse::new_approve("");
    resp.system_message = "[MISTAKE_LEDGER] do not X".to_owned();
    let json = cursor::render(&resp, "UserPromptSubmit");
    assert!(json.contains(r#""continue":true"#), "{json}");
    assert!(!json.contains("agent_message"), "submit honors no agent_message: {json}");
    assert!(
        !json.contains("MISTAKE_LEDGER"),
        "allow-path submit must NOT spam a user popup: {json}"
    );
}

#[test]
fn cursor_submit_block_surfaces_reason_in_user_message() {
    let json = cursor::render(&HookResponse::new_block("denied: bad prompt"), "UserPromptSubmit");
    assert!(json.contains(r#""continue":false"#), "{json}");
    assert!(json.contains(r#""user_message""#), "block reason rides user_message: {json}");
    assert!(json.contains("denied: bad prompt"), "{json}");
    assert!(!json.contains(r#""permission""#), "submit has no permission field: {json}");
}

#[test]
fn cursor_after_file_edit_emits_empty_object() {
    // SPEC: afterFileEdit honors NO output fields. Emit `{}`, not a permission blob.
    let json = cursor::render(&HookResponse::new_block("ignored"), "PostToolUse");
    assert_eq!(json, "{}", "afterFileEdit output is an empty object: {json}");
}

#[test]
fn cursor_lifecycle_hooks_emit_empty_object_not_permission_blob() {
    let resp = HookResponse::new_approve("[SUBAGENT_START] id:1");
    for event in ["PreCompact", "SubagentStart", "SubagentStop", "SessionEnd"] {
        let json = cursor::render(&resp, event);
        assert_eq!(json, "{}", "{event} must emit {{}}: {json}");
        assert!(
            !json.contains("permission"),
            "{event} must not emit permission blob: {json}"
        );
    }
}

#[test]
fn cursor_subagent_stop_maps_to_subagent_stop_not_harness_stop() {
    let input = cursor::lower(r#"{"hook_event_name":"subagentStop","conversation_id":"c1"}"#)
        .expect("cursor lowers");
    assert_eq!(
        input.hook_event_name, "SubagentStop",
        "subagentStop must NOT map to Stop (would emit followup_message)"
    );
}

#[test]
fn cursor_stop_block_renders_snake_case_followup_message() {
    // SPEC (cursor.com/docs/hooks): the stop hook output is {followup_message}
    // ONLY — snake_case, and NO `continue` field. A reblock (decision==block, the
    // gate forcing the next dispatch turn) surfaces the reblock text as a non-empty
    // `followup_message`, which Cursor auto-submits as the next user message to
    // continue the loop. The edge passes the answered event ("Stop") so even a bare
    // verdict routes through render_stop.
    let resp = HookResponse::new_stop_block("finish the work");
    let json = cursor::render(&resp, "Stop");
    assert!(
        json.contains("followup_message"),
        "reblock rides snake_case followup_message: {json}"
    );
    assert!(
        !json.contains("followupMessage"),
        "must NOT emit camelCase (Cursor ignores it — the loophole): {json}"
    );
    assert!(json.contains("finish the work"), "{json}");
    assert!(
        !json.contains(r#""continue""#),
        "stop hook has NO continue field per spec: {json}"
    );
    assert!(
        !json.contains(r#""permission""#),
        "stop has no permission field: {json}"
    );
}

#[test]
fn cursor_stop_clean_emits_empty_object_no_followup() {
    // A clean stop (decision != block — drained board / [ALL_BLOCKED]) must NOT
    // carry a follow-up, else Cursor would auto-resubmit and spin. The advisory
    // text rides the gate's own message but is suppressed here so Cursor stops.
    let resp = HookResponse::new_approve("");
    let json = cursor::render(&resp, "Stop");
    assert!(
        !json.contains("followup_message"),
        "clean stop must omit follow-up so Cursor halts: {json}"
    );
}

#[test]
fn cursor_lifecycle_hooks_emit_empty_object() {
    // PreCompact / SubagentStart / SubagentStop / SessionEnd honor NO output
    // fields on Cursor — must emit `{}`, not a spurious pre_tool permission blob.
    for event in ["PreCompact", "SubagentStart", "SubagentStop", "SessionEnd"] {
        let resp = HookResponse::new_approve("relay context queued");
        let json = cursor::render(&resp, event);
        assert_eq!(json, "{}", "{event} must be empty object: {json}");
        assert!(!json.contains("permission"), "{event} has no permission: {json}");
    }
}

#[test]
fn cursor_pre_tool_allow_carries_agent_message() {
    let resp = HookResponse::new_pre_tool_use_allow("[INTENT] type:fix");
    let json = cursor::render(&resp, "PreToolUse");
    assert!(json.contains(r#""permission":"allow""#), "{json}");
    assert!(json.contains("agent_message"), "{json}");
    assert!(json.contains("[INTENT]"), "{json}");
}

#[test]
fn cursor_pre_tool_allow_prefers_additional_context_over_allow_reason() {
    // Turn-shadow relay uses `new_pre_tool_use_with_context("allow", shadow)`;
    // the boilerplate reason must NOT displace rich `additional_context` in
    // `agent_message` — the loophole that made probe 3 emit only "allow".
    let resp = HookResponse::new_pre_tool_use_with_context(
        "allow",
        "[INTENT] type:fix risk:low complexity:simple\n[LOOP] goal:card harness:loop-until-done iter:1 done:3-witness→close→next same turn",
    );
    let json = cursor::render(&resp, "PreToolUse");
    assert!(json.contains("[INTENT]"), "shadow must reach agent_message: {json}");
    assert!(json.contains("[LOOP]"), "LOOP compact must reach agent_message: {json}");
    assert!(
        !json.contains(r#""agent_message":"allow""#),
        "boilerplate allow must not win over relay: {json}"
    );
    assert!(
        !json.contains(r#""user_message""#),
        "allow-path must not mirror relay into user_message: {json}"
    );
}

#[test]
fn cursor_armed_sink_never_emits_a_top_level_null_pair() {
    // The original Cursor wedge: an allow rendered in CC's shape, so Cursor read
    // its absent `continue`/`permission` as null and `invalid type: null` blocked
    // the IDE. With the sink armed, the rendered body carries real booleans.
    let json = Vendor::Cursor.render(&HookResponse::new_approve("ok"));
    assert!(
        !json.contains(r#""continue":null"#),
        "no null continue: {json}"
    );
    assert!(
        !json.contains(r#""permission":null"#),
        "no null permission: {json}"
    );
}

// --- loop-parity: the autonomous-loop AUTO_CONTINUE must reach the agent in
// EVERY vendor's native Stop contract, or the universal loop dies on that tool.
// SOURCE: roadmap universal.loop-parity-audit. Locks the empirical finding that
// a Stop `decision:block` carrying [AUTO_CONTINUE] renders natively per vendor.

/// A canonical Stop-block verdict carrying the harness continuation text, stamped
/// with the Stop event the way the stop gate emits it.
fn stop_continue() -> HookResponse {
    let mut resp = HookResponse::new_block("[AUTO_CONTINUE] Kanban has pending work — do not stop.");
    resp.hook_specific_output = Some(kavach_types::HookSpecificOutput {
        hook_event_name: "Stop".to_owned(),
        ..Default::default()
    });
    resp
}

#[test]
fn auto_continue_reaches_claude_code_stop() {
    // Claude Code: the canonical block body — agent reads `reason`.
    let json = Vendor::ClaudeCode.render_for(&stop_continue(), "Stop");
    assert!(json.contains(r#""decision":"block""#), "cc block: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "cc carries continuation: {json}");
}

#[test]
fn auto_continue_reaches_cursor_stop_as_followup_message() {
    // Cursor's stop contract is {followup_message} (snake_case, no `continue`):
    // continuation is driven SOLELY by a non-empty followup_message.
    let json = Vendor::Cursor.render_for(&stop_continue(), "Stop");
    assert!(json.contains("followup_message"), "cursor stop key: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "cursor carries continuation: {json}");
    assert!(!json.contains(r#""continue""#), "cursor stop has no `continue` field: {json}");
}

#[test]
fn auto_continue_reaches_codex_stop_body() {
    // Codex mirrors CC's body; the Stop block reaches the agent via `reason`.
    let json = Vendor::Codex.render_for(&stop_continue(), "Stop");
    assert!(json.contains(r#""decision":"block""#), "codex block: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "codex carries continuation: {json}");
}

#[test]
fn auto_continue_reaches_pi_agent_end_as_block() {
    // Pi's agent_end (Stop-equivalent) carries the continuation as a `{block:true}`
    // return — the shim re-injects `reason` so the agent keeps working.
    let json = Vendor::Pi.render_for(&stop_continue(), "Stop");
    assert!(json.contains(r#""block":true"#), "pi block: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "pi carries continuation: {json}");
}

#[test]
fn clean_stop_does_not_resubmit_on_cursor() {
    // A non-block verdict (drained board / ALL_BLOCKED) must NOT emit a
    // followup_message — else Cursor would resubmit and spin forever.
    let allow = HookResponse::new_approve("");
    let json = Vendor::Cursor.render_for(&allow, "Stop");
    assert!(!json.contains("followup_message"), "clean stop must not resubmit: {json}");
}

// ── SHARED-STATE PROOF (centralized brain) ────────────────────────────────
// SOURCE: roadmap universal.shared-state-proof. The same kanban/memory/next-card
// is shared across CC/Cursor/Codex/Antigravity (+Pi when shipped) because the
// engine reads DB state ONCE, vendor-blind, then renders per-vendor. Vendor only
// affects input lowering + output rendering — NEVER which card is selected. These
// tests lock that: one canonical next-card verdict reaches EVERY vendor surface
// carrying the identical card identity; only the wire dialect differs.

/// A canonical "next card dispatched" verdict the way the stop gate emits it from
/// shared DB state. The card key is the cross-vendor invariant every surface must
/// carry verbatim.
fn next_card_dispatch(card_key: &str) -> HookResponse {
    let mut resp = HookResponse::new_block(&format!(
        "[AUTO_CONTINUE] NEXT TASK [{card_key}]: shared kanban dispatched this card."
    ));
    resp.hook_specific_output = Some(kavach_types::HookSpecificOutput {
        hook_event_name: "Stop".to_owned(),
        ..Default::default()
    });
    resp
}

#[test]
fn same_next_card_reaches_every_vendor_surface() {
    // The engine's selection is keyed off the shared DB, not the caller's vendor.
    // Whatever card the centralized brain picks must survive rendering into EVERY
    // dialect with its identity intact — proving the brain is shared, not per-tool.
    const CARD: &str = "universal.shared-state-proof";
    let verdict = next_card_dispatch(CARD);
    // Every shipped vendor (Pi excluded until its dialect lands) renders the SAME
    // card key. The wire shape differs per vendor; the card identity does not.
    for vendor in Vendor::all() {
        let json = vendor.render_for(&verdict, "Stop");
        assert!(
            json.contains(CARD),
            "{} dropped the shared card key: {json}",
            vendor.name()
        );
        assert!(
            json.contains("AUTO_CONTINUE"),
            "{} dropped the continuation signal: {json}",
            vendor.name()
        );
    }
}

#[test]
fn drained_board_stops_every_vendor_without_resubmit() {
    // The OTHER half of shared state: when the centralized brain reports the queue
    // drained (a non-block verdict), NO vendor may resubmit. Cursor is the only
    // resubmit-capable surface (followup_message); the rest carry no block body.
    let drained = HookResponse::new_approve("");
    for vendor in Vendor::all() {
        let json = vendor.render_for(&drained, "Stop");
        assert!(
            !json.contains("followup_message"),
            "{} would resubmit on a drained board: {json}",
            vendor.name()
        );
        assert!(
            !json.contains(r#""decision":"block""#),
            "{} emitted a block on a drained board: {json}",
            vendor.name()
        );
    }
}
