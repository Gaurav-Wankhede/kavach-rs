//! `PreCompact` gate — the anti-amnesia seam.
//!
//! Compaction is about to summarize and DISCARD the verbatim history (lossy:
//! bytebell.ai/blog/context-auto-compact-warning/; re-inject-to-survive:
//! arxiv.org/pdf/2602.22402, redis.io/blog/context-rot). This is the LAST hook before
//! discard, so it UNCONDITIONALLY snapshots the durable working set to the DB and
//! re-injects the spine into the post-compact context. See
//! `decision.engine.precompact-anti-amnesia-guard`.
use kavach_types::HookInput;
pub(crate) fn run(input: &HookInput) {
    let mut session = kavach_session::get_or_create_session();
    // DYNAMIC INJECTION: pull the durable spine live from the DB — the active card +
    // its TOUCHES paths (reconcile predicate) — so the post-compact turn reopens WITH
    // the work it was mid-flight on, not a summarized ghost of it. Snapshot it to a
    // decision row first so it survives even the post-compact context itself.
    let memory_guard = build_memory_guard(&session.project);
    let ci = &input.custom_instructions;
    // HARD: never exit_silent when there is durable state to protect — the injection
    // is unconditional whenever a guard block exists, regardless of custom_instructions.
    if memory_guard.is_none() && ci.is_empty() {
        drop(kavach_hook::exit_silent());
        return;
    }
    let mut context = String::new();
    if let Some(guard) = memory_guard {
        context.push_str(&guard);
    }
    if !ci.is_empty() {
        context.push_str(&kavach_hook::context_block(
            "PRE_COMPACT",
            &[
                ("custom_instructions", ci),
                ("date", &kavach_hook::today_full()),
            ],
        ));
    }
    session.queue_lifecycle_relay(&context);
    // CC path: notification context. Cursor drops allow output — relay above.
    drop(kavach_hook::exit_notification_context(&context));
}
/// Build the `[MEMORY_GUARD]` block from live DB state: the `in_progress` card, its
/// TOUCHES paths, and the resume directive. Returns `None` when there is no active
/// card (nothing to protect — fail-soft to today's silent behavior). When a card IS
/// active, the snapshot is also persisted to a decision row so the working set
/// survives in the DB independent of the summarized context.
fn build_memory_guard(project: &str) -> Option<String> {
    if project.is_empty() {
        return None;
    }
    let (key, content) = in_progress_card(project)?;
    let paths = super::session_start::reconcile::touched_paths_from_card(&content);
    let touches = if paths.is_empty() {
        "(none declared)".to_owned()
    } else {
        paths.join(" ")
    };
    // F2: the snapshot write can fail silently; if it did, the in-context guard is
    // the ONLY surviving copy and compaction is about to discard it. Surface the
    // outcome so the agent copies the key NOW rather than trusting a phantom row.
    let persisted_line =
        persisted_line(snapshot_to_decision(project, &key, &touches), project, &key);
    let mut block = format!(
        "[MEMORY_GUARD] (anti-amnesia: compaction is about to discard verbatim history)\n\
         active_card: {key}\n\
         touches: {touches}\n\
         {persisted_line}\n\
         action AFTER compaction: do NOT restart from scratch. Re-read the active card \
         (`kavach db get --project {project} --category roadmap --key {key} --full`), resume at \
         the VERIFY step on the listed TOUCHES paths. Your operating contract is re-injected \
         every turn — obey it, do not re-derive it.\n"
    );
    // F4: SessionStart re-injects the DECISION_MAP spine but PreCompact previously did
    // NOT — the seam relied on the NEXT UserPromptSubmit to carry it, leaving the
    // post-compact turn decision-blind if that assumption broke. Carry the spine across
    // the seam directly, via the SAME emitter SessionStart uses (no drift).
    super::intent::append_mermaid_views(&mut block, project, "");
    Some(block)
}
/// Render the `persisted:` line for the guard from the snapshot-write outcome (F2).
/// `true` → the recall command; `false` → an explicit FAILED warning so the agent
/// copies the working set NOW instead of trusting a row that was never written.
fn persisted_line(ok: bool, project: &str, key: &str) -> String {
    if ok {
        format!(
            "persisted: snapshot written to decision `precompact.snapshot.{key}` — recall with \
             `kavach db get --project {project} --category decision --key precompact.snapshot.{key} --full`."
        )
    } else {
        format!(
            "⚠ persisted: FAILED — the snapshot row could NOT be written (kavach daemon \
             unreachable). This [MEMORY_GUARD] is the ONLY surviving copy and compaction will \
             discard it. COPY active_card `{key}` + touches NOW, before continuing."
        )
    }
}
/// Persist the working-set snapshot to a decision row so it outlives the summarized
/// context. Returns `true` iff the row was written; the caller surfaces a `false`
/// into the guard block so the failure is LLM-visible, not swallowed (F2).
fn snapshot_to_decision(project: &str, card_key: &str, touches: &str) -> bool {
    let body = format!(
        "PRE-COMPACT SNAPSHOT (auto). Active in_progress card at the compaction seam.\n\
         card: {card_key}\ntouches: {touches}\n\
         Resume at VERIFY on these paths after compaction; do not re-edit from scratch."
    );
    let params = serde_json::json!({
        "project": project,
        "category": "decision",
        "key": format!("precompact.snapshot.{card_key}"),
        "title": format!("Pre-compact snapshot: {card_key}"),
        "content": body,
        "update_key": format!("precompact.snapshot.{card_key}"),
    });
    kavach_rpc::client::call::<_, serde_json::Value>("db.write", Some(params)).is_ok()
}
/// The single `in_progress` roadmap card `(key, content)`, or `None` on RPC miss.
fn in_progress_card(project: &str) -> Option<(String, String)> {
    let params = serde_json::json!({ "project": project });
    let v = kavach_rpc::client::call::<_, serde_json::Value>(
        "roadmap.list_in_progress_cards",
        Some(params),
    )
    .ok()?;
    let first = v.as_array()?.iter().next()?;
    let key = first.get("key").and_then(serde_json::Value::as_str)?;
    let content = first
        .get("content")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    Some((key.to_owned(), content.to_owned()))
}
#[cfg(test)]
#[path = "pre_compact_test.rs"]
#[path = "pre_compact_test.rs"]
mod tests;
