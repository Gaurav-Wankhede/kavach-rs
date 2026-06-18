// `kavach think <query>` — hybrid keyword+graph retrieval over the kavach memory
// corpus (Brain-OS G2). Emits cited RRF-ranked hits as JSON; when the corpus is
// thin for the query, auto-files a research card so the gap becomes tracked work.
// SOURCE: roadmap.unit.harness.brain-os.g2-think-mode.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

/// Below this many hits the query is treated as a knowledge gap (auto-filed).
const GAP_FLOOR: usize = 3;

pub(super) fn run(project: &str, query: &str, limit: usize) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return emit(&format!(r#"{{"error":"tokio: {e}"}}"#)),
    };
    runtime.block_on(async { run_async(project, query, limit).await })
}

async fn run_async(project_slug: &str, query: &str, limit: usize) -> i32 {
    let db = match kavach_surreal::open_default_resilient().await {
        Ok(d) => d,
        Err(e) => return emit(&format!(r#"{{"error":"db: {e}"}}"#)),
    };
    let hits = match kavach_surreal::search_corpus(&db, query, limit).await {
        Ok(h) => h,
        Err(e) => return emit(&format!(r#"{{"error":"{e}"}}"#)),
    };
    let filed = if hits.len() < GAP_FLOOR {
        file_gap(&db, project_slug, query).await
    } else {
        false
    };
    emit(&payload(query, &hits, filed))
}

/// File the unanswered query as a `research` card so the gap is tracked work.
/// A missing project is non-fatal: we still report the hits, just unfiled.
async fn file_gap(db: &Surreal<Any>, project_slug: &str, query: &str) -> bool {
    let Ok(Some(project)) = kavach_surreal::project_get_by_slug(db, project_slug).await else {
        return false;
    };
    let Some(project_id) = project.id else {
        return false;
    };
    // blake3 hex is 64 ASCII chars; take the first 59 so `research.gap.<hex>`
    // stays <=72. Char-by-char (not byte-slice) is panic-free by construction.
    let digest: String = blake3::hash(query.as_bytes())
        .to_hex()
        .chars()
        .take(59)
        .collect();
    let key = format!("research.gap.{digest}");
    kavach_surreal::upsert_entry_full()
        .db(db)
        .category("research")
        .project_id(&project_id)
        .entry_key(&key)
        .title(&format!("knowledge gap: {query}"))
        .content(&format!(
            "`kavach think` found <{GAP_FLOOR} hits for this query — corpus is thin. \
             Research the topic and persist a decision/research row to close the gap."
        ))
        .event_source("think")
        .qualified_name("")
        .references(&[])
        .build_for_call()
        .await
        .is_ok()
}

fn payload(query: &str, hits: &[kavach_surreal::BrainHit], filed: bool) -> String {
    let items: Vec<String> = hits
        .iter()
        .map(|h| {
            format!(
                r#"{{"id":"{}","score":{:.6}}}"#,
                h.id.replace('"', r#"\""#),
                h.score
            )
        })
        .collect();
    format!(
        r#"{{"query":"{}","hits":[{}],"count":{},"gap_filed":{}}}"#,
        query.replace('"', r#"\""#),
        items.join(","),
        hits.len(),
        filed
    )
}

fn emit(msg: &str) -> i32 {
    match print_or_exit(msg) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}
