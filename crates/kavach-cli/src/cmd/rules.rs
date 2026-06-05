mod check;
mod generate;
mod list;
mod show;

use crate::cli::RulesAction;

pub(super) fn run(action: RulesAction) -> i32 {
    match action {
        RulesAction::List => list::run(),
        RulesAction::Check { path } => check::run(&path),
        RulesAction::Generate { dir } => generate::run(&dir),
        RulesAction::Show { name } => show::run(&name),
    }
}
