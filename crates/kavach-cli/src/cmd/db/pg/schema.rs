use postgres::Client;
use postgres_native_tls::MakeTlsConnector;
use thiserror::Error;

/// Typed connection failures — replaces the prior `Box<dyn std::error::Error>`
/// so callers can distinguish TLS errors from Postgres protocol errors. Each
/// `#[from]` variant auto-generates a From impl so `?` propagation still works.
/// Source: <https://docs.rs/thiserror> — #[from] implies #[source].
#[derive(Debug, Error)]
pub(super) enum ConnectError {
    #[error("TLS connector initialization failed: {0}")]
    Tls(#[from] native_tls::Error),

    #[error("Postgres connection failed: {0}")]
    Postgres(#[from] postgres::Error),
}

#[derive(Debug, Clone)]
pub(super) struct Table {
    pub schema: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub(super) struct Column {
    pub schema: String,
    pub table: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub(super) struct ForeignKey {
    pub child_schema: String,
    pub child_table: String,
    pub child_column: String,
    pub parent_schema: String,
    pub parent_table: String,
    pub parent_column: String,
    pub constraint_name: String,
}

/// Connect to Postgres using native-tls so remote providers (Neon, Supabase,
/// managed RDS) with sslmode=require work. Local Postgres over UNIX socket also
/// works because TLS negotiation is optional — server falls back to plain.
pub(super) fn connect(dsn: &str) -> Result<Client, ConnectError> {
    let tls_builder = native_tls::TlsConnector::builder().build()?;
    let connector = MakeTlsConnector::new(tls_builder);
    let client = Client::connect(dsn, connector)?;
    Ok(client)
}

pub(super) fn list_tables(client: &mut Client) -> Result<Vec<Table>, postgres::Error> {
    let rows = client.query(
        "SELECT table_schema, table_name
         FROM information_schema.tables
         WHERE table_schema NOT IN ('pg_catalog','information_schema')
           AND table_type = 'BASE TABLE'
         ORDER BY table_schema, table_name",
        &[],
    )?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(Table {
            schema: row.get(0),
            name: row.get(1),
        });
    }
    Ok(out)
}

pub(super) fn list_columns(client: &mut Client) -> Result<Vec<Column>, postgres::Error> {
    let rows = client.query(
        "SELECT table_schema, table_name, column_name
         FROM information_schema.columns
         WHERE table_schema NOT IN ('pg_catalog','information_schema')
         ORDER BY table_schema, table_name, ordinal_position",
        &[],
    )?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(Column {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
        });
    }
    Ok(out)
}

pub(super) fn list_foreign_keys(client: &mut Client) -> Result<Vec<ForeignKey>, postgres::Error> {
    let rows = client.query(
        "SELECT
            kcu.table_schema, kcu.table_name, kcu.column_name,
            rel_kcu.table_schema, rel_kcu.table_name, rel_kcu.column_name,
            kcu.constraint_name
         FROM information_schema.table_constraints tco
         JOIN information_schema.key_column_usage kcu
            ON tco.constraint_schema = kcu.constraint_schema
           AND tco.constraint_name = kcu.constraint_name
         JOIN information_schema.referential_constraints rco
            ON tco.constraint_schema = rco.constraint_schema
           AND tco.constraint_name = rco.constraint_name
         JOIN information_schema.key_column_usage rel_kcu
            ON rco.unique_constraint_schema = rel_kcu.constraint_schema
           AND rco.unique_constraint_name = rel_kcu.constraint_name
           AND kcu.ordinal_position = rel_kcu.ordinal_position
         WHERE tco.constraint_type = 'FOREIGN KEY'
           AND kcu.table_schema NOT IN ('pg_catalog','information_schema')
         ORDER BY kcu.table_schema, kcu.table_name, kcu.ordinal_position",
        &[],
    )?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(ForeignKey {
            child_schema: row.get(0),
            child_table: row.get(1),
            child_column: row.get(2),
            parent_schema: row.get(3),
            parent_table: row.get(4),
            parent_column: row.get(5),
            constraint_name: row.get(6),
        });
    }
    Ok(out)
}
