//! Language + platform guard appenders. Each pulls a guard's optional advisory
//! and joins it via `push_opt`, keeping the `collect` orchestrator flat.
use std::fmt::Write as _;

use kavach_types::HookInput;

use super::append::push_opt;
use crate::gates::pre_write_context::WriteContext;

/// Rust + TypeScript + SQL language advisories (test files are exempt).
pub(super) fn append_lang_guards(ctx: &WriteContext<'_>, context: &mut String) {
    if ctx.is_test {
        return;
    }
    if ctx.is_rust {
        push_opt(
            context,
            super::super::pre_write_rust_guard::format_advisory(ctx.file_path, ctx.content),
        );
        push_opt(
            context,
            super::super::pre_write_rust_guard::format_lint_advisory(ctx.file_path, ctx.content),
        );
        // Karpathy Principle 2: Simplicity.
        push_opt(
            context,
            super::super::pre_write_simplicity_guard::advisory(ctx.file_path, ctx.content),
        );
    }
    if ctx.is_frontend {
        push_opt(
            context,
            super::super::pre_write_ts_guard::format_advisory(ctx.file_path, ctx.content),
        );
    }
    push_opt(
        context,
        super::super::pre_write_sql_guard::format_advisory(ctx.file_path, ctx.content),
    );
}

/// Irreversible-action + platform + API-gateway advisories (test files exempt).
pub(super) fn append_platform_guards(ctx: &WriteContext<'_>, context: &mut String) {
    if ctx.is_test {
        return;
    }
    // Irreversible-action guard (SQL DROP/TRUNCATE/DELETE-without-WHERE + critical
    // paths). Write tool cannot prompt mid-stream so we emit an [IRREVERSIBLE]
    // block instead. Bash-side irreversibility lives in destructive_cli_guard.
    let hits = kavach_patterns::irreversible_guard::detect(ctx.file_path, ctx.content);
    if !hits.is_empty() {
        context.push_str("\n[IRREVERSIBLE]\n");
        for h in hits.iter().take(5) {
            writeln!(context, "  L{} — {}: {}", h.line, h.pattern, h.fix).ok();
        }
    }
    for advisory in [
        super::super::pre_write_response_guard::format_advisory(ctx.file_path, ctx.content),
        super::super::pre_write_microservice_guard::format_advisory(ctx.file_path, ctx.content),
        super::super::pre_write_infra_guard::format_advisory(ctx.file_path, ctx.content),
    ] {
        push_opt(context, advisory);
    }
    push_opt(
        context,
        super::super::api_gateway_guard::format_advisory(ctx.file_path, ctx.content),
    );
}

/// Surgical-diff + scope + Tailwind + GNAP craft advisories.
pub(super) fn append_craft_guards(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    files_modified_this_turn: &[String],
    context: &mut String,
) {
    let ns = input.get_string("new_string");
    push_opt(
        context,
        super::super::pre_write_surgical_guard::diff_advisory(ctx.tool_name, ns),
    );
    push_opt(
        context,
        super::super::pre_write_surgical_guard::scope_advisory(files_modified_this_turn),
    );
    if ctx.is_frontend {
        push_opt(
            context,
            super::super::pre_write_tailwind_guard::advisory(ctx.file_path, ctx.content),
        );
    }
    push_opt(
        context,
        super::super::pre_write_gnap_advisory::advisory(ctx.file_path, ctx.content),
    );
}
