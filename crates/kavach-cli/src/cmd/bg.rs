// kavach bg — bridge to CC 2.1.152+ /bg primitive for background sessions.
// SOURCE: roadmap.unit.kavach-bg-session.
mod start;
mod status;
mod stop;
mod types;

pub(crate) use types::{BgAction, BgArgs};

pub(crate) fn run(args: BgArgs) -> i32 {
    match args.action {
        BgAction::Start {
            project,
            task,
            isolation,
        } => start::run(&project, &task, &isolation),
        BgAction::Status { project } => status::run(&project),
        BgAction::Stop { project, task } => stop::run(&project, &task),
    }
}
