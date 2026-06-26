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

pub(super) fn dm_escape(label: &str) -> String {
    label.replace('"', "&quot;").replace(['\n', '\r'], " ")
}
