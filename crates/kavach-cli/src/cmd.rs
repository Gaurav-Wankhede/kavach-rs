pub(crate) mod glob;
pub(crate) mod grep;
pub(crate) mod replace;
pub(crate) mod trace;
pub(crate) mod slice;
pub(crate) mod json;
// hub: CLI dispatch hub — dispatch() fn is intentionally here as the top-level router
mod audit;
mod commands;
mod context;
// `pub(crate)` so `cli::db` reaches `db::write::CATEGORY_HELP` (SSoT for the
// --category clap help — rca.kavach-db-write-category-enum-inconsistent).
pub(crate) mod db;

mod gates;
pub(crate) mod harness_loop;
mod heal;

mod install;
pub(crate) mod io_safe;
mod lint;
mod loophole;

mod origin;
mod oversized;


pub(crate) mod pipeline;

mod retrieve;
mod rules;
mod security;
mod servers;
mod session;
mod status;
mod think;


pub(crate) mod verify;


mod working_state;

use crate::cli::Commands;

/// Dispatch CLI command and return exit code (0 = success, 1 = error).
pub(crate) fn dispatch(command: Commands) -> i32 {
    match command {
        
        Commands::Glob {
            pattern,
            path,
            entry_type,
            hidden,
            max_depth,
            max_count,
        } => glob::run(&pattern, &path, entry_type, hidden, max_depth, max_count),

        Commands::Grep {
            pattern,
            path,
            def,
            call,
            type_name,
            ast,
            lang,
            glob,
            ignore_case,
            word_regexp,
            scope,
            json,
            max_count,
        } => grep::run(
            &pattern,
            &path,
            def.as_deref(),
            call.as_deref(),
            type_name.as_deref(),
            ast,
            lang.as_deref(),
            glob.as_deref(),
            ignore_case,
            word_regexp,
            scope,
            json,
            max_count,
        ),

        Commands::Slice { path, range } => slice::run(&path, &range),
        Commands::Replace {
            find,
            replace,
            path,
            fixed_strings,
            regex,
            dry_run,
        } => replace::run(&find, &replace, &path, fixed_strings, regex, dry_run),
        Commands::Trace {
            symbol,
            path,
            depth,
            callers,
            callees,
            json,
        } => trace::run(&symbol, &path, depth, callers, callees, json),
        Commands::Json {
            query,
            path,
            keys,
            raw,
            json,
        } => json::run(&query, path.as_deref(), keys, raw, json),
        Commands::Status => status::run(),
        Commands::Servers { action } => servers::run(&action),
        Commands::Gates {
            gate_name,
            hook,
            verify,
            vendor,
        } => gates::run(&gate_name, hook, verify, vendor.as_deref()),
        Commands::Session { action } => session::run(&action),
        Commands::Db { action } => db::run(action),
        Commands::Install { vendor, dry_run } => install::run(&vendor, dry_run),
        Commands::Heal { action } => match action {
            crate::cli::HealAction::Capture {
                project,
                incident,
                summary,
                log,
                diff_base,
            } => heal::run(&project, &incident, &summary, log.as_deref(), &diff_base),
            crate::cli::HealAction::Sweep { project } => heal::sweep::run(&project),
            crate::cli::HealAction::MergeGate { pr, witness_pass } => {
                heal::merge_gate::run(pr, witness_pass)
            }
            crate::cli::HealAction::Ingest { project } => heal::ingest::run(&project),
        },
        Commands::Loophole { action } => dispatch_loophole(action),
        Commands::Audit {
            path,
            lens,
            deep,
            fix_cards,
        } => audit::run(
            &path.unwrap_or_else(doctor_workspace_root),
            deep,
            audit::lens::Selection::from_flag(&lens),
            fix_cards,
        ),

        Commands::Origin {
            names,
            path,
            query,
            stdin,
            all,
        } => dispatch_origin(&names, path, query, stdin, all),

        Commands::Loop { action } => harness_loop::run(action),
        v @ Commands::Verify { .. } => dispatch_verify(v),


        Commands::Pipeline { action } => pipeline::run(action),
        Commands::Security { action } => security::run(action),

        Commands::Context {
            project,
            limit,
            status,
            key,
            delta,
            full,
        } => context::run(&project, limit, status.as_deref(), key.as_deref(), delta, full),

        Commands::Think {
            project,
            query,
            limit,
            session_weighted,
        } => think::run(&project, &query, limit, session_weighted),

        Commands::Lint { action } => lint::run(action),
        Commands::Retrieve { hash } => retrieve::run(&hash),

        Commands::WorkingState { action } => working_state::run(&action),
        Commands::CommandTree { tree, markdown } => commands::run(tree, markdown),
    }
}

fn dispatch_origin(
    names: &[String],
    path: Option<std::path::PathBuf>,
    query: Option<String>,
    stdin: bool,
    all: bool,
) -> i32 {
    let root = path.unwrap_or_else(doctor_workspace_root);
    if !root.exists() {
        io_safe::ewrite_or_exit(&format!("origin: target path missing: {}", root.display())).ok();
        return 2;
    }
    if stdin {
        let mut json = String::new();
        if std::io::Read::read_to_string(&mut std::io::stdin(), &mut json).is_err() {
            return io_safe::ewrite_or_exit("origin: failed to read stdin").map_or(1, |()| 2);
        }
        return origin::run_query(&json, &root, all);
    }
    if let Some(json) = query {
        return origin::run_query(&json, &root, all);
    }
    if names.is_empty() {
        return io_safe::ewrite_or_exit("origin: pass NAME(s), --query <json>, or --stdin")
            .map_or(1, |()| 2);
    }
    let mut code = 0;
    for name in names {
        code |= origin::run(name, &root, all);
    }
    code
}

/// Dispatch `verify` — extracted to keep `dispatch` readable.
fn dispatch_verify(command: Commands) -> i32 {
    let Commands::Verify {
        project,
        key,
        crate_name,
        external_verified,
        proof,
    } = command
    else {
        return 1;
    };
    verify::run(
        &project,
        &key,
        crate_name.as_deref(),
        external_verified,
        proof.as_deref(),
    )
}

/// Dispatch the `loophole` subcommands (sweep / loop / cron) — extracted from
/// `dispatch` to keep that router readable.
fn dispatch_loophole(action: crate::cli::LoopholeAction) -> i32 {
    use crate::cli::LoopholeAction;
    match action {
        LoopholeAction::Sweep {
            project,
            run_id,
            round,
        } => loophole::run(&project, &run_id, round),
        LoopholeAction::Loop {
            project,
            run_id,
            dry_rounds,
            max_rounds,
        } => loophole::run_loop(&project, &run_id, dry_rounds, max_rounds),
        LoopholeAction::Cron {
            project,
            hour,
            dry_run,
        } => loophole::cron::run(&project, hour, dry_run),
    }
}

/// Resolve the kavach workspace root for `kavach doctor`: walk up from cwd to the
/// nearest ancestor containing a `crates/` dir. Falls back to cwd so the command
/// still runs (and reports the missing-path exit) rather than panicking.
fn doctor_workspace_root() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut probe = cwd.as_path();
    loop {
        if probe.join("crates").is_dir() {
            return probe.to_path_buf();
        }
        match probe.parent() {
            Some(parent) => probe = parent,
            None => return cwd.clone(),
        }
    }
}
