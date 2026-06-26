// hub: CLI dispatch hub — dispatch() fn is intentionally here as the top-level router
mod ask;
pub(crate) mod bg;
pub(crate) mod bulk;
mod commands;
mod context;
pub(crate) mod goal;
// `pub(crate)` so `cli::db` reaches `db::write::CATEGORY_HELP` (SSoT for the
// --category clap help — rca.kavach-db-write-category-enum-inconsistent).
pub(crate) mod db;
pub(crate) mod deploy;
mod doctor;
mod gates;
pub(crate) mod harness_loop;
mod heal;
mod install;
pub(crate) mod io_safe;
mod lint;
mod loophole;
pub(crate) mod mistake;
mod oversized;
mod phase;
mod phase_registry;
pub(crate) mod pipeline;
mod rules;
mod schema;
mod security;
mod servers;
mod session;
mod spec;
mod status;
mod tailwind_plus;
mod tasks;
pub(crate) mod team;
mod think;
mod todos;
mod toolbelt;
pub(crate) mod verify;
pub(crate) mod verify_frontend;
pub(crate) mod verify_frontend_detect;

use crate::cli::Commands;

/// Dispatch CLI command and return exit code (0 = success, 1 = error).
pub(crate) fn dispatch(command: Commands) -> i32 {
    match command {
        Commands::Status => status::run(),
        Commands::Web { port } => {
            servers::ensure_db_up();
            match kavach_web::serve(port) {
                Ok(()) => 0,
                Err(e) => {
                    if let Err(io_err) = io_safe::ewrite_or_exit(&format!("kavach web: {e}")) {
                        return io_safe::into_exit_code(io_err);
                    }
                    1
                }
            }
        }
        Commands::Servers { action } => servers::run(&action),
        Commands::Gates {
            gate_name,
            hook,
            verify,
            vendor,
        } => gates::run(&gate_name, hook, verify, vendor.as_deref()),
        Commands::Session { action } => session::run(&action),
        Commands::Rules { action } => rules::run(action),
        Commands::Db { action } => db::run(action),
        Commands::Install { vendor, dry_run } => install::run(&vendor, dry_run),
        Commands::Schema { vendor, all } => schema::run(vendor.as_deref(), all),
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
        Commands::Ask { prompt, max_uses } => ask::run(&prompt, max_uses),
        Commands::Oversized { action } => oversized::run(action),
        Commands::TailwindPlus { action } => tailwind_plus::run(action),
        Commands::Doctor => doctor::run(&doctor_workspace_root()),
        Commands::Phase { action } => phase::run(action),
        Commands::Loop { action } => harness_loop::run(action),
        v @ Commands::Verify { .. } => dispatch_verify(v),
        Commands::Deploy { skip_tests } => deploy::run(skip_tests),
        Commands::VerifyFrontend {
            path,
            skip_tests,
            prefer,
        } => {
            let prefer_enum = match prefer.as_str() {
                "biome" => verify_frontend_detect::Prefer::Biome,
                "eslint" => verify_frontend_detect::Prefer::Eslint,
                "auto" => verify_frontend_detect::Prefer::Auto,
                other => {
                    let msg = format!(
                        "kavach: --prefer must be one of: auto | biome | eslint (got {other})"
                    );
                    if let Err(io_err) = io_safe::ewrite_or_exit(&msg) {
                        return io_safe::into_exit_code(io_err);
                    }
                    return 1;
                }
            };
            verify_frontend::run(&path, skip_tests, prefer_enum)
        }
        Commands::Pipeline { action } => pipeline::run(action),
        Commands::Security { action } => security::run(action),
        Commands::Spec { action } => spec::run(action),
        Commands::Tasks { action } => tasks::run(action),
        Commands::Todos { action } => todos::run(action),
        Commands::Context {
            project,
            limit,
            status,
            key,
        } => context::run(&project, limit, status.as_deref(), key.as_deref()),
        Commands::Mistake(args) => mistake::run(args),
        Commands::Bulk(args) => bulk::run(args),
        Commands::Bg(args) => bg::run(args),
        Commands::Goal(args) => goal::run(args),
        Commands::Team(args) => team::run(args),
        Commands::Think {
            project,
            query,
            limit,
        } => think::run(&project, &query, limit),
        Commands::Toolbelt { action } => toolbelt::run(action),
        Commands::Lint { action } => lint::run(action),
        Commands::CommandTree { tree, markdown } => commands::run(tree, markdown),
    }
}

/// Dispatch `verify` — extracted to keep `dispatch` under the nano-file ceiling.
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
/// `dispatch` to keep that router under the 100-line nano-file ceiling.
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
