//! One-shot authoritative probe: run the real `advise` over the workspace and
//! print every file the deployed gate actually flags. `#[ignore]` by default.

use super::advise;

fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            if let Ok(c) = std::fs::read_to_string(&p) {
                if let Some(s) = p.to_str() {
                    if advise(s, &c).is_some() {
                        out.push(s.to_owned());
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "manual probe — run with --ignored --nocapture"]
fn list_workspace_bloat() {
    let mut out = Vec::new();
    walk(std::path::Path::new("../../crates"), &mut out);
    walk(std::path::Path::new("crates"), &mut out);
    out.sort();
    out.dedup();
    eprintln!("GATE_FLAGGED_COUNT={}", out.len());
    for f in &out {
        eprintln!("FLAG {f}");
    }
}
