//! Single `match DbAction` dispatch table — one arm
//! per CLI verb routing to its `cmd/db/*` handler. Cohesive routing surface;
//! splitting arms across files fragments one match with no reuse gain.
use super::{
    backup, checkpoint, citation, concept, delete, doctor, embed_backfill, event, get, kanban,
    mistakes, mt, next_prompt, parallel_run, query, query_raw, register, search, status_update,
    wipe_project, write,
};
use crate::cli::DbAction;

#[expect(
    clippy::too_many_lines,
    reason = "single match DbAction dispatch table with one arm per CLI verb"
)]
pub(crate) fn run(action: DbAction) -> i32 {
    match action {
        DbAction::Register { slug, path, stack } => register::run(&slug, &path, stack.as_deref()),
        DbAction::Query {
            project,
            category,
            all,
            depth,
            field,
            compact,
        } => query::run(
            &project,
            category.as_deref(),
            all,
            depth.as_deref(),
            field.as_deref(),
            compact,
        ),
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
            exec_prompt,
        } => write::run(&super::rpc_client::WriteRequest {
            project: &project,
            category: &category,
            key: &key,
            title: &title,
            content: content.as_deref(),
            new,
            update_key: update_key.as_deref(),
            priority,
            exec_prompt: exec_prompt.as_deref(),
            depends_on: &depends_on,
        }),
        DbAction::Get {
            project,
            category,
            key,
            full,
            snippet,
            field,
            compact,
        } => get::run(
            &project,
            &category,
            &key,
            full,
            snippet,
            field.as_deref(),
            compact,
        ),
        DbAction::Event {
            event_type,
            payload,
        } => event::run(&event_type, payload.as_deref()),
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
        DbAction::Kanban {
            project,
            limit,
            status,
            active_first,
            key,
            lane,
            include_verified,
            json,
            format,
        } => kanban::run(
            &project,
            limit,
            status.as_deref(),
            active_first,
            key.as_deref(),
            lane.as_deref(),
            include_verified,
            json,
            format.as_deref(),
        ),
        DbAction::KanbanClose { project, key } => kanban::close(&project, &key),
        DbAction::StatusUpdate {
            project,
            category,
            key,
            status,
            verify_cmd,
        } => status_update::run(&project, &category, &key, &status, verify_cmd.as_deref()),
        DbAction::ConceptAdd {
            name,
            display,
            desc,
            tags,
            sources,
        } => concept::add(&name, &display, &desc, tags.as_deref(), sources.as_deref()),
        DbAction::ConceptLink { from, edge, to } => concept::link(&from, &edge, &to),
        DbAction::ConceptSearch { query, limit } => concept::search(&query, limit),
        DbAction::CitationAdd {
            project,
            entry_key,
            name,
            slug,
            url,
        } => citation::add(&project, &entry_key, &name, &slug, &url),
        DbAction::CitationList { project } => citation::list(&project),
        DbAction::GateConfigGet { project, gate_key } => {
            super::gate_config::get(&project, &gate_key)
        }
        DbAction::GateConfigSet {
            project,
            gate_key,
            kind,
            num,
            boolean,
            list,
            text,
        } => super::gate_config::set(&project, &gate_key, &kind, num, boolean, list, text),
        DbAction::QueryRaw { query } => query_raw::run(&query),
        DbAction::NextPrompt { project } => next_prompt::run(&project),
        DbAction::ParallelRun { project, max_parallel, verify_cmd } => parallel_run::run(&project, max_parallel, verify_cmd),
        DbAction::EmbedBackfill { batch, json } => embed_backfill::run(batch, json),
        DbAction::Checkpoint { truncate } => checkpoint::run(truncate),
        DbAction::Mistakes { action } => mistakes::run(action),
        DbAction::Mt(action) => mt_dispatch(action),
        DbAction::Doctor(action) => doctor_dispatch(action),
        DbAction::Backup { keep } => backup::run(keep),
    }
}

fn doctor_dispatch(action: crate::cli::db::DoctorAction) -> i32 {
    use crate::cli::db::DoctorAction;
    match action {
        DoctorAction::RepairDeps { project, apply } => doctor::repair_deps(&project, apply),
        DoctorAction::StripProseDeps { project, apply } => doctor::strip_prose(&project, apply),
    }
}

fn mt_dispatch(action: crate::cli::db::MtAction) -> i32 {
    use crate::cli::db::MtAction;
    match action {
        MtAction::New {
            project,
            key,
            kind,
            title,
            goal,
            modality,
            op,
            when,
            then,
            scope,
            do_not_touch,
            verify_cmd,
            verify_test,
            verify_assert,
            verify_manual,
            source_kind,
            source_ref,
            source_note,
            depends_on,
            conflicts_with,
            supersedes,
            manual_by,
            priority,
            exec_prompt,
            dry_run,
        } => mt::run(&mt::MtArgs {
            project,
            key,
            kind,
            title,
            goal,
            modality,
            op,
            when,
            then,
            scope,
            do_not_touch,
            verify_cmd,
            verify_test,
            verify_assert,
            verify_manual,
            source_kind,
            source_ref,
            source_note,
            depends_on,
            conflicts_with,
            supersedes,
            manual_by,
            priority,
            exec_prompt,
            dry_run,
        }),
        MtAction::Check { project, key } => mt::check(&project, key.as_deref()),
    }
}
