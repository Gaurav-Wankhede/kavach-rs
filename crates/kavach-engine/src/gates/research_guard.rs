//! Research ADVISORY gate: a research-class intent without research evidence
//! gets a non-blocking `RESEARCH_ADVISORY` carrying a live exact-timestamp, a
//! context-derived topic, and a distrust-the-weights instruction (config/test
//! files + low-risk intents exempt). Never a hard block — the agent decides.
mod detect;
mod patterns;
mod topic;

pub(crate) use detect::check;
