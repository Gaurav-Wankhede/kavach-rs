//! `kavach db concept {add,link,search}` — global L0 concept graph (cross-project).
//! L0 concepts are canon; L1 entities bridge to them via `bridge-create`.
//! SOURCE: <https://surrealdb.com/docs/surrealql/statements/define/table>
use super::rpc_client;

/// Insert or update a global L0 concept.
pub(super) fn add(
    name: &str,
    display: &str,
    desc: &str,
    tags: Option<&str>,
    sources: Option<&str>,
) -> i32 {
    let tags_vec = tags.map(|t| t.split(',').map(str::trim).map(str::to_owned).collect()).unwrap_or_default();
    let sources_vec = sources.map(|s| s.split(',').map(str::trim).map(str::to_owned).collect()).unwrap_or_default();
    match rpc_client::concept_add(name, display, desc, tags_vec, sources_vec) {
        Ok(_) => {
            println!("ok — concept '{name}' upserted");
            0
        }
        Err(e) => emit_err(&format!("concept add: {e}")),
    }
}

/// Relate two L0 concepts via an ontology edge.
pub(super) fn link(from: &str, edge: &str, to: &str) -> i32 {
    match rpc_client::concept_link(from, edge, to) {
        Ok(_) => {
            println!("ok — edge '{edge}' from '{from}' to '{to}'");
            0
        }
        Err(e) => emit_err(&format!("concept link: {e}")),
    }
}

/// Full-text (BM25) search over concept descriptions.
pub(super) fn search(query: &str, limit: usize) -> i32 {
    match rpc_client::concept_search(query, limit) {
        Ok(items) if items.is_empty() => {
            println!("(no concepts match '{query}')");
            0
        }
        Ok(items) => {
            for item in items {
                let desc = item.properties
                    .as_ref()
                    .and_then(|v| v.get("desc"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                println!("{:<24}  {}", item.name, desc);
            }
            0
        }
        Err(e) => emit_err(&format!("concept search: {e}")),
    }
}

fn emit_err(msg: &str) -> i32 {
    eprintln!("{msg}");
    1
}