//! RCA-gate regression tests, split by family: allow-path + block-path of the
//! `check` orchestrator, prose/decision detectors, and the transcript scanner.
#[path = "pre_write_rca_guard/tests/detect.rs"]
mod detect;
#[path = "pre_write_rca_guard/tests/gate_allow.rs"]
mod gate_allow;
#[path = "pre_write_rca_guard/tests/gate_block.rs"]
mod gate_block;
#[path = "pre_write_rca_guard/tests/scan.rs"]
mod scan;
