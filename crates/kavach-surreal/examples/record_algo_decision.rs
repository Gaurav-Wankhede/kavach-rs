//! One-off: record the algo_decision row for the CLI dep-resolve-or-drop helper.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = kavach_surreal::open_default().await?;
    let project = kavach_surreal::project_get_by_slug(&db, "kavach-rs")
        .await?
        .ok_or("project not found")?;
    let project_id = project.id.ok_or("project has no id")?;
    let p = kavach_surreal::AlgoUpsertParams {
        project: project_id,
        problem_class: "bare-key-tail resolve-or-drop (n<20 known keys)",
        chosen: "linear any-scan over known_keys, locally-duplicated bare_tail",
        time_complexity: "O(n)",
        space_complexity: "O(1)",
        file_path: "crates/kavach-cli/src/cmd/db/write.rs",
        search_year: 2026,
        search_month: 7,
    };
    let id = kavach_surreal::algo_upsert(&db, &p).await?;
    println!("wrote algo_decision id={id:?}");
    Ok(())
}
