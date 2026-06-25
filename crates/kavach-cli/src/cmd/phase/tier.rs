use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

fn parse_tier_line(content: &str) -> Option<kavach_types::ProjectTier> {
    for line in content.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("tier=") else {
            continue;
        };
        let value = match rest.split([';', ' ', '\n']).next() {
            Some(v) => v.trim(),
            None => continue,
        };
        if let Some(tier) = kavach_types::ProjectTier::parse(value) {
            return Some(tier);
        }
    }
    None
}

fn current_tier_for(project: &str) -> kavach_types::ProjectTier {
    let Ok(runtime) = tokio::runtime::Runtime::new() else {
        return kavach_types::ProjectTier::Refactor;
    };
    runtime.block_on(async { current_tier_for_async(project).await })
}

async fn current_tier_for_async(project: &str) -> kavach_types::ProjectTier {
    let Ok(db) = kavach_surreal::open_default_resilient().await else {
        return kavach_types::ProjectTier::Refactor;
    };
    let Ok(Some(project_rec)) = kavach_surreal::projects::get_by_slug(&db, project).await else {
        return kavach_types::ProjectTier::Refactor;
    };
    let Some(project_id) = project_rec.id else {
        return kavach_types::ProjectTier::Refactor;
    };
    let Ok(rows) = kavach_surreal::read::list_by_project(&db, "decision", &project_id).await else {
        return kavach_types::ProjectTier::Refactor;
    };
    rows.into_iter()
        .find(|row| row.entry_key == "workflow.tier.current")
        .and_then(|row| parse_tier_line(&row.content))
        .map_or(kavach_types::ProjectTier::Refactor, |v| v)
}

pub(crate) fn handle_tier_set(tier: &str, project: &str, reason: &str, allow_downgrade: bool) -> i32 {
    let tier_lower = tier.to_lowercase();
    let Some(new_tier) = kavach_types::ProjectTier::parse(&tier_lower) else {
        let msg = format!("invalid tier: {tier}. Valid: refactor, feature, platform");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    };

    let current_tier = current_tier_for(project);
    if !current_tier.can_promote_to(new_tier) && !allow_downgrade {
        let msg = format!(
            "[TIER_DOWNGRADE_REFUSED] project={project} current={} target={} \
             — downgrade requires --allow-downgrade (one-way promotion rule)",
            current_tier.as_str(),
            new_tier.as_str()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 2;
    }

    let direction = if current_tier == new_tier {
        "noop"
    } else if current_tier.can_promote_to(new_tier) {
        "promote"
    } else {
        "downgrade"
    };
    let line = format!(
        "[TIER_SET] project={project} {}={}→{} reason={reason}\n\
         Persist with: kavach db write --project {project} --category decision \
         --new --key workflow.tier.current --title 'Project tier: {}' \
         --content 'tier={}; reason={reason}'",
        direction,
        current_tier.as_str(),
        new_tier.as_str(),
        new_tier.as_str(),
        new_tier.as_str(),
    );
    if let Err(io_err) = print_or_exit(&line) {
        return into_exit_code(io_err);
    }
    0
}
