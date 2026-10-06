// `kavach think <query>` — hybrid keyword+graph retrieval over the kavach memory
// corpus (Brain-OS G2). Emits cited RRF-ranked hits as JSON; when the corpus is
// thin for the query, auto-files a research card so the gap becomes tracked work.
// SOURCE: roadmap.unit.harness.brain-os.g2-think-mode.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};

/// Below this many hits the query is treated as a knowledge gap (auto-filed).
const GAP_FLOOR: usize = 3;

pub(super) fn run(project: &str, query: &str, limit: usize, session_weighted: bool) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return emit(&format!(r#"{{"error":"tokio: {e}"}}"#)),
    };
    runtime.block_on(async { run_async(project, query, limit, session_weighted).await })
}

fn recall_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").map_or_else(
        |_| {
            std::env::var("USERPROFILE")
                .map_or_else(|_| std::path::PathBuf::from("."), std::path::PathBuf::from)
        },
        std::path::PathBuf::from,
    );
    home.join(".kavach").join("recall_frequency.json")
}

fn working_state_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").map_or_else(
        |_| {
            std::env::var("USERPROFILE")
                .map_or_else(|_| std::path::PathBuf::from("."), std::path::PathBuf::from)
        },
        std::path::PathBuf::from,
    );
    home.join(".kavach").join("working_state.json")
}

fn apply_session_weighting(query: &str, hits: &mut [kavach_db::BrainHit]) {
    let ws_content = std::fs::read_to_string(working_state_file()).unwrap_or_default();
    let ws_lower = ws_content.to_lowercase();

    let r_path = recall_file();
    let mut freq_map: std::collections::HashMap<String, u32> = std::fs::read_to_string(&r_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let q_count = freq_map.entry(query.to_owned()).or_insert(0);
    *q_count = q_count.saturating_add(1);

    for hit in hits.iter_mut() {
        let count = freq_map.entry(hit.id.clone()).or_insert(0);
        *count = count.saturating_add(1);

        #[expect(
            clippy::float_arithmetic,
            reason = "session recency scoring heuristic"
        )]
        {
            let freq_mult = 1.0 + (f64::from(*count) * 0.2);
            hit.score *= freq_mult;

            if !ws_lower.is_empty() && (ws_lower.contains(&hit.id.to_lowercase()) || hit.id.to_lowercase().contains(query)) {
                hit.score *= 10.0;
            }
        }
    }

    if let Some(parent) = r_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("warning: failed to create research cache dir: {e}");
    }
    if let Ok(json) = serde_json::to_string(&freq_map)
        && let Err(e) = std::fs::write(&r_path, json)
    {
        eprintln!("warning: failed to write research cache: {e}");
    }

    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
}

async fn run_async(project_slug: &str, query: &str, limit: usize, session_weighted: bool) -> i32 {
    let db = match kavach_db::open_default_resilient().await {
        Ok(d) => d,
        Err(e) => return emit(&format!(r#"{{"error":"db: {e}"}}"#)),
    };
    let fetch_limit = if session_weighted { limit.max(10).saturating_mul(2) } else { limit };
    let mut hits = match kavach_db::search_corpus_project(&db, Some(project_slug), query, fetch_limit).await {
        Ok(h) => h,
        Err(e) => return emit(&format!(r#"{{"error":"{e}"}}"#)),
    };

    if session_weighted {
        apply_session_weighting(query, &mut hits);
        hits.truncate(limit);
    }

    let filed = if hits.len() < GAP_FLOOR {
        file_gap(&db, project_slug, query).await
    } else {
        false
    };
    emit(&payload(query, &hits, filed))
}

/// File the unanswered query as a `research` card so the gap is tracked work.
/// A missing project is non-fatal: we still report the hits, just unfiled.
async fn file_gap(db: &kavach_db::turso::TursoStore, project_slug: &str, query: &str) -> bool {
    let Ok(Some(project)) = kavach_db::project_get_by_slug(db, project_slug).await else {
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
    kavach_db::upsert_entry_full()
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

fn payload(query: &str, hits: &[kavach_db::BrainHit], filed: bool) -> String {
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
