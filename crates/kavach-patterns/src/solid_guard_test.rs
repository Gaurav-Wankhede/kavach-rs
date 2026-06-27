//! Test hub for `solid_guard` — dispatches to per-concern test modules.

#[path = "solid_guard/test_srp.rs"]
mod test_srp;

mod test_ocp;

#[path = "solid_guard/test_lsp.rs"]
mod test_lsp;

#[path = "solid_guard/test_isp.rs"]
mod test_isp;

#[path = "solid_guard/test_dip_client.rs"]
mod test_dip_client;

#[path = "solid_guard/test_dip_state.rs"]
mod test_dip_state;

#[path = "solid_guard/test_dip_globals.rs"]
mod test_dip_globals;

#[path = "solid_guard/test_misc.rs"]
mod test_misc;
