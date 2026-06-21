// Framework-public env-var prefix allowlist for grep commands.
// See decision.engine.env-guard-grep-prefix-arch.

/// Detect grep commands that filter only on framework-public env-var prefixes.
///
/// PUBLIC_ vars (Astro), VITE_ (Vite), `NEXT_PUBLIC`_ (Next.js), `EXPO_PUBLIC`_ (Expo),
/// `REACT_APP`_ (CRA), `NUXT_PUBLIC`_ (Nuxt 3) — all framework-conventional non-secret
/// browser-exposed prefixes. Allows `grep PUBLIC_ .env` and similar.
///
/// Blocks if command also references known sensitive var name patterns
/// (secret, password, `database_url`, dsn, credential) — defense-in-depth.
pub(crate) fn is_public_prefix_grep(lc: &str) -> bool {
    let safe_prefixes = [
        "public_",
        "vite_",
        "next_public_",
        "expo_public_",
        "react_app_",
        "nuxt_public_",
    ];
    let has_safe = safe_prefixes.iter().any(|p| lc.contains(p));
    if !has_safe {
        return false;
    }
    let blocked = ["secret", "password", "database_url", "dsn", "credential"];
    !blocked.iter().any(|s| lc.contains(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_public_underscore_prefix() {
        assert!(is_public_prefix_grep("grep public_ .env"));
        assert!(is_public_prefix_grep("grep '^public_' .env"));
    }

    #[test]
    fn allows_framework_prefixes() {
        assert!(is_public_prefix_grep("grep vite_ .env"));
        assert!(is_public_prefix_grep("grep next_public_ .env"));
        assert!(is_public_prefix_grep("grep react_app_ .env"));
        assert!(is_public_prefix_grep("grep expo_public_ .env"));
        assert!(is_public_prefix_grep("grep nuxt_public_ .env"));
    }

    #[test]
    fn blocks_when_no_safe_prefix() {
        assert!(!is_public_prefix_grep("grep port .env"));
        assert!(!is_public_prefix_grep("grep stripe .env"));
    }

    #[test]
    fn blocks_when_blocked_keyword_co_occurs() {
        // PUBLIC_SECRET combination must NOT pass — defense-in-depth.
        assert!(!is_public_prefix_grep("grep public_ .env | grep secret"));
        assert!(!is_public_prefix_grep("grep public_database_url .env"));
    }

    #[test]
    fn blocks_unrelated_strings() {
        // Empty input, no env-related keywords, must not match.
        assert!(!is_public_prefix_grep(""));
        assert!(!is_public_prefix_grep("ls -la"));
    }
}
