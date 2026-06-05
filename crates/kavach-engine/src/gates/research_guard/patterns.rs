//! Static pattern tables: bug-fix intent triggers, valid research-source URLs,
//! and config-file exemptions.

// Bug/fix trigger tokens moved to the single source of truth:
// `kavach_config::research_triggers::BUG_FIX_TRIGGERS`. `requires_research` now
// delegates to `kavach_config::requires_research`, which consults that floor —
// removing the divergent local copy that caused the TABULA_RASA disagreement.

/// URL/source substrings that count as legitimate research evidence.
pub(super) static RESEARCH_PATTERNS: &[&str] = &[
    // Code repositories & issues
    "github.com",
    "gitlab.com",
    "issues/",
    "pull/",
    "discussions/",
    // Academic & research
    "arxiv.org",
    "acm.org",
    "ieee.org",
    "research.google",
    // Q&A & community
    "stackoverflow.com",
    "stackexchange.com",
    "dev.to",
    "reddit.com/r/rust",
    "reddit.com/r/programming",
    // DSA & competitive programming
    "leetcode.com",
    "hackerrank.com",
    "codeforces.com",
    "atcoder.jp",
    "topcoder.com",
    "codewars.com",
    "exercism.org",
    "neetcode.io",
    "algoexpert.io",
    "algocademy.com",
    "geeksforgeeks.org",
    "projecteuler.net",
    // System design & architecture
    "roadmap.sh",
    "systemdesignhandbook.com",
    "awesome-architecture.com",
    "martinfowler.com",
    "highscalability.com",
    "infoq.com",
    "dzone.com",
    // Rust ecosystem
    "crates.io",
    "docs.rs",
    "lib.rs",
    "rustsec.org",
    "rust-lang.org",
    // Database documentation
    "docs.scylladb.com",
    "scylladb.com",
    "postgresql.org",
    "neon.tech",
    "supabase.com",
    "planetscale.com",
    // Cloudflare ecosystem
    "developers.cloudflare.com",
    "blog.cloudflare.com",
    // Official docs & blogs
    "learn.microsoft.com",
    "cloud.google.com",
    "aws.amazon.com/blogs",
    "fly.io/blog",
    "jepsen.io",
    // WebSearch patterns
    "WebFetch(domain:github.com)",
    "WebSearch github",
    "WebSearch arxiv",
    "WebSearch stackoverflow",
    "WebSearch system design",
    "WebSearch scylladb",
    "WebSearch cloudflare",
];

/// Path substrings exempt from the research requirement (config/docs, not code).
pub(super) static CONFIG_EXEMPT_PATTERNS: &[&str] = &[
    "/.claude/",
    "/CLAUDE.md",
    ".json",
    "settings.json",
    "claude-progress.txt",
];
