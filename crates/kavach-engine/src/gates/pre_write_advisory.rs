//! Stage 4: Advisory collection — all section-8 non-blocking advisories.
//! RAG, rust/ts/sql P1, platform P1, think-first, simplicity, surgical,
//! tailwind, GNAP, algo-inject, memory awareness.
mod append;
mod collect;
mod counterfactual;
mod guards;
mod memory;

pub(crate) use collect::collect;
