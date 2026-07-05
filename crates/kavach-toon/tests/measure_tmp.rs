// Throwaway measurement harness — deleted after one run. SOURCE: anthropic.com/engineering/effective-context-engineering-for-ai-agents
use kavach_toon::compact::{Level, compress};

#[test]
fn measure_savings() {
    let prose = "A gate that denies or blocks a tool call is a binding instruction, not an error to route around. Read the block text, do exactly what it names this turn, then retry the same corrected call. Never retry verbatim, never disable or skip the hook, never reword the work to dodge a gate, and never declare the work done while a block is standing. You should also remember that the artifact must exist and the diff must be landed and the build must be passing before you claim that the task is complete.";
    let structured = "[SESSION_START]\nmodel: fable\ncontext_window: 200000\nusable_budget: 180000\nproject: kavach-rs\n[TEMPORAL_AWARENESS]\ntoday: Monday, 2026-07-06\nrule: treat THIS as the current date. When researching, search for information that is current as of today; do not assume the training-cutoff date.\n[AUTONOMY_CONTRACT]\nAct, do not narrate: execute the task, show the output, and state the result. You are the orchestrator and you should fan out every read and write task to the cheap executor tier.";
    for (name, s) in [("prose", prose), ("structured", structured)] {
        for lvl in [Level::Lite, Level::Full, Level::Ultra] {
            let out = compress(s, lvl);
            let wi = s.split_whitespace().count();
            let wo = out.split_whitespace().count();
            let pct = (i64::try_from(wo).unwrap_or(0) - i64::try_from(wi).unwrap_or(0)) * 100 / i64::try_from(wi).unwrap_or(1);
            println!("{name} {lvl:?}: tok {wi}->{wo} ({pct}%), bytes {}->{}", s.len(), out.len());
        }
    }
}
