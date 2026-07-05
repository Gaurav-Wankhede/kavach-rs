//! One-off: record the algo_decision row for the relationships sort+dedup choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = kavach_surreal::open_default().await?;
    let project = kavach_surreal::project_get_by_slug(&db, "kavach-rs")
        .await?
        .ok_or("project not found")?;
    let project_id = project.id.ok_or("project has no id")?;
    let p = kavach_surreal::AlgoUpsertParams {
        project: project_id,
        problem_class: "small-set dedup (n<20 edges/row)",
        chosen: "sort_unstable + dedup over Vec<ExtractedRelationship>",
        time_complexity: "O(n log n)",
        space_complexity: "O(n)",
        file_path: "crates/kavach-engine/src/gates/event_log/relationships.rs",
        search_year: 2026,
        search_month: 7,
    };
    let id = kavach_surreal::algo_upsert(&db, &p).await?;
    println!("wrote algo_decision id={id:?}");
    Ok(())
}
