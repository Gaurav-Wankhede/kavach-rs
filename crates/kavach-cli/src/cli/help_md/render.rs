/// Render one clap Arg as a Markdown table row: `| flag | help | default |`.
#[must_use]
pub(super) fn arg_row(a: &clap::Arg) -> String {
    let flag = a
        .get_long()
        .map(|l| format!("--{l}"))
        .unwrap_or_else(|| format!("<{}>", a.get_id()));
    let help = a
        .get_help()
        .map(|h| h.to_string().replace('\n', " ").replace('|', "\\|"))
        .unwrap_or_default();
    let default = a
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(", ");
    format!("| `{flag}` | {help} | {default} |\n")
}

#[cfg(test)]
#[path = "render_test.rs"]
mod tests;
