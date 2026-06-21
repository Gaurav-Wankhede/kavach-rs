// Extracts commands after shell builtin source (` source ... && ...`).
// See decision.engine.env-guard-source-extract-arch.
mod builtin;
mod extract;
mod offset;
mod scan;

#[cfg(test)]
mod tests;

pub(crate) use builtin::has_source_builtin;
pub(crate) use extract::extract_post_source_command;
