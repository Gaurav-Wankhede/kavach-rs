// Security allowlists and validation guards for SurrealQL injection prevention.
// SECURITY: All field/edge/table names are allow-listed to prevent SurrealQL injection.
// SOURCE: https://github.com/orgs/surrealdb/discussions/1330 (SurrealQL injection guidance)

const ALLOWED_FIELDS: &[&str] = &[
    "project",
    "entry_key",
    "title",
    "content",
    "status",
    "entry_status",
    "category",
    "tags",
    "decay_score",
    "access_count",
    "created_at",
    "updated_at",
    "accessed_at",
    "source",
    "spec_key",
];

const ALLOWED_EDGES: &[&str] = &[
    "serves",
    "implements",
    "blocks",
    "depends_on",
    "references",
    "mentions",
    "supersedes",
    "contains",
    "modifies",
    "uses_skill",
];

const ALLOWED_TABLES: &[&str] = &[
    "decision", "research", "roadmap", "pattern", "app_spec", "project", "session", "entity",
    "kanban", "part",
];

pub(super) fn is_allowed_field(name: &str) -> bool {
    ALLOWED_FIELDS.contains(&name)
}

pub(super) fn is_allowed_edge(name: &str) -> bool {
    ALLOWED_EDGES.contains(&name)
}

pub(super) fn is_allowed_table(name: &str) -> bool {
    ALLOWED_TABLES.contains(&name)
}

pub(super) fn is_valid_duration(s: &str) -> bool {
    if s.is_empty() || s.len() > 16 {
        return false;
    }
    let last = match s.as_bytes().last() {
        Some(b) => *b,
        None => return false,
    };
    if !matches!(last, b'd' | b'h' | b'm' | b's' | b'w' | b'y') {
        return false;
    }
    let split_idx = s.len().saturating_sub(1);
    let Some((digits, _)) = s.split_at_checked(split_idx) else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

pub(super) fn is_safe_key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub(super) fn safe_field(name: &str) -> Option<&str> {
    is_allowed_field(name).then_some(name)
}

pub const NEVER_MATCH: &str = "1 = 2";
