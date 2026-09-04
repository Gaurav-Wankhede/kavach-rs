// Pre-existing WIP (not from the Turso migration): dispatcher arms for several
// RPC wrappers below were removed in an uncommitted working-tree refactor,
// leaving them dead. Allowed so Phase 4 (migrate) compiles alongside that WIP.
#![expect(dead_code, reason = "orphaned RPC wrappers from in-flight CLI refactor")]
mod error;
mod resilience;
mod witness;
mod write_request;
pub(crate) use error::{format_err, should_fallback_to_direct};
pub(crate) use resilience::{open_direct_resilient, or_str};
pub(crate) use witness::mint_receipt;
pub(crate) use write_request::WriteRequest;
pub(crate) use write_request::write;
use kavach_rpc::client::call;
use kavach_rpc::methods::db::{
    DeleteParams, DeleteResult, EventParams, EventResult, GetParams, GetResult,
    KanbanCloseParams, KanbanCloseResult, KanbanParams, KanbanResult, QueryParams,
    QueryResult, RegisterParams, RegisterResult, SearchParams, SearchResult,
    StatusUpdateParams, StatusUpdateResult, WipeProjectParams, WipeProjectResult,
};
#[expect(
    dead_code,
    reason = "RPC API surface — kanban::run staged for RPC fallback wiring (Phase 2 of CLI->RPC migration)"
)]
pub(super) fn kanban(
    project: &str,
    limit: usize,
    status: Option<&str>,
    key: Option<&str>,
) -> Result<KanbanResult, String> {
    let params = KanbanParams::new(
        project.to_owned(),
        limit,
        status.map(String::from),
        key.map(String::from),
    );
    call::<_, KanbanResult>("db.kanban", Some(params)).map_err(format_err)
}
pub(super) fn query(
    project: &str,
    category: Option<&str>,
    all: bool,
) -> Result<QueryResult, String> {
    let params = QueryParams {
        project: project.to_owned(),
        category: category.map(String::from),
        all: Some(all),
    };
    call::<_, QueryResult>("db.query", Some(params)).map_err(format_err)
}
pub(super) fn register(
    slug: &str,
    abs_path: &str,
    stack: Option<&str>,
) -> Result<RegisterResult, String> {
    let params = RegisterParams {
        slug: slug.to_owned(),
        abs_path: abs_path.to_owned(),
        stack: stack.map(String::from),
    };
    call::<_, RegisterResult>("db.register", Some(params)).map_err(format_err)
}
pub(super) fn search(
    project: &str,
    category: Option<&str>,
    status: Option<&str>,
    since: Option<&str>,
    contains: Option<&str>,
    limit: usize,
) -> Result<SearchResult, String> {
    let params = SearchParams {
        project: project.to_owned(),
        category: category.map(String::from),
        status: status.map(String::from),
        since: since.map(String::from),
        contains: contains.map(String::from),
        limit,
    };
    call::<_, SearchResult>("db.search", Some(params)).map_err(format_err)
}
pub(super) fn get(
    project: &str,
    category: &str,
    key: &str,
    full: bool,
) -> Result<GetResult, String> {
    let params = GetParams {
        project: project.to_owned(),
        category: category.to_owned(),
        key: key.to_owned(),
        full: Some(full),
    };
    call::<_, GetResult>("db.get", Some(params)).map_err(format_err)
}
pub(super) fn status_update(
    project: &str,
    category: &str,
    key: &str,
    status: &str,
    receipt: Option<kavach_patterns::witness_receipt::Receipt>,
) -> Result<StatusUpdateResult, String> {
    let params = StatusUpdateParams {
        project: project.to_owned(),
        category: category.to_owned(),
        key: key.to_owned(),
        status: status.to_owned(),
        receipt,
    };
    call::<_, StatusUpdateResult>("db.status_update", Some(params)).map_err(format_err)
}
pub(super) fn kanban_close(
    project: &str,
    key: &str,
    receipt: Option<kavach_patterns::witness_receipt::Receipt>,
) -> Result<KanbanCloseResult, String> {
    let params = KanbanCloseParams {
        project: project.to_owned(),
        key: key.to_owned(),
        receipt,
    };
    call::<_, KanbanCloseResult>("db.kanban_close", Some(params)).map_err(format_err)
}
pub(super) fn delete(
    project: &str,
    category: &str,
    key: Option<&str>,
    all: bool,
    dry_run: bool,
    confirm: Option<String>,
) -> Result<DeleteResult, String> {
    let params = DeleteParams {
        project: project.to_owned(),
        category: category.to_owned(),
        key: key.map(String::from),
        key_prefix: None,
        all: Some(all),
        dry_run: Some(dry_run),
        confirm,
    };
    call::<_, DeleteResult>("db.delete", Some(params)).map_err(format_err)
}
pub(super) fn wipe_project(
    project: &str,
    dry_run: bool,
    confirm: Option<String>,
) -> Result<WipeProjectResult, String> {
    let params = WipeProjectParams {
        project: project.to_owned(),
        dry_run: Some(dry_run),
        confirm,
    };
    call::<_, WipeProjectResult>("db.wipe_project", Some(params)).map_err(format_err)
}
pub(super) fn event(
    event_type: &str,
    payload: Option<&str>,
    work_dir: &str,
) -> Result<EventResult, String> {
    let params = EventParams {
        event_type: event_type.to_owned(),
        payload: payload.map(String::from),
        work_dir: work_dir.to_owned(),
    };
    call::<_, EventResult>("db.event", Some(params)).map_err(format_err)
}
#[expect(
    dead_code,
    reason = "Probe API for callers that need to gate behavior on daemon presence — kept public for downstream consumers"
)]
pub(super) fn is_daemon_available() -> bool {
    call::<(), serde_json::Value>("system.health", None::<()>).is_ok()
}
pub(super) fn concept_add(
    name: &str,
    display: &str,
    desc: &str,
    tags: Vec<String>,
    sources: Vec<String>,
) -> Result<kavach_rpc::methods::concept::IdResult, String> {
    let params = kavach_rpc::methods::concept::AddParams {
        name: name.to_owned(),
        display: display.to_owned(),
        desc: desc.to_owned(),
        tags: Some(tags),
        sources: Some(sources),
    };
    call::<_, kavach_rpc::methods::concept::IdResult>("concept.add", Some(params))
        .map_err(format_err)
}
pub(super) fn concept_link(from: &str, edge: &str, to: &str) -> Result<String, String> {
    let params = kavach_rpc::methods::concept::LinkParams {
        from: from.to_owned(),
        edge: edge.to_owned(),
        to: to.to_owned(),
    };
    call::<_, String>("concept.link", Some(params)).map_err(format_err)
}
pub(super) fn concept_delete(name: &str) -> Result<i64, String> {
    let params = kavach_rpc::methods::concept::DeleteParams {
        name: name.to_owned(),
    };
    call::<_, kavach_rpc::methods::concept::DeleteResult>("concept.delete", Some(params))
        .map(|r| r.removed)
        .map_err(format_err)
}
pub(super) fn concept_delete_by_prefix(prefix: &str, confirm: bool) -> Result<i64, String> {
    let params = kavach_rpc::methods::concept::DeleteByPrefixParams {
        prefix: prefix.to_owned(),
        confirm,
    };
    call::<_, kavach_rpc::methods::concept::DeleteResult>("concept.delete_by_prefix", Some(params))
        .map(|r| r.removed)
        .map_err(format_err)
}
pub(super) fn concept_search(
    query: &str,
    limit: usize,
) -> Result<Vec<kavach_surreal::Entity>, String> {
    let params = kavach_rpc::methods::concept::SearchParams {
        query: query.to_owned(),
        limit: Some(limit),
    };
    call::<_, Vec<kavach_surreal::Entity>>("concept.search", Some(params)).map_err(format_err)
}
pub(super) fn citation_add(
    project: &str,
    entry_key: &str,
    name: &str,
    metadata: Vec<kavach_surreal::CitationMeta>,
) -> Result<kavach_rpc::methods::citation::IdResult, String> {
    let params = kavach_rpc::methods::citation::AddParams {
        project: project.to_owned(),
        entry_key: entry_key.to_owned(),
        name: name.to_owned(),
        metadata,
    };
    call::<_, kavach_rpc::methods::citation::IdResult>("citation.add", Some(params))
        .map_err(format_err)
}
pub(super) fn citation_list(project: &str) -> Result<Vec<kavach_surreal::Citation>, String> {
    let params = kavach_rpc::methods::citation::ListParams {
        project: project.to_owned(),
    };
    call::<_, Vec<kavach_surreal::Citation>>("citation.list", Some(params)).map_err(format_err)
}
pub(super) fn gate_config_get(
    project: &str,
    gate_key: &str,
) -> Result<Option<kavach_rpc::methods::db::GateValueDto>, String> {
    let params = kavach_rpc::methods::db::GateCfgGetParams {
        project: project.to_owned(),
        gate_key: gate_key.to_owned(),
    };
    call::<_, Option<kavach_rpc::methods::db::GateValueDto>>("db.gate_config_get", Some(params))
        .map_err(format_err)
}
pub(super) fn gate_config_set(
    project: &str,
    gate_key: &str,
    value: kavach_rpc::methods::db::GateValueDto,
) -> Result<String, String> {
    let params = kavach_rpc::methods::db::GateCfgSetParams {
        project: project.to_owned(),
        gate_key: gate_key.to_owned(),
        value,
    };
    call::<_, String>("db.gate_config_set", Some(params)).map_err(format_err)
}
pub(super) fn bridge_create(
    src_table: &str,
    src_key: &str,
    edge: &str,
    concept: &str,
) -> Result<kavach_rpc::methods::bridge::IdResult, String> {
    let params = kavach_rpc::methods::bridge::CreateParams::new(
        src_table.to_owned(),
        src_key.to_owned(),
        edge.to_owned(),
        concept.to_owned(),
    );
    call::<_, kavach_rpc::methods::bridge::IdResult>("bridge.create", Some(params))
        .map_err(format_err)
}
pub(super) fn bridge_concepts_for(project: &str) -> Result<Vec<kavach_surreal::BridgeHit>, String> {
    let params = kavach_rpc::methods::bridge::ConceptsForParams::new(project.to_owned());
    call::<_, Vec<kavach_surreal::BridgeHit>>("bridge.concepts_for", Some(params))
        .map_err(format_err)
}
pub(super) fn bridge_projects_for(
    concept: &str,
) -> Result<Vec<kavach_surreal::ProjectHit>, String> {
    let params = kavach_rpc::methods::bridge::ProjectsForParams::new(concept.to_owned());
    call::<_, Vec<kavach_surreal::ProjectHit>>("bridge.projects_for", Some(params))
        .map_err(format_err)
}
pub(super) fn mistake_hit_count(
    name: &str,
) -> Result<kavach_rpc::methods::mistake::HitCountResult, String> {
    let params = kavach_rpc::methods::mistake::HitCountParams::new(name.to_owned());
    call::<_, kavach_rpc::methods::mistake::HitCountResult>("mistake.hit_count", Some(params))
        .map_err(format_err)
}
pub(super) fn mistake_purge(
    gate: &str,
) -> Result<kavach_rpc::methods::mistake::PurgeResult, String> {
    let params = kavach_rpc::methods::mistake::PurgeParams::new(gate.to_owned());
    call::<_, kavach_rpc::methods::mistake::PurgeResult>("mistake.purge", Some(params))
        .map_err(format_err)
}
pub(super) fn ope_evaluate(
    params: kavach_rpc::methods::db::OpeEvaluateParams,
) -> Result<kavach_rpc::methods::db::OpeEvaluateResult, String> {
    call::<_, kavach_rpc::methods::db::OpeEvaluateResult>("db.ope_evaluate", Some(params))
        .map_err(format_err)
}
pub(super) fn ope_audit(
    params: kavach_rpc::methods::db::OpeAuditParams,
) -> Result<kavach_rpc::methods::db::OpeAuditResult, String> {
    call::<_, kavach_rpc::methods::db::OpeAuditResult>("db.ope_audit", Some(params))
        .map_err(format_err)
}
pub(super) fn run_record(params: serde_json::Value) -> Result<serde_json::Value, String> {
    call::<_, serde_json::Value>("run.record", Some(params)).map_err(format_err)
}
pub(super) fn run_update_status(params: serde_json::Value) -> Result<serde_json::Value, String> {
    call::<_, serde_json::Value>("run.update_status", Some(params)).map_err(format_err)
}
pub(super) fn raw_query(query: &str) -> Result<kavach_rpc::methods::db::RawQueryResult, String> {
    let params = kavach_rpc::methods::db::RawQueryParams {
        query: query.to_owned(),
    };
    call::<_, kavach_rpc::methods::db::RawQueryResult>("db.raw_query", Some(params))
        .map_err(format_err)
}
#[cfg(test)]
#[path = "rpc_client_test.rs"]
mod tests;
