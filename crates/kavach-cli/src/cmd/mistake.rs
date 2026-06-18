// kavach mistake — inspect / clear / disable the K-PRI mistake ledger.
//
// ARCH: MistakeLedgerCli
// SCOPE: project | CAP: AP | SEARCHED: 2026-05
// TIME: O(N) per list/stats (N = mistake rows; bounded by gate firing rate)
// SPACE: O(N) per call
// YEAR: 2026
//   to avoid an extra RocksDB-LOCK contention path (the daemon owns the lock).
//
// SOURCE: arxiv.org/html/2512.11485 (Mistake Notebook Learning) — ledger admin is part of the loop.
// SOURCE: oneuptime.com/blog/post/2026-02-03-rust-clap-cli-applications — clap-derive subcommand pattern.
// SOURCE: github.com/clap-rs/clap v4.5 — Subcommand enum semantics.

use std::process::Command as ProcessCommand;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
#[command(
    about = "Inspect / clear the K-PRI mistake ledger (auto-populated by stop-gate behavioral blocks)",
    long_about = "The mistake ledger records behavioral blocks from gates (loop escapes, \
unpersisted findings, loophole misses). Rows accumulate hit_count across sessions so \
session-start can inject hot patterns.\n\n\
WHEN: After a stop-gate block, inspect the row; use stats before tuning gate policy.",
    after_help = "EXAMPLES:\n  \
kavach mistake stats --project nicole-carpenter\n  \
kavach mistake list --project P --limit 10 [--gate behavioral_breaker_tripped]\n  \
kavach mistake inspect --project P --key mistake.<gate>.<slug>\n  \
kavach mistake clear --project P --key K --confirm\n  \
kavach mistake clear-all --project P --confirm"
)]
pub(crate) struct MistakeArgs {
    #[command(subcommand)]
    pub action: MistakeAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum MistakeAction {
    /// List mistake ledger rows sorted by `hit_count` (most recurring first).
    List {
        /// Project slug whose mistake rows to list.
        #[arg(long)]
        project: String,
        /// Max rows to print (default 10).
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Filter to one gate name (e.g. `behavioral_breaker_tripped`).
        #[arg(long)]
        gate: Option<String>,
    },
    /// Show full content for one mistake row (gate block detail + metadata).
    Inspect {
        /// Project slug.
        #[arg(long)]
        project: String,
        /// Mistake row key (from `mistake list` or stop-gate advisory).
        #[arg(long)]
        key: String,
    },
    /// Delete one mistake row after human review (--confirm required).
    Clear {
        /// Project slug.
        #[arg(long)]
        project: String,
        /// Mistake row key to delete.
        #[arg(long)]
        key: String,
        /// Required safety flag — refuses without it.
        #[arg(long)]
        confirm: bool,
    },
    /// Wipe all mistake rows for a project (--confirm required).
    ClearAll {
        /// Project slug.
        #[arg(long)]
        project: String,
        /// Required safety flag — refuses without it.
        #[arg(long)]
        confirm: bool,
    },
    /// Aggregate hit counts by gate (dashboard for recurring failures).
    Stats {
        /// Project slug.
        #[arg(long)]
        project: String,
    },
}

pub(crate) fn run(args: MistakeArgs) -> i32 {
    match args.action {
        MistakeAction::List {
            project,
            limit,
            gate,
        } => list(&project, limit, gate.as_deref()),
        MistakeAction::Inspect { project, key } => inspect(&project, &key),
        MistakeAction::Clear {
            project,
            key,
            confirm,
        } => clear(&project, &key, confirm),
        MistakeAction::ClearAll { project, confirm } => clear_all(&project, confirm),
        MistakeAction::Stats { project } => stats(&project),
    }
}

fn list(project: &str, limit: usize, gate_filter: Option<&str>) -> i32 {
    let Some(rows) = collect_mistake_rows(project) else {
        eprintln!("kavach mistake: db query failed (is the daemon running?)");
        return 1;
    };
    let mut rows: Vec<(String, u32)> = rows
        .into_iter()
        .filter(|(key, _)| gate_filter.is_none_or(|g| key.starts_with(&format!("mistake.{g}."))))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows.truncate(limit);
    if rows.is_empty() {
        println!("kavach mistake list: no rows for project={project}");
        return 0;
    }
    println!("[MISTAKE_LEDGER] project={project} top={limit}");
    for (key, hits) in &rows {
        println!("  hits={hits:>4}  key={key}");
    }
    println!("\nrun: kavach mistake inspect --project {project} --key <key>");
    0
}

fn inspect(project: &str, key: &str) -> i32 {
    let out = ProcessCommand::new("kavach")
        .args([
            "db",
            "get",
            "--project",
            project,
            "--category",
            "pattern",
            "--key",
            key,
            "--full",
        ])
        .output();
    let Ok(o) = out else {
        eprintln!("kavach mistake inspect: spawn failed");
        return 1;
    };
    if !o.status.success() {
        eprintln!("kavach mistake inspect: no row for key={key}");
        return 1;
    }
    print!("{}", String::from_utf8_lossy(&o.stdout));
    0
}

fn clear(project: &str, key: &str, confirm: bool) -> i32 {
    let mut args: Vec<&str> = vec![
        "db",
        "delete",
        "--project",
        project,
        "--category",
        "pattern",
        "--key",
        key,
    ];
    if confirm {
        args.push("--confirm");
    } else {
        args.push("--dry-run");
    }
    let out = ProcessCommand::new("kavach").args(&args).output();
    let Ok(o) = out else {
        eprintln!("kavach mistake clear: spawn failed");
        return 1;
    };
    print!("{}", String::from_utf8_lossy(&o.stdout));
    if !confirm {
        println!("\n(dry-run; re-run with --confirm to actually delete)");
    }
    i32::from(!o.status.success())
}

fn clear_all(project: &str, confirm: bool) -> i32 {
    if !confirm {
        eprintln!(
            "kavach mistake clear-all: refusing without --confirm \
                   (would wipe every mistake.* row for project={project})"
        );
        return 2;
    }
    let Some(rows) = collect_mistake_rows(project) else {
        eprintln!("kavach mistake clear-all: db query failed");
        return 1;
    };
    let mut wiped = 0_usize;
    let mut failed = 0_usize;
    for (key, _) in &rows {
        match ProcessCommand::new("kavach")
            .args([
                "db",
                "delete",
                "--project",
                project,
                "--category",
                "pattern",
                "--key",
                key,
                "--confirm",
            ])
            .output()
        {
            Ok(o) if o.status.success() => wiped = wiped.saturating_add(1),
            Ok(_) | Err(_) => failed = failed.saturating_add(1),
        }
    }
    if failed == 0 {
        println!("kavach mistake clear-all: wiped {wiped} row(s) for project={project}");
        0
    } else {
        eprintln!(
            "kavach mistake clear-all: wiped {wiped} row(s), {failed} failed for project={project}"
        );
        1
    }
}

fn stats(project: &str) -> i32 {
    let Some(rows) = collect_mistake_rows(project) else {
        eprintln!("kavach mistake stats: db query failed");
        return 1;
    };
    if rows.is_empty() {
        println!("kavach mistake stats: 0 rows for project={project} (clean ledger)");
        return 0;
    }
    let total_rows = rows.len();
    let total_hits: u64 = rows.iter().map(|(_, h)| u64::from(*h)).sum();
    let mut by_gate: std::collections::BTreeMap<String, (u32, u32)> =
        std::collections::BTreeMap::new();
    for (key, hits) in &rows {
        let gate = key
            .strip_prefix("mistake.")
            .and_then(|s| s.rsplit_once('.').map(|(g, _sig)| g.to_owned()))
            .unwrap_or_else(|| "unknown".to_owned());
        let entry = by_gate.entry(gate).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = entry.1.saturating_add(*hits);
    }
    println!("[MISTAKE_STATS] project={project}");
    println!("  rows: {total_rows}");
    println!("  hits: {total_hits}");
    println!("\n  by gate:");
    for (gate, (row_count, hits)) in &by_gate {
        println!("    {gate:>32}  rows={row_count:>3}  hits={hits:>4}");
    }
    0
}

fn collect_mistake_rows(project: &str) -> Option<Vec<(String, u32)>> {
    let out = ProcessCommand::new("kavach")
        .args(["db", "query", "--project", project, "--category", "pattern"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    let keys: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .find(|t| t.starts_with("mistake."))
                .map(str::to_owned)
        })
        .collect();
    let mut rows: Vec<(String, u32)> = Vec::with_capacity(keys.len());
    for key in keys {
        let hits = fetch_hit_count(project, &key).unwrap_or(1);
        rows.push((key, hits));
    }
    Some(rows)
}

fn fetch_hit_count(project: &str, key: &str) -> Option<u32> {
    let out = ProcessCommand::new("kavach")
        .args([
            "db",
            "get",
            "--project",
            project,
            "--category",
            "pattern",
            "--key",
            key,
            "--full",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8(out.stdout).ok()?;
    body.lines()
        .find_map(|line| line.find("hit_count=").map(|i| (line, i)))
        .and_then(|(line, idx)| {
            let start = idx.saturating_add("hit_count=".len());
            let tail = line.get(start..)?;
            let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u32>().ok()
        })
}