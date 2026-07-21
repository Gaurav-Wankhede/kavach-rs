pub(super) fn status_rank(status: &str) -> u8 {
    match status {
        "verified" => 0,
        "active" | "done" => 1,
        "todo" => 2,
        _ => 3,
    }
}

pub(super) fn status_class(status: &str) -> &'static str {
    match status {
        "verified" | "active" | "done" => "done",
        "todo" => "open",
        _ => "draft",
    }
}

pub(super) fn dm_sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// SOURCE: kavach decision.context-rot-mermaid-label-truncate
pub(super) fn dm_escape(label: &str) -> String {
    label.replace('"', "&quot;").replace(['\n', '\r'], " ")
}

const DM_LABEL_MAX: usize = 60;

pub(super) fn dm_truncate(label: &str) -> String {
    let escaped = dm_escape(label);
    if escaped.len() <= DM_LABEL_MAX {
        return escaped;
    }
    let mut out = String::with_capacity(DM_LABEL_MAX + 3);
    let mut used = 0usize;
    for ch in escaped.chars() {
        let next = used.saturating_add(ch.len_utf8());
        if next > DM_LABEL_MAX {
            break;
        }
        out.push(ch);
        used = next;
    }
    out.push_str("…");
    out
}
    label.replace('"', "&quot;").replace(['\n', '\r'], " ")
}
