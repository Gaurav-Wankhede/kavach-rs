// `kavach db citation {add,get,list,link,traverse,refresh}` — official-docs context awareness (C9).
//! kavach:nano-file-exempt — flat 1:1 verb→RPC-call table; one fn per verb is
//! the cohesive unit (each has exactly one dispatch call site, zero reuse gain
//! from splitting).
use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_surreal::CitationMeta;

fn ok(msg: &str) -> i32 {
    match print_or_exit(msg) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

fn err(msg: &str) -> i32 {
    match ewrite_or_exit(&format!("error: {msg}")) {
        Ok(()) => 1,
        Err(io) => into_exit_code(io),
    }
}

pub(super) fn add(project: &str, entry_key: &str, name: &str, slug: &str, url: &str) -> i32 {
    let meta = CitationMeta::new(slug.to_owned(), url.to_owned());
    match rpc_client::citation_add(project, entry_key, name, vec![meta]) {
        Ok(r) => ok(&format!("citation upserted: {entry_key} (id={})", r.id)),
        Err(e) => err(&format!("add: {e}")),
    }
}

pub(super) fn get(project: &str, entry_key: &str) -> i32 {
    match rpc_client::citation_get(project, entry_key) {
        Ok(Some(c)) => ok(&format!(
            "{} [{}] access={} urls={}",
            c.name,
            c.entry_key,
            c.access_count,
            c.metadata.len()
        )),
        Ok(None) => ok(&format!("(no citation {project}/{entry_key})")),
        Err(e) => err(&format!("get: {e}")),
    }
}

pub(super) fn list(project: &str) -> i32 {
    match rpc_client::citation_list(project) {
        Ok(rows) => {
            for c in &rows {
                let code = ok(&format!(
                    "{} [{}] urls={}",
                    c.name,
                    c.entry_key,
                    c.metadata.len()
                ));
                if code != 0 {
                    return code;
                }
            }
            0
        }
        Err(e) => err(&format!("list: {e}")),
    }
}

pub(super) fn link(node: &str, citation: &str) -> i32 {
    match rpc_client::citation_link(node, citation) {
        Ok(_) => ok(&format!("linked: {node} -cite-> {citation}")),
        Err(e) => err(&format!("link: {e}")),
    }
}

pub(super) fn traverse(citation: &str) -> i32 {
    match rpc_client::citation_traverse(citation) {
        Ok(citers) => {
            for c in &citers {
                let code = ok(c);
                if code != 0 {
                    return code;
                }
            }
            0
        }
        Err(e) => err(&format!("traverse: {e}")),
    }
}

pub(super) fn refresh(citation: &str, delta: f64) -> i32 {
    match rpc_client::citation_refresh(citation, delta) {
        Ok(n) => ok(&format!("rewarded {n} cite edge(s) by {delta}")),
        Err(e) => err(&format!("refresh: {e}")),
    }
}
