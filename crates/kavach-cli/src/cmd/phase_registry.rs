// ARCH: DB-backed SDLC phase registry — kills the frozen VALID_PHASES const.
// PATTERN: directive_cache | SCOPE: cli | CAP: AP | SEARCHED: 2026-06
// Mirrors current_tier_for: reads an ordered `workflow.phase.set` decision row
// (`phases=A,B,C,...`) via kavach_surreal. Fail-soft to BUILTIN_PHASES when the
// row is absent or the DB cannot be opened, so phase commands never hard-fail.

/// Fail-soft default ordering when no `workflow.phase.set` row exists. The DB row
/// (seeded identically) is authoritative; this only covers a missing/unreachable DB.
const BUILTIN_PHASES: [&str; 4] = ["PLAN", "IMPLEMENT", "TEST", "HARDEN"];

/// Project slug whose `workflow.phase.set` row governs the phase ordering.
const PHASE_PROJECT: &str = "kavach-rs";

/// The ordered phase set: DB-backed, fail-soft to `BUILTIN_PHASES`.
#[must_use]
pub(super) fn phases() -> Vec<String> {
    let Ok(runtime) = tokio::runtime::Runtime::new() else {
        return builtin();
    };
    let from_db = runtime.block_on(async { phases_from_db().await });
    from_db.filter(|v| !v.is_empty()).unwrap_or_else(builtin)
}

fn builtin() -> Vec<String> {
    BUILTIN_PHASES.iter().map(|s| (*s).to_owned()).collect()
}

/// `true` when `name` (case-insensitive) is a registered phase.
#[must_use]
pub(super) fn is_valid(name: &str) -> bool {
    let upper = name.to_uppercase();
    phases().iter().any(|p| p == &upper)
}

/// The phase after `current` in registry order, or `None` at the final phase or
/// when `current` is unknown (caller decides how to message each case).
#[must_use]
pub(super) fn next_after(current: &str) -> Option<String> {
    let upper = current.to_uppercase();
    let ordered = phases();
    let idx = ordered.iter().position(|p| p == &upper)?;
    ordered.get(idx.saturating_add(1)).cloned()
}

/// The first phase in registry order — the canonical default for a fresh session.
#[must_use]
pub(super) fn first() -> String {
    phases()
        .into_iter()
        .next()
        .unwrap_or_else(|| BUILTIN_PHASES[0].to_owned())
}

async fn phases_from_db() -> Option<Vec<String>> {
    let db = kavach_surreal::open_default_resilient().await.ok()?;
    let project_rec = kavach_surreal::projects::get_by_slug(&db, PHASE_PROJECT)
        .await
        .ok()??;
    let project_id = project_rec.id?;
    let rows = kavach_surreal::read::list_by_project(&db, "decision", &project_id)
        .await
        .ok()?;
    let row = rows
        .into_iter()
        .find(|r| r.entry_key == "workflow.phase.set")?;
    Some(parse_phase_line(&row.content))
}

/// Parse `phases=PLAN,IMPLEMENT,...` from the row body. Tolerates surrounding
/// prose: scans each line for the `phases=` prefix and splits the value on commas.
fn parse_phase_line(content: &str) -> Vec<String> {
    for line in content.lines() {
        let Some(rest) = line.trim_start().strip_prefix("phases=") else {
            continue;
        };
        let parsed: Vec<String> = rest
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_ordered_phases() {
        let got = parse_phase_line("phases=PLAN,IMPLEMENT,TEST,HARDEN");
        assert_eq!(got, vec!["PLAN", "IMPLEMENT", "TEST", "HARDEN"]);
    }

    #[test]
    fn parse_uppercases_and_trims() {
        let got = parse_phase_line("note\nphases= plan , implement \n");
        assert_eq!(got, vec!["PLAN", "IMPLEMENT"]);
    }

    #[test]
    fn parse_returns_empty_when_no_marker() {
        assert!(parse_phase_line("no marker here").is_empty());
    }

    #[test]
    fn builtin_is_the_canonical_four() {
        assert_eq!(builtin(), vec!["PLAN", "IMPLEMENT", "TEST", "HARDEN"]);
    }
}
