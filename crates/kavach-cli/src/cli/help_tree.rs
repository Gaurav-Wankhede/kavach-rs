use clap::CommandFactory;

use crate::cli::Cli;

/// Print every command path (command → subcommand → leaf) with its summary.
#[must_use]
pub(crate) fn render() -> String {
    super::help_stack::on_big_stack(render_inner)
}

fn render_inner() -> String {
    let mut out = String::from("kavach\n");
    let root = Cli::command();
    walk(&root, 1, &mut out);
    out
}

fn walk(cmd: &clap::Command, depth: usize, out: &mut String) {
    let mut subs: Vec<&clap::Command> = cmd.get_subcommands().collect();
    subs.sort_by_key(|c| c.get_name().to_owned());
    for sub in subs {
        if sub.get_name() == "help" {
            continue;
        }
        let pad = "  ".repeat(depth);
        let about = sub
            .get_about()
            .map(|a| format!("  — {a}"))
            .unwrap_or_default();
        out.push_str(&format!("{pad}{}{about}\n", sub.get_name()));
        walk(sub, depth + 1, out);
    }
}

#[cfg(test)]
#[path = "help_tree_test.rs"]
mod tests;
