//! Shared appenders: every advisory is joined to the context with a leading
//! newline, so collection sites stay a uniform `push_opt(&mut ctx, ...)`.

/// Append `block` to `ctx` preceded by a newline separator.
pub(super) fn push_block(ctx: &mut String, block: &str) {
    ctx.push('\n');
    ctx.push_str(block);
}

/// Append `block` only when present.
pub(super) fn push_opt(ctx: &mut String, block: Option<String>) {
    if let Some(b) = block {
        push_block(ctx, &b);
    }
}
