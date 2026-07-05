use std::io::Read as _;

use kavach_toon::caveman::{self, Level};

/// `kavach caveman` — compress stdin with the deployed caveman compressor (debug/witness).
pub(super) fn run(level: &str, verify: bool) -> i32 {
    let level = match level.to_ascii_lowercase().as_str() {
        "lite" => Level::Lite,
        "full" => Level::Full,
        "ultra" => Level::Ultra,
        other => {
            eprintln!("kavach caveman: unknown --level '{other}' (expected lite|full|ultra)");
            return 2;
        }
    };

    let mut input = String::new();
    if let Err(e) = std::io::stdin().lock().read_to_string(&mut input) {
        eprintln!("kavach caveman: failed to read stdin: {e}");
        return 2;
    }

    // SOURCE: anthropic.com/engineering/effective-context-engineering-for-ai-agents
    let output = caveman::compress(&input, level);
    println!("{output}");

    if verify && let Err(e) = caveman::assert_lossless(&input, &output) {
        eprintln!("kavach caveman: lossless check failed: {e}");
        return 1;
    }

    0
}
