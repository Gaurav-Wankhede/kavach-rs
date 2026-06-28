//! Parallel scan over the selected lenses — scoped-thread sharding, zero-LLM.
use super::finding::Finding;
use super::lens::{self, Selection};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Scan every file across the selected lenses on scoped threads, merged + deduped.
#[must_use]
pub(super) fn scan_all(root: &Path, files: &[PathBuf], sel: Selection) -> Vec<Finding> {
    let out: Mutex<Vec<Finding>> = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let chunk = files.len().div_ceil(workers).max(1);
    std::thread::scope(|s| {
        for shard in files.chunks(chunk) {
            let out = &out;
            s.spawn(move || {
                let mut local = Vec::new();
                for path in shard {
                    let Ok(src) = std::fs::read_to_string(path) else {
                        continue;
                    };
                    let rel = path.strip_prefix(root).unwrap_or(path);
                    let file = rel.to_string_lossy();
                    local.extend(lens::scan_file(&file, &src, sel));
                }
                if let Ok(mut g) = out.lock() {
                    g.append(&mut local);
                }
            });
        }
    });
    let mut v = out.into_inner().unwrap_or_default();
    v.sort_by_key(Finding::dedup_key);
    v.dedup_by_key(|f| f.dedup_key());
    v
}
