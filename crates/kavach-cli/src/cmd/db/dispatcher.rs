//! kavach:micro-file-exempt single `match DbAction` dispatch table — one arm
//! per CLI verb routing to its `cmd/db/*` handler. Cohesive routing surface;
//! splitting arms across files fragments one match with no reuse gain.
use super::{
    archive, backfill_relationships, bridge, concept, delete, event, expire, find, flow,
    gate_config, get,
    graph_query, kanban, lane, list, mistake_hits, pg, populate_graph, priority, query, register,
    register_part, rotate, search, status_update, sync, tree, wipe_project, write,
};
use crate::cli::DbAction;

pub(crate) fn run(action: DbAction) -> i32 {
    match action {
        DbAction::Register { slug, path, stack } => register::run(&slug, &path, stack.as_deref()),
        DbAction::RegisterPart {
            project,
            name,
            path,
            part_type,
        } => register_part::run(&project, &name, &path, &part_type),
        DbAction::Query {
            project,
            category,
            all,
        } => query::run(&project, category.as_deref(), all),
        DbAction::Search {
            project,
            category,
            status,
            since,
            contains,
            limit,
        } => search::run(
            &project,
            category.as_deref(),
            status.as_deref(),
            since.as_deref(),
            contains.as_deref(),
            limit,
        ),
        DbAction::Write {
            project,
            category,
            key,
            title,
            content,
            new,
            update_key,
            priority,
            depends_on,
        } => write::run(&super::rpc_client::WriteRequest {
            project: &project,
            category: &category,
            key: &key,
            title: &title,
            content: content.as_deref(),
            new,
            update_key: update_key.as_deref(),
            priority,
            depends_on: &depends_on,
        }),
        DbAction::PrioritySet {
            project,
            category,
            key,
            priority: new_priority,
            clear,
        } => priority::run(&project, &category, &key, new_priority, clear),
        DbAction::Sync => sync::run(),
        DbAction::FindProject { path } => find::run_project(&path),
        DbAction::FindPart { path } => find::run_part(&path),
        DbAction::ListProjects => list::run_projects(),
        DbAction::SetParent { child, parent } => tree::set_parent(&child, parent.as_deref()),
        DbAction::Tree => tree::render(),
        DbAction::ListParts { project } => list::run_parts(&project),
        DbAction::Expire => expire::run(),
        DbAction::Get {
            project,
            category,
            key,
            full,
        } => get::run(&project, &category, &key, full),
        DbAction::Event {
            event_type,
            payload,
        } => event::run(&event_type, payload.as_deref()),
        action => dispatch_remaining(action),
    }
}

fn dispatch_remaining(action: DbAction) -> i32 {
    match action {
        DbAction::Rotate { days } => rotate::run(days),
        DbAction::Archive {
            floor_days,
            dry_run,
        } => archive::run(floor_days, dry_run),
        DbAction::Kanban {
            project,
            limit,
            status,
            active_first,
            key,
            lane,
            include_verified,
            json,
        } => kanban::run(
            &project,
            limit,
            status.as_deref(),
            active_first,
            key.as_deref(),
            lane.as_deref(),
            include_verified,
            json,
        ),
        DbAction::KanbanClose { project, key } => kanban::close(&project, &key),
        DbAction::StatusUpdate {
            project,
            category,
            key,
            status,
            owner_gated,
        } => status_update::run(&project, &category, &key, &status, owner_gated),
        DbAction::PopulateGraph => populate_graph::run(),
        DbAction::BackfillRelationships { project, dry_run } => {
            backfill_relationships::run(project.as_deref(), dry_run)
        }
        DbAction::GraphQuery {
            entity_type,
            name,
            limit,
        } => graph_query::run(entity_type.as_deref(), name.as_deref(), limit),
        DbAction::PgIntrospect { dsn } => pg::run_introspect(&dsn),
        DbAction::PgIsolation { dsn } => pg::run_isolation(&dsn),
        DbAction::PgEr { dsn } => pg::run_er(&dsn),
        DbAction::PgDrift { dsn } => pg::run_drift(&dsn),
        DbAction::Delete {
            project,
            category,
            key,
            all,
            confirm,
            dry_run,
        } => delete::run(&project, &category, key.as_deref(), all, confirm, dry_run),
        DbAction::WipeProject {
            project,
            confirm,
            dry_run,
        } => wipe_project::run(&project, confirm, dry_run),
        DbAction::ConceptAdd {
            name,
            display,
            desc,
            tags,
            sources,
        } => concept::add(&name, &display, &desc, tags.as_deref(), sources.as_deref()),
        DbAction::ConceptLink { from, edge, to } => concept::link(&from, &edge, &to),
        DbAction::ConceptSearch { query, limit } => concept::search(&query, limit),
        DbAction::ConceptList { limit } => concept::list(limit),
        DbAction::ConceptDelete { name } => concept::delete(&name),
        DbAction::ConceptDeletePrefix { prefix, confirm } => {
            concept::delete_by_prefix(&prefix, confirm)
        }
        gc @ (DbAction::GateConfigGet { .. }
        | DbAction::GateConfigSet { .. }
        | DbAction::GateConfigDelete { .. }
        | DbAction::GateConfigList { .. }) => dispatch_gate_config(gc),
        DbAction::BridgeCreate {
            src_table,
            src_key,
            edge,
            concept,
        } => bridge::create(&src_table, &src_key, &edge, &concept),
        DbAction::BridgeConceptsFor { project } => bridge::concepts_for(&project),
        DbAction::BridgeProjectsFor { concept } => bridge::projects_for(&concept),
        DbAction::MistakeHitCount { name } => mistake_hits::run(&name),
        flow_action @ (DbAction::FlowAdd { .. } | DbAction::FlowShow { .. }) => {
            dispatch_flow(flow_action)
        }
        DbAction::LaneSet {
            project,
            key,
            lane: new_lane,
            clear,
        } => lane::run(&project, &key, new_lane, clear),
        _ => 1, // SAFETY: dispatch_remaining only called with remaining actions
    }
}

/// Implementation-flow DAG verbs (`flow-add` / `flow-show`), split out so
/// `dispatch_remaining` stays within the per-fn line budget.
fn dispatch_flow(action: DbAction) -> i32 {
    match action {
        DbAction::FlowAdd {
            project,
            key,
            title,
            steps_json,
            mermaid,
        } => flow::add(
            &project,
            &key,
            &title,
            steps_json.as_deref(),
            mermaid.as_deref(),
        ),
        DbAction::FlowShow {
            project,
            key,
            format,
        } => flow::show(&project, &key, &format),
        _ => 1, // SAFETY: dispatch_flow only called with Flow* actions
    }
}

/// Route the four `gate-config` verbs. Split out of `dispatch_remaining` to keep
/// that match under the per-fn line cap; only ever called with `GateConfig*`.
fn dispatch_gate_config(action: DbAction) -> i32 {
    match action {
        DbAction::GateConfigGet { project, gate_key } => gate_config::get(&project, &gate_key),
        DbAction::GateConfigSet {
            project,
            gate_key,
            kind,
            num,
            boolean,
            list,
            text,
        } => gate_config::set(&project, &gate_key, &kind, num, boolean, list, text),
        DbAction::GateConfigDelete { project, gate_key } => {
            gate_config::delete(&project, &gate_key)
        }
        DbAction::GateConfigList { project } => gate_config::list(&project),
        _ => 1, // SAFETY: only called with GateConfig* actions
    }
}
