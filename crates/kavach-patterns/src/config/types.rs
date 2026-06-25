use std::collections::HashMap;

#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched cross-crate in kavach-engine; non_exhaustive => E0004"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AntiProdLevel {
    P0MockData,
    P1ProdLeak,
    P2ErrorBlind,
    P3TypeLoose,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct AntiProdResult {
    pub level: AntiProdLevel,
    pub code: &'static str,
    pub match_text: String,
    pub message: &'static str,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Config {
    pub sensitive: Vec<String>,
    pub blocked: Vec<String>,
    pub code_exts: Vec<String>,
    pub large_exts: Vec<String>,
    pub valid_agents: HashMap<String, Vec<String>>,
    pub intent_words: HashMap<String, Vec<String>>,
    pub loaded_from: String,
}
