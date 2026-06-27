//! Tests for the sqlx-migrate RCA gate, split by concern: core gating logic in
//! `gating`, `DATABASE_URL` local-detection bypass cases in `local_db`.

mod gating;
mod local_db;
