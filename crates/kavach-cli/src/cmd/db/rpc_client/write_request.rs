use kavach_rpc::methods::db::WriteParams;

#[derive(Clone, Copy)]
pub(crate) struct WriteRequest<'a> {
    pub project: &'a str,
    pub category: &'a str,
    pub key: &'a str,
    pub title: &'a str,
    pub content: Option<&'a str>,
    pub new: bool,
    pub update_key: Option<&'a str>,
    pub priority: Option<i64>,
    pub exec_prompt: Option<&'a str>,
    pub depends_on: &'a [String],
}

pub(crate) fn write(
    req: &WriteRequest<'_>,
    known_keys: &[String],
) -> Result<kavach_rpc::methods::db::WriteResult, String> {
    let effective_key = req.update_key.unwrap_or(req.key);
    let relationships = resolve_relationships(req, effective_key, known_keys);
    let params = WriteParams {
        project: req.project.to_owned(),
        category: req.category.to_owned(),
        key: req.key.to_owned(),
        title: req.title.to_owned(),
        content: req.content.map(String::from),
        new: Some(req.new),
        update_key: req.update_key.map(String::from),
        priority: req.priority,
        exec_prompt: req.exec_prompt.map(String::from),
        relationships,
    };
    kavach_rpc::client::call::<_, kavach_rpc::methods::db::WriteResult>("db.write", Some(params))
        .map_err(super::error::format_err)
}

/// Build the `relationships` payload for `db.write`.
///
/// NLU-prose edges are SPECULATIVE and are resolve-or-dropped against `known_keys`
/// BEFORE they reach the daemon. This gate previously existed only on the
/// direct/fallback path — but that path never persists graph edges at all (its
/// edge write is still a `TODO`), so the gate guarded nothing while the RPC path,
/// the ONLY writer of real edges, carried every prose guess straight through.
/// That is what seeded the phantom `(unresolved)` DAG nodes from ordinary English
/// words (`their`, `dedup`, `every`, `model`, `rename`).
///
/// Comparison is on the BARE tail, so a project-qualified target still resolves.
pub(crate) fn resolve_relationships(
    req: &WriteRequest<'_>,
    _effective_key: &str,
    known_keys: &[String],
) -> Vec<(String, String)> {
    let body = req.content.unwrap_or("");
    let mut rels = kavach_engine::extract_memory_entry_relationships(body);
    for dep in req.depends_on {
        let target = dep.trim();
        if !target.is_empty() {
            rels.push(kavach_engine::ExtractedRelationship::new(
                "depends_on",
                target,
            ));
        }
    }
    fn bare_tail(k: &str) -> &str {
        k.rsplit('/').next().unwrap_or(k)
    }
    rels.into_iter()
        .filter(|r| {
            !r.speculative
                || known_keys
                    .iter()
                    .any(|k| bare_tail(k) == bare_tail(&r.target))
        })
        .map(|r| {
            let tgt = if r.target.contains('/') {
                r.target
            } else {
                format!("{}/{}/{}", req.project, req.category, r.target)
            };
            (r.rel, tgt)
        })
        .collect()
}
