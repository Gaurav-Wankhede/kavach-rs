//! `kavach db migrate` — SurrealDB to embedded Turso migration CLI runner.
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use kavach_db::turso::TursoStore;
use kavach_db::turso::migrate::{
    ImportOpts, NormalizedRecord, SourceCounts, export_from_surreal, import_records, read_ndjson,
    validate_store,
};

/// Run the requested migrate verb. Returns the process exit code.
pub(crate) fn run(verb: &str, from: &str, to: &str, overwrite: bool) -> i32 {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: tokio runtime: {e}");
            return 1;
        }
    };
    match verb {
        "export" => rt.block_on(export_cmd(to)),
        "import" => rt.block_on(import_cmd(from, to, overwrite)),
        "verify" => rt.block_on(verify_cmd(from, to)),
        _ => {
            eprintln!("unknown migrate verb: {verb} (expected export|import|verify)");
            2
        }
    }
}

/// Export from live SurrealDB into NDJSON.
async fn export_cmd(out_path: &str) -> i32 {
    let dest = if out_path.is_empty() {
        "kavach_export.ndjson"
    } else {
        out_path
    };
    println!("connecting to SurrealDB...");
    let db = match kavach_db::open_default_resilient().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot connect to SurrealDB: {e}");
            return 1;
        }
    };
    let file = match tokio::fs::File::create(dest).await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: create output file {dest}: {e}");
            return 1;
        }
    };
    let mut writer = tokio::io::BufWriter::new(file);
    println!("exporting records to {dest}...");
    match export_from_surreal(&db, &mut writer).await {
        Ok(rep) => {
            println!(
                "export complete: {} memories, {} entities, {} edges, {} projects, {} parts",
                rep.memory_count,
                rep.entity_count,
                rep.edge_count,
                rep.project_count,
                rep.part_count
            );
            0
        }
        Err(e) => {
            eprintln!("error during export: {e}");
            1
        }
    }
}

/// Import NDJSON records into TursoStore with validation.
async fn import_cmd(from: &str, to: &str, overwrite: bool) -> i32 {
    if from.is_empty() {
        eprintln!("error: --from <file.ndjson> is required for import");
        return 2;
    }
    let file = match File::open(from) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: open {from}: {e}");
            return 1;
        }
    };
    let records = match read_ndjson(BufReader::new(file)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error reading ndjson: {e}");
            return 1;
        }
    };
    let mut counts = SourceCounts::new();
    let mut seen_edges = std::collections::HashSet::new();
    for rec in &records {
        if let NormalizedRecord::Edge(ed) = rec {
            let src_ep = kavach_db::turso::migrate::export::normalize_endpoint(&ed.from_id);
            let dst_ep = kavach_db::turso::migrate::export::normalize_endpoint(&ed.to_id);
            if seen_edges.insert((src_ep, ed.rel_type.clone(), dst_ep)) {
                counts.edge = counts.edge.saturating_add(1);
            }
        } else {
            rec.count_into(&mut counts);
        }
    }
    println!(
        "source: {} memories, {} entities, {} edges, {} projects",
        counts.memory, counts.entity, counts.edge, counts.project
    );
    let store = match open_store(to).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error opening target store: {e}");
            return 1;
        }
    };
    let opts = ImportOpts::new(500, overwrite, false);
    println!("importing records into Turso...");
    if let Err(e) = import_records(&store, &records, opts).await {
        eprintln!("error during import: {e}");
        return 1;
    }
    println!("validating imported store...");
    match validate_store(&store, &counts).await {
        Ok(v) => {
            if v.is_clean {
                println!(
                    "validation passed: {} memories, {} entities, {} edges",
                    v.memory_count, v.entity_count, v.edge_count
                );
                0
            } else {
                eprintln!(
                    "validation failed: {} dangling edges, fts_matched: {}, mem: {}/{}, ent: {}/{}, edges: {}/{}",
                    v.dangling_edges.len(),
                    v.fts_matched,
                    v.memory_count,
                    counts.memory,
                    v.entity_count,
                    counts.entity,
                    v.edge_count,
                    counts.edge
                );
                1
            }
        }
        Err(e) => {
            eprintln!("error during validation: {e}");
            1
        }
    }
}

/// Verify existing Turso database against source NDJSON.
async fn verify_cmd(from: &str, to: &str) -> i32 {
    if from.is_empty() {
        eprintln!("error: --from <file.ndjson> is required for verify");
        return 2;
    }
    let file = match File::open(from) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: open {from}: {e}");
            return 1;
        }
    };
    let records = match read_ndjson(BufReader::new(file)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error reading ndjson: {e}");
            return 1;
        }
    };
    let mut counts = SourceCounts::new();
    let mut seen_edges = std::collections::HashSet::new();
    for rec in &records {
        if let NormalizedRecord::Edge(ed) = rec {
            let src_ep = kavach_db::turso::migrate::export::normalize_endpoint(&ed.from_id);
            let dst_ep = kavach_db::turso::migrate::export::normalize_endpoint(&ed.to_id);
            if seen_edges.insert((src_ep, ed.rel_type.clone(), dst_ep)) {
                counts.edge = counts.edge.saturating_add(1);
            }
        } else {
            rec.count_into(&mut counts);
        }
    }
    let store = match open_store(to).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error opening target store: {e}");
            return 1;
        }
    };
    match validate_store(&store, &counts).await {
        Ok(v) => {
            if v.is_clean {
                println!(
                    "verification OK: {} memories, {} entities, {} edges match source",
                    v.memory_count, v.entity_count, v.edge_count
                );
                0
            } else {
                eprintln!(
                    "verification failed: {} dangling edges, fts_matched: {}, mem: {}/{}, ent: {}/{}, edges: {}/{}",
                    v.dangling_edges.len(),
                    v.fts_matched,
                    v.memory_count,
                    counts.memory,
                    v.entity_count,
                    counts.entity,
                    v.edge_count,
                    counts.edge
                );
                1
            }
        }
        Err(e) => {
            eprintln!("error during validation: {e}");
            1
        }
    }
}

/// Open target TursoStore, using default location if path is empty.
async fn open_store(to: &str) -> Result<TursoStore, String> {
    if to.is_empty() {
        TursoStore::open_default()
            .await
            .map_err(|e| format!("open default turso db: {e}"))
    } else {
        TursoStore::open(Path::new(to))
            .await
            .map_err(|e| format!("open turso db {to}: {e}"))
    }
}
