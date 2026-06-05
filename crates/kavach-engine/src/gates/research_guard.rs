//! Research-before-bugfix gate: a bug/fix intent without research evidence gets
//! a `RESEARCH_REQUIRED` advisory (config files + low-risk intents exempt).
mod detect;
mod patterns;

pub(crate) use detect::check;
