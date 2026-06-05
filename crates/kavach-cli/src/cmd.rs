// hub: CLI dispatch hub — dispatch() fn is intentionally here as the top-level router
mod app;
mod ask;
pub(crate) mod bg;
pub(crate) mod bulk;
mod context;
pub(crate) mod goal;
// `pub(crate)` so `cli::db` reaches `db::write::CATEGORY_HELP` (SSoT for the
// --category clap help — rca.kavach-db-write-category-enum-inconsistent).
pub(crate) mod db;
pub(crate) mod deploy;
mod gates;
pub(crate) mod harness_loop;
pub(crate) mod io_safe;
pub(crate) mod mcp;
pub(crate) mod mistake;
mod oversized;
mod phase;
pub(crate) mod pipeline;
mod rag;
mod rpc;
mod rules;
mod security;
mod session;
mod spec;
mod status;
mod tailwind_plus;
mod tasks;
pub(crate) mod team;
mod todos;
pub(crate) mod verify;
pub(crate) mod verify_frontend;
pub(crate) mod verify_frontend_detect;

use crate::cli::Commands;

/// Dispatch CLI command and return exit code (0 = success, 1 = error).
pub(crate) fn dispatch(command: Commands) -> i32 {
    match command {
        Commands::Status => status::run(),
        Commands::Gates {
            gate_name,
            hook,
            verify,
            vendor,
        } => gates::run(&gate_name, hook, verify, vendor.as_deref()),
        Commands::Session { action } => session::run(&action),
        Commands::Rules { action } => rules::run(action),
        Commands::Db { action } => db::run(action),
        Commands::Rpc {
            transport,
            apply_schema,
        } => rpc::run(&transport, apply_schema),
        Commands::Rag { action } => rag::run(action),
        Commands::Ask { prompt, max_uses } => ask::run(&prompt, max_uses),
        Commands::Oversized { action } => oversized::run(action),
        Commands::TailwindPlus { action } => tailwind_plus::run(action),
        Commands::Phase { action } => phase::run(action),
        Commands::Loop { action } => harness_loop::run(action),
        Commands::Verify {
            project,
            key,
            crate_name,
        } => verify::run(&project, &key, crate_name.as_deref()),
        Commands::Deploy { skip_tests, bundle } => deploy::run(skip_tests, bundle),
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
        Commands::App => app::run(),
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
        Commands::Mcp => mcp::run(),
    }
}
