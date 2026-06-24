// kavach:nano-file-exempt — RPC registration hub: a flat linear aggregator of
// register_async_method calls. It cannot decompose into a hub+leaf tree (the
// RpcModule is built in one pass); each verb is one ~6-line stanza.
// split: intentional - RpcModule construction wiring all method namespaces
use crate::error::{internal, invalid_params};
use crate::methods::{
    brain, bridge, bulk, change, citation, concept, db, db_harness, decisions, events,
    gate_patterns, gates, graph, lease, mistake, mistake_top, nlm, nlm_serve, projects, rag,
    roadmap, run, session, system, trust,
};
use crate::state::AppState;
use jsonrpsee::RpcModule;
use jsonrpsee::types::ErrorObjectOwned;

/// Build and register all RPC method handlers.
///
/// Wires all method namespaces (system, algo, arch, projects, gates, etc.) into a single
/// `RpcModule` that is ready for jsonrpsee server binding.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if any method registration fails (malformed method name,
/// handler setup error, or internal state unavailability).
#[expect(
    clippy::too_many_lines,
    reason = "single linear dispatcher, no branching"
)]
pub fn build_module(state: AppState) -> Result<RpcModule<AppState>, ErrorObjectOwned> {
    let mut module = RpcModule::new(state);

    module
        .register_async_method("system.health", |_params, ctx, _ext| async move {
            system::health(&ctx).await
        })
        .map_err(|e| internal(format!("register system.health: {e}")))?;

    

    // change.wait — GUI live-update long-poll: parks until the change feed
    // advances past `since`, then returns the new version. SOURCE: state::ChangeFeed.
    module
        .register_async_method("change.wait", |params, ctx, _ext| async move {
            let p: change::WaitParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            change::wait(&ctx, p).await
        })
        .map_err(|e| internal(format!("register change.wait: {e}")))?;

    module
        .register_async_method("algo.list_recent", |params, ctx, _ext| async move {
            let p: decisions::ListParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            decisions::algo_list(&ctx, p).await
        })
        .map_err(|e| internal(format!("register algo.list_recent: {e}")))?;

    module
        .register_async_method("arch.list_recent", |params, ctx, _ext| async move {
            let p: decisions::ListParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            decisions::arch_list(&ctx, p).await
        })
        .map_err(|e| internal(format!("register arch.list_recent: {e}")))?;

    module
        .register_async_method("arch.upsert", |params, ctx, _ext| async move {
            let p: decisions::ArchUpsertRpcParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            decisions::arch_upsert_rpc(&ctx, p).await
        })
        .map_err(|e| internal(format!("register arch.upsert: {e}")))?;

    module
        .register_async_method("algo.upsert", |params, ctx, _ext| async move {
            let p: decisions::AlgoUpsertRpcParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            decisions::algo_upsert_rpc(&ctx, p).await
        })
        .map_err(|e| internal(format!("register algo.upsert: {e}")))?;

    module
        .register_async_method("projects.find_by_path", |params, ctx, _ext| async move {
            let p: projects::FindByPathParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            projects::find_by_path(&ctx, p).await
        })
        .map_err(|e| internal(format!("register projects.find_by_path: {e}")))?;

    module
        .register_async_method("projects.get_by_slug", |params, ctx, _ext| async move {
            let p: projects::GetBySlugParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            projects::get_by_slug(&ctx, p).await
        })
        .map_err(|e| internal(format!("register projects.get_by_slug: {e}")))?;

    

    module
        .register_async_method("projects.ancestry", |params, ctx, _ext| async move {
            let p: projects::AncestryParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            projects::ancestry(&ctx, p).await
        })
        .map_err(|e| internal(format!("register projects.ancestry: {e}")))?;

    module
        .register_async_method("run.list", |params, ctx, _ext| async move {
            let p: run::ListParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            run::list(&ctx, p).await
        })
        .map_err(|e| internal(format!("register run.list: {e}")))?;

    module
        .register_async_method("run.record", |params, ctx, _ext| async move {
            let p: run::RecordParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            run::record(&ctx, p).await
        })
        .map_err(|e| internal(format!("register run.record: {e}")))?;

    module
        .register_async_method("run.update_status", |params, ctx, _ext| async move {
            let p: run::UpdateStatusParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            run::update_status(&ctx, p).await
        })
        .map_err(|e| internal(format!("register run.update_status: {e}")))?;

    module
        .register_async_method("run.cancel", |params, ctx, _ext| async move {
            let p: run::CancelParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            run::cancel(&ctx, p).await
        })
        .map_err(|e| internal(format!("register run.cancel: {e}")))?;

    module
        .register_async_method("run.spawn", |params, ctx, _ext| async move {
            let p: run::SpawnParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            run::spawn(&ctx, p).await
        })
        .map_err(|e| internal(format!("register run.spawn: {e}")))?;

    module
        .register_async_method(
            "gate_pattern.find_autonomous",
            |params, ctx, _ext| async move {
                let p: gate_patterns::FindParams = params
                    .parse()
                    .map_err(|e| invalid_params(format!("parse params: {e}")))?;
                gate_patterns::find_autonomous(&ctx, p).await
            },
        )
        .map_err(|e| internal(format!("register gate_pattern.find_autonomous: {e}")))?;

    module
        .register_async_method("gate_pattern.upsert", |params, ctx, _ext| async move {
            let p: gate_patterns::UpsertRpcParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            gate_patterns::upsert(&ctx, p).await
        })
        .map_err(|e| internal(format!("register gate_pattern.upsert: {e}")))?;

    module
        .register_async_method("gate_pattern.list_hot", |params, ctx, _ext| async move {
            let p: gate_patterns::ListHotParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            gate_patterns::list_hot(&ctx, p).await
        })
        .map_err(|e| internal(format!("register gate_pattern.list_hot: {e}")))?;

    module
        .register_async_method("event.append", |params, ctx, _ext| async move {
            let p: events::AppendParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            events::append(&ctx, p).await
        })
        .map_err(|e| internal(format!("register event.append: {e}")))?;

    module
        .register_async_method("rag.tree_list_labels", |_params, ctx, _ext| async move {
            rag::tree_list_labels(&ctx).await
        })
        .map_err(|e| internal(format!("register rag.tree_list_labels: {e}")))?;

    module
        .register_async_method(
            "rag.tree_list_refreshable",
            |_params, ctx, _ext| async move { rag::tree_list_refreshable(&ctx).await },
        )
        .map_err(|e| internal(format!("register rag.tree_list_refreshable: {e}")))?;

    module
        .register_async_method("graph.entity_upsert", |params, ctx, _ext| async move {
            let p: graph::EntityUpsertParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            graph::entity_upsert(&ctx, p).await
        })
        .map_err(|e| internal(format!("register graph.entity_upsert: {e}")))?;

    module
        .register_async_method("graph.entity_find", |params, ctx, _ext| async move {
            let p: graph::EntityFindParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            graph::entity_find(&ctx, p).await
        })
        .map_err(|e| internal(format!("register graph.entity_find: {e}")))?;

    module
        .register_async_method("graph.add_relationship", |params, ctx, _ext| async move {
            let p: graph::RelateParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            graph::add_relationship(&ctx, p).await
        })
        .map_err(|e| internal(format!("register graph.add_relationship: {e}")))?;

    module
        .register_async_method("graph.get_related", |params, ctx, _ext| async move {
            let p: graph::GetRelatedParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            graph::get_related(&ctx, p).await
        })
        .map_err(|e| internal(format!("register graph.get_related: {e}")))?;

    module
        .register_async_method("brain.think", |params, ctx, _ext| async move {
            let p: brain::ThinkParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            brain::think(&ctx, p).await
        })
        .map_err(|e| internal(format!("register brain.think: {e}")))?;

    module
        .register_async_method("concept.add", |params, ctx, _ext| async move {
            let p: concept::AddParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            concept::add(&ctx, p).await
        })
        .map_err(|e| internal(format!("register concept.add: {e}")))?;

    module
        .register_async_method("concept.link", |params, ctx, _ext| async move {
            let p: concept::LinkParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            concept::link(&ctx, p).await
        })
        .map_err(|e| internal(format!("register concept.link: {e}")))?;

    

    module
        .register_async_method("concept.search", |params, ctx, _ext| async move {
            let p: concept::SearchParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            concept::search(&ctx, p).await
        })
        .map_err(|e| internal(format!("register concept.search: {e}")))?;

    module
        .register_async_method("concept.list", |params, ctx, _ext| async move {
            let p: concept::ListParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            concept::list(&ctx, p).await
        })
        .map_err(|e| internal(format!("register concept.list: {e}")))?;

    module
        .register_async_method("nlm.store", |params, ctx, _ext| async move {
            let p: nlm::StoreParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            nlm::store(&ctx, p).await
        })
        .map_err(|e| internal(format!("register nlm.store: {e}")))?;

    module
        .register_async_method("nlm.query", |params, ctx, _ext| async move {
            let p: nlm::QueryParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            nlm::query(&ctx, p).await
        })
        .map_err(|e| internal(format!("register nlm.query: {e}")))?;

    module
        .register_async_method("nlm.advise", |params, ctx, _ext| async move {
            let p: nlm_serve::AdviseParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            nlm_serve::advise(&ctx, p).await
        })
        .map_err(|e| internal(format!("register nlm.advise: {e}")))?;

    module
        .register_async_method("concept.delete", |params, ctx, _ext| async move {
            let p: concept::DeleteParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            concept::delete(&ctx, p).await
        })
        .map_err(|e| internal(format!("register/concept-purge-single failed: {e}")))?;

    module
        .register_async_method("concept.delete_by_prefix", |params, ctx, _ext| async move {
            let p: concept::DeleteByPrefixParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            concept::delete_by_prefix(&ctx, p).await
        })
        .map_err(|e| internal(format!("register/concept-purge-prefix failed: {e}")))?;

    module
        .register_async_method("citation.add", |params, ctx, _ext| async move {
            let p: citation::AddParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            citation::add(&ctx, p).await
        })
        .map_err(|e| internal(format!("register citation.add: {e}")))?;

    module
        .register_async_method("citation.get", |params, ctx, _ext| async move {
            let p: citation::GetParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            citation::get(&ctx, p).await
        })
        .map_err(|e| internal(format!("register citation.get: {e}")))?;

    module
        .register_async_method("citation.list", |params, ctx, _ext| async move {
            let p: citation::ListParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            citation::list(&ctx, p).await
        })
        .map_err(|e| internal(format!("register citation.list: {e}")))?;

    module
        .register_async_method("citation.link", |params, ctx, _ext| async move {
            let p: citation::LinkParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            citation::link(&ctx, p).await
        })
        .map_err(|e| internal(format!("register citation.link: {e}")))?;

    module
        .register_async_method("citation.traverse", |params, ctx, _ext| async move {
            let p: citation::TraverseParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            citation::traverse(&ctx, p).await
        })
        .map_err(|e| internal(format!("register citation.traverse: {e}")))?;

    module
        .register_async_method("citation.refresh", |params, ctx, _ext| async move {
            let p: citation::RefreshParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            citation::refresh(&ctx, p).await
        })
        .map_err(|e| internal(format!("register citation.refresh: {e}")))?;

    module
        .register_async_method("citation.for_nodes", |params, ctx, _ext| async move {
            let p: citation::ForNodesParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            citation::for_nodes(&ctx, p).await
        })
        .map_err(|e| internal(format!("register citation.for_nodes: {e}")))?;

    // SPEC: docs/architecture/session-occupancy-lease.md — lease.{acquire,heartbeat,unlock,status}
    module
        .register_async_method("lease.acquire", |params, ctx, _ext| async move {
            let p: lease::AcquireParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            lease::acquire(&ctx, p).await
        })
        .map_err(|e| internal(format!("register lease.acquire: {e}")))?;

    module
        .register_async_method("lease.acquire_set", |params, ctx, _ext| async move {
            let p: lease::AcquireSetParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            lease::acquire_set(&ctx, p).await
        })
        .map_err(|e| internal(format!("register lease.acquire_set: {e}")))?;

    module
        .register_async_method("lease.heartbeat", |params, ctx, _ext| async move {
            let p: lease::HeartbeatParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            lease::heartbeat(&ctx, p).await
        })
        .map_err(|e| internal(format!("register lease.heartbeat: {e}")))?;

    module
        .register_async_method("lease.unlock", |params, ctx, _ext| async move {
            let p: lease::UnlockParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            lease::unlock(&ctx, p).await
        })
        .map_err(|e| internal(format!("register lease.unlock: {e}")))?;

    module
        .register_async_method("lease.status", |params, ctx, _ext| async move {
            let p: lease::StatusParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            lease::status(&ctx, p).await
        })
        .map_err(|e| internal(format!("register lease.status: {e}")))?;

    module
        .register_async_method("bridge.create", |params, ctx, _ext| async move {
            let p: bridge::CreateParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            bridge::create(&ctx, p).await
        })
        .map_err(|e| internal(format!("register bridge.create: {e}")))?;

    module
        .register_async_method("bridge.concepts_for", |params, ctx, _ext| async move {
            let p: bridge::ConceptsForParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            bridge::concepts_for(&ctx, p).await
        })
        .map_err(|e| internal(format!("register bridge.concepts_for: {e}")))?;

    module
        .register_async_method("bridge.projects_for", |params, ctx, _ext| async move {
            let p: bridge::ProjectsForParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            bridge::projects_for(&ctx, p).await
        })
        .map_err(|e| internal(format!("register bridge.projects_for: {e}")))?;

    module
        .register_async_method("mistake.hit_count", |params, ctx, _ext| async move {
            let p: mistake::HitCountParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            mistake::hit_count(&ctx, p).await
        })
        .map_err(|e| internal(format!("register mistake.hit_count: {e}")))?;

    module
        .register_async_method("mistake.record", |params, ctx, _ext| async move {
            let p: mistake::RecordParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            mistake::record(&ctx, p).await
        })
        .map_err(|e| internal(format!("register mistake.record: {e}")))?;

    module
        .register_async_method("mistake.purge", |params, ctx, _ext| async move {
            let p: mistake::PurgeParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            mistake::purge(&ctx, p).await
        })
        .map_err(|e| internal(format!("register mistake.purge: {e}")))?;

    module
        .register_async_method("mistake.top", |params, ctx, _ext| async move {
            let p: mistake_top::TopParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            mistake_top::top(&ctx, p).await
        })
        .map_err(|e| internal(format!("register mistake.top: {e}")))?;

    module
        .register_async_method("roadmap.entry_status", |params, ctx, _ext| async move {
            let p: roadmap::EntryStatusParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::entry_status(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.entry_status: {e}")))?;

    module
        .register_async_method("roadmap.next_open_task", |params, ctx, _ext| async move {
            let p: roadmap::NextOpenTaskParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::next_open_task(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.next_open_task: {e}")))?;


    module
        .register_async_method("roadmap.open_set_census", |params, ctx, _ext| async move {
            let p: roadmap::NextOpenTaskParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::open_set_census(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.open_set_census: {e}")))?;

    module
        .register_async_method("roadmap.next_open_hunt", |params, ctx, _ext| async move {
            let p: roadmap::NextOpenTaskParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::next_open_hunt(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.next_open_hunt: {e}")))?;

    module
        .register_async_method(
            "roadmap.promote_next_backlog",
            |params, ctx, _ext| async move {
                let p: roadmap::NextOpenTaskParams = params
                    .parse()
                    .map_err(|e| invalid_params(format!("parse params: {e}")))?;
                roadmap::promote_next_backlog(&ctx, p).await
            },
        )
        .map_err(|e| internal(format!("register roadmap.promote_next_backlog: {e}")))?;

    module
        .register_async_method("roadmap.claim_card", |params, ctx, _ext| async move {
            let p: roadmap::ClaimCardParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::claim_card(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.claim_card: {e}")))?;

    module
        .register_async_method("roadmap.list_done_cards", |params, ctx, _ext| async move {
            let p: roadmap::NextOpenTaskParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::list_done_cards(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.list_done_cards: {e}")))?;

    module
        .register_async_method(
            "roadmap.list_in_progress_cards",
            |params, ctx, _ext| async move {
                let p: roadmap::NextOpenTaskParams = params
                    .parse()
                    .map_err(|e| invalid_params(format!("parse params: {e}")))?;
                roadmap::list_in_progress_cards(&ctx, p).await
            },
        )
        .map_err(|e| internal(format!("register roadmap.list_in_progress_cards: {e}")))?;

    module
        .register_async_method("roadmap.verify_card", |params, ctx, _ext| async move {
            let p: roadmap::ClaimCardParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::verify_card(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.verify_card: {e}")))?;

    module
        .register_async_method("roadmap.list_titles", |params, ctx, _ext| async move {
            let p: roadmap::ListTitlesParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            roadmap::list_titles(&ctx, p).await
        })
        .map_err(|e| internal(format!("register roadmap.list_titles: {e}")))?;

    module
        .register_async_method("session.get", |params, ctx, _ext| async move {
            let p: session::SessionGetParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            session::get(&ctx, p).await
        })
        .map_err(|e| internal(format!("register session.get: {e}")))?;

    module
        .register_async_method("session.upsert", |params, ctx, _ext| async move {
            let p: session::SessionUpsertParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            session::upsert(&ctx, p).await
        })
        .map_err(|e| internal(format!("register session.upsert: {e}")))?;

    module
        .register_async_method("gates.inspect_bash", |params, ctx, _ext| async move {
            let p: gates::InspectBashParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            gates::inspect_bash(&ctx, p)
        })
        .map_err(|e| internal(format!("register gates.inspect_bash: {e}")))?;

    module
        .register_async_method("gates.scan_write", |params, ctx, _ext| async move {
            let p: gates::ScanWriteParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            gates::scan_write(&ctx, p)
        })
        .map_err(|e| internal(format!("register gates.scan_write: {e}")))?;

    

    

    module
        .register_async_method("trust.classify", |params, ctx, _ext| async move {
            let p: trust::ClassifyParams = params
                .parse()
                .map_err(|e| invalid_params(format!("parse params: {e}")))?;
            trust::classify(&ctx, p).await
        })
        .map_err(|e| internal(format!("register trust.classify: {e}")))?;

    

    // db.* namespace — CLI command equivalents (parameterized queries in kavach_surreal)
    module
        .register_async_method("db.kanban", |params, ctx, _ext| async move {
            let p: db::KanbanParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::kanban(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.kanban_ranked", |params, ctx, _ext| async move {
            let p: db::KanbanRankedParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::kanban_ranked(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.query", |params, ctx, _ext| async move {
            let p: db::QueryParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::query(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.raw_query", |params, ctx, _ext| async move {
            let p: db::RawQueryParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::raw_query(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.list_projects", |params, ctx, _ext| async move {
            let p: db::ListProjectsParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::list_projects(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.list_parts", |params, ctx, _ext| async move {
            let p: db::ListPartsParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::list_parts(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.set_parent", |params, ctx, _ext| async move {
            let p: db::SetParentParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::set_parent(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.register", |params, ctx, _ext| async move {
            let p: db::RegisterParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::register(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.register_part", |params, ctx, _ext| async move {
            let p: db::RegisterPartParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::register_part(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.rotate", |params, ctx, _ext| async move {
            let p: db::RotateParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::rotate(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.expire", |params, ctx, _ext| async move {
            let p: db::ExpireParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::expire(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.find_project", |params, ctx, _ext| async move {
            let p: db::FindParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::find_project(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.find_part", |params, ctx, _ext| async move {
            let p: db::FindParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::find_part(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.tree", |params, ctx, _ext| async move {
            let p: db::TreeParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::tree(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.search", |params, ctx, _ext| async move {
            let p: db::SearchParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::search(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.get", |params, ctx, _ext| async move {
            let p: db::GetParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::get(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.write", |params, ctx, _ext| async move {
            let p: db::WriteParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::write(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.status_update", |params, ctx, _ext| async move {
            let p: db::StatusUpdateParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::status_update(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.set_priority", |params, ctx, _ext| async move {
            let p: db::SetPriorityParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::set_priority(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.set_lane", |params, ctx, _ext| async move {
            let p: db::SetLaneParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::set_lane(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.set_harness", |params, ctx, _ext| async move {
            let p: db_harness::SetHarnessParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db_harness::set_harness(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.get_harness", |params, ctx, _ext| async move {
            let p: db_harness::GetHarnessParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db_harness::get_harness(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.latest_goal_attempt", |params, ctx, _ext| async move {
            let p: db_harness::LatestAttemptParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db_harness::latest_goal_attempt(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.event", |params, ctx, _ext| async move {
            let p: db::EventParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::event(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.bandit_row", |params, ctx, _ext| async move {
            let p: db::BanditRowParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::bandit_row(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.ope_evaluate", |params, ctx, _ext| async move {
            let p: db::OpeEvaluateParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::ope_evaluate(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.ope_audit", |params, ctx, _ext| async move {
            let p: db::OpeAuditParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::ope_audit(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.policy_improve", |params, ctx, _ext| async move {
            let p: db::PolicyImproveParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::policy_improve(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.policy_current", |params, ctx, _ext| async move {
            let p: db::PolicyCurrentParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::policy_current(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method(
            "db.bandit_backfill_session",
            |params, ctx, _ext| async move {
                let p: db::BanditBackfillParams =
                    params.parse().map_err(|e| invalid_params(e.to_string()))?;
                db::bandit_backfill_session(&ctx, p).await
            },
        )
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.graph_query", |params, ctx, _ext| async move {
            let p: db::GraphQueryParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::graph_query(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.graph_fetch", |params, ctx, _ext| async move {
            let p: db::GraphFetchParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::graph_fetch(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.flow_upsert", |params, ctx, _ext| async move {
            let p: db::FlowUpsertParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::flow_upsert(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.flow_render", |params, ctx, _ext| async move {
            let p: db::FlowRenderParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::flow_render(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.flow_list", |params, ctx, _ext| async move {
            let p: db::FlowListParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::flow_list(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.decision_render", |params, ctx, _ext| async move {
            let p: db::DecisionRenderParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::decision_render(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.practice_render", |params, ctx, _ext| async move {
            let p: db::PracticeRenderParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::practice_render(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.stack_render", |params, ctx, _ext| async move {
            let p: db::StackRenderParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::stack_render(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.pattern_render", |params, ctx, _ext| async move {
            let p: db::PatternRenderParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::pattern_render(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.retired_patterns", |params, ctx, _ext| async move {
            let p: db::RetiredPatternsParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::retired_patterns(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.gate_config_get", |params, ctx, _ext| async move {
            let p: db::GateCfgGetParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::gate_config_get(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.gate_config_set", |params, ctx, _ext| async move {
            let p: db::GateCfgSetParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::gate_config_set(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.gate_config_delete", |params, ctx, _ext| async move {
            let p: db::GateCfgDeleteParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::gate_config_delete(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.gate_config_list", |params, ctx, _ext| async move {
            let p: db::GateCfgListParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::gate_config_list(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.kanban_close", |params, ctx, _ext| async move {
            let p: db::KanbanCloseParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::kanban_close(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.delete", |params, ctx, _ext| async move {
            let p: db::DeleteParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::delete(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.wipe_project", |params, ctx, _ext| async move {
            let p: db::WipeProjectParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::wipe_project(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    module
        .register_async_method("db.archive", |params, ctx, _ext| async move {
            let p: db::ArchiveParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            db::archive(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    // bulk-mode RPC verbs — SOURCE: roadmap.unit.kavach-bulk-mode.
    module
        .register_async_method("bulk.sweep_create", |params, ctx, _ext| async move {
            let p: bulk::CreateRpcParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            bulk::create(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;
    module
        .register_async_method("bulk.sweep_apply_event", |params, ctx, _ext| async move {
            let p: bulk::BumpParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            bulk::bump(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;
    module
        .register_async_method("bulk.sweep_close", |params, ctx, _ext| async move {
            let p: bulk::CloseParams = params.parse().map_err(|e| invalid_params(e.to_string()))?;
            bulk::close(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;
    module
        .register_async_method("bulk.sweep_list_active", |params, ctx, _ext| async move {
            let p: bulk::ListActiveParams =
                params.parse().map_err(|e| invalid_params(e.to_string()))?;
            bulk::list_active_rpc(&ctx, p).await
        })
        .map_err(|e| internal(e.to_string()))?;

    Ok(module)
}
