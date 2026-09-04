//! `kavach db citation {add,list}` — official-docs context pinned to a
//! (project, entry-key) for research-consumption guard and citation graph.
//! SOURCE: <https://surrealdb.com/docs/surrealql/statements/define/table>
use super::rpc_client;
use kavach_surreal::CitationMeta;

/// Pin an official-docs citation to a (project, entry-key).
pub(super) fn add(
    project: &str,
    entry_key: &str,
    name: &str,
    slug: &str,
    url: &str,
) -> i32 {
    let metadata = vec![CitationMeta::new(slug.to_owned(), url.to_owned())];
    match rpc_client::citation_add(project, entry_key, name, metadata) {
        Ok(_) => {
            println!("ok — citation '{name}' pinned to {project}/{entry_key}");
            0
        }
        Err(e) => emit_err(&format!("citation add: {e}")),
    }
}

/// List every citation for a project (newest-updated first).
pub(super) fn list(project: &str) -> i32 {
    match rpc_client::citation_list(project) {
        Ok(items) if items.is_empty() => {
            println!("(no citations for {project})");
            0
        }
        Ok(items) => {
            for item in items {
                let meta = item.metadata.first();
                let slug = meta.map_or("", |m| m.slug.as_str());
                let url = meta.map_or("", |m| m.url.as_str());
                println!(
                    "{:<20} {:<12} {:<30} {}",
                    item.entry_key, slug, item.name, url
                );
            }
            0
        }
        Err(e) => emit_err(&format!("citation list: {e}")),
    }
}

fn emit_err(msg: &str) -> i32 {
    eprintln!("{msg}");
    1
}