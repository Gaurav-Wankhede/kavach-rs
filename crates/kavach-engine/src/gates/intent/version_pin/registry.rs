pub(super) fn npm_url(name: &str) -> String {
    format!("https://registry.npmjs.org/{name}")
}

pub(super) fn pypi_url(name: &str) -> String {
    format!("https://pypi.org/pypi/{name}/json")
}

pub(super) fn crates_index_url(name: &str) -> String {
    let n = name.to_lowercase();
    let base = "https://index.crates.io";
    let chars: Vec<char> = n.chars().collect();
    match chars.len() {
        0 => base.to_owned(),
        1 => format!("{base}/1/{n}"),
        2 => format!("{base}/2/{n}"),
        3 => {
            let first: String = chars.iter().take(1).collect();
            format!("{base}/3/{first}/{n}")
        }
        _ => {
            let p1: String = chars.iter().take(2).collect();
            let p2: String = chars.iter().skip(2).take(2).collect();
            format!("{base}/{p1}/{p2}/{n}")
        }
    }
}

#[cfg(test)]
#[path = "registry_test.rs"]
mod registry_test;
