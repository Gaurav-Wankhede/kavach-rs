//! Status/progress/resume prompts MUST query kavach-db before answering.
//! Injects the current session's project slug to enforce session-project
//! isolation. SOURCE: ssojet.com/blog/tenant-isolation-in-multi-tenant-systems.
use std::fmt::Write as _;

/// Status-shaped prompt keywords that require a `kavach db kanban` lookup.
fn is_status_query(lower: &str) -> bool {
    const KEYS: &[&str] = &[
        "progress",
        "status",
        "what have we done",
        "what did we",
        "what was done",
        "what's done",
        "whats done",
        "what is done",
        "resume",
        "where did we leave",
        "where were we",
        "catch me up",
        "what phases",
        "which phases",
        "roadmap",
        "next step",
        "next phase",
        "next task",
        "what's next",
        "whats next",
        "provide me the next",
        "give me the next",
    ];
    KEYS.iter().any(|k| lower.contains(k))
}

/// Resolve the project slug: pwd-derived leaf name wins over the (possibly stale)
/// session bind, since the user's cwd is the strongest signal of intent.
/// SOURCE: `decision:rca.agent_routing_gate_awareness` (Issue 1 post-incident).
fn resolve_slug(session: &kavach_session::SessionState) -> String {
    let pwd_slug = std::env::current_dir()
        .ok()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_lowercase().replace([' ', '_'], "-"))
        })
        .filter(|s| !s.is_empty());
    match (pwd_slug.as_deref(), session.project.as_str()) {
        (Some(pwd), bound) if !bound.is_empty() && pwd != bound => pwd.to_owned(),
        (Some(pwd), "") => pwd.to_owned(),
        (_, bound) if !bound.is_empty() => bound.to_owned(),
        _ => "<slug>".to_owned(),
    }
}

/// Append DB query requirement for status/progress/resume/next-task prompts.
/// Skips injection when the agent already queried kavach this session
/// (`decision:rca.gate_session_amnesia` — durable artifacts beat re-prompting).
///
/// Thin wrapper: reads the ambient session singleton, then delegates to the pure
/// [`append_db_query_required_with`]. The split keeps the global read OUT of the
/// decision logic so unit tests inject `memory_queried` deterministically instead
/// of depending on whatever on-disk session the test process happens to inherit.
pub(crate) fn append_db_query_required(context: &mut String, prompt: &str) {
    let session = kavach_session::get_or_create_session();
    let slug = resolve_slug(&session);
    append_db_query_required_with(context, prompt, session.memory_queried, &slug);
}

/// Pure core: emit the `[DB_QUERY_REQUIRED]` block iff the prompt is status-shaped
/// AND the session has NOT already queried kavach. No global reads — every input
/// is an explicit parameter, so the behavior is fully determined by its arguments.
pub(crate) fn append_db_query_required_with(
    context: &mut String,
    prompt: &str,
    memory_queried: bool,
    slug: &str,
) {
    if !is_status_query(&prompt.to_lowercase()) {
        return;
    }
    if memory_queried {
        return;
    }
    writeln!(context, "\n[DB_QUERY_REQUIRED]").ok();
    writeln!(
        context,
        "MANDATORY: Run `kavach db kanban --project {slug}` BEFORE answering."
    )
    .ok();
    writeln!(
        context,
        "Chat history is NOT authoritative — it may be truncated, compressed, or wrong."
    )
    .ok();
    writeln!(
        context,
        "The kavach-db SurrealDB store is the single source of truth for project state."
    )
    .ok();
    writeln!(
        context,
        "Steps: 1) kavach db kanban --project {slug}  2) Answer from kanban output."
    )
    .ok();
    writeln!(
        context,
        "Answering from chat history without querying DB is a protocol violation.\n"
    )
    .ok();
    writeln!(context, "[SESSION_PROJECT_ISOLATION]").ok();
    writeln!(context, "This session is bound to project: {slug}").ok();
    writeln!(
        context,
        "NEVER query or execute tasks from other projects in this session."
    )
    .ok();
    writeln!(context, "If user mentions a different project, respond: \"This session is bound to {slug}. Start a new session for other projects.\"").ok();
}
