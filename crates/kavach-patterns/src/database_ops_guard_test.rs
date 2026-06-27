//! Test hub for `database_ops_guard` — declares the per-concern test leaves.
//! Split out of the former inline `mod tests` to honor the ≤100-LOC nano-file law;
//! each leaf is a child module so `crate::database_ops_guard::detect` resolves.

#[path = "database_ops_guard/test_sql.rs"]
mod test_sql;

mod test_nosql;

#[path = "database_ops_guard/test_kv.rs"]
mod test_kv;

#[path = "database_ops_guard/test_cloudflare.rs"]
mod test_cloudflare;

#[path = "database_ops_guard/test_graph_vector.rs"]
mod test_graph_vector;

#[path = "database_ops_guard/test_shared.rs"]
mod test_shared;
