// ARCH: FrontendStackDetector
//   {"name":"read package.json scripts","reason":"scripts are conventional, not authoritative; biome.json/eslint.config.* are the contract"},
//   {"name":"git-tracked file scan","reason":"requires git, fails in source distributions"}
// ]
// TIME: O(1) — constant stat() of 7 known config paths | SPACE: O(1)
// YEAR: 2026 | SEARCHED: 2026-05
// PATTERN: presence_check_priority_table | SCOPE: kavach-cli | CAP: AP
//
// Pure presence-based detector for `kavach verify-frontend`. No I/O beyond
// metadata; no parsing.

use std::path::Path;

/// Tool stack detected for a frontend project. Mutually exclusive — detector
/// picks one. `Tsc` is the fallback when no linter config is present.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FrontendStack {
    Biome,
    Eslint,
    Tsc,
}

/// Caller's preferred tool when multiple configs are present. `Auto` means
/// "use the detector's priority": biome > eslint > tsc.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Prefer {
    Auto,
    Biome,
    Eslint,
}

/// Package-script runner detected from project lockfile. Bun's bunx is a
/// drop-in npx replacement (~100× faster on local bins per 2026-05 research)
/// and works on systems where Node/npm isn't installed at all. Detection
/// order: bun lockfile → pnpm → yarn → npx fallback.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum PackageRunner {
    Bunx,
    PnpmDlx,
    YarnDlx,
    Npx,
}

impl PackageRunner {
    /// CLI program name to invoke this runner.
    pub(crate) const fn program(self) -> &'static str {
        match self {
            Self::Bunx => "bunx",
            Self::PnpmDlx => "pnpm",
            Self::YarnDlx => "yarn",
            Self::Npx => "npx",
        }
    }

    /// Leading args before the package name. `pnpm dlx <pkg>` and `yarn dlx <pkg>`
    /// require the `dlx` subcommand; bunx and npx invoke the package directly.
    pub(crate) const fn leading_args(self) -> &'static [&'static str] {
        match self {
            Self::Bunx | Self::Npx => &[],
            Self::PnpmDlx | Self::YarnDlx => &["dlx"],
        }
    }
}

/// Detect the project's package-script runner from its lockfile. Bun-only
/// projects (no Node) work because bunx executes packages without Node runtime.
/// Falls back to npx when no recognized lockfile is present.
///
/// Yarn 1.x (classic) does NOT support `yarn dlx` — that's Yarn 2+ (Berry) only.
/// To avoid a cryptic "command dlx not found" failure misreported as a lint
/// violation, yarn.lock contents are checked for the Berry header; classic
/// yarn.lock files (no `__metadata:` block) fall back to Npx since classic
/// users typically also have node/npm installed.
pub(crate) fn detect_runner(root: &Path) -> PackageRunner {
    if root.join("bun.lockb").exists() || root.join("bun.lock").exists() {
        return PackageRunner::Bunx;
    }
    if root.join("pnpm-lock.yaml").exists() {
        return PackageRunner::PnpmDlx;
    }
    if let Some(runner) = detect_yarn_runner(root) {
        return runner;
    }
    PackageRunner::Npx
}

/// Probe `<root>/yarn.lock`. Returns `YarnDlx` for Yarn 2+ (Berry) lockfiles,
/// Npx for Yarn 1.x (classic — no dlx subcommand), None if no yarn.lock at all.
/// Berry lockfiles contain a `__metadata:` block as their version marker;
/// classic lockfiles start with `# yarn lockfile v1`.
fn detect_yarn_runner(root: &Path) -> Option<PackageRunner> {
    let yarn_lock = root.join("yarn.lock");
    if !yarn_lock.exists() {
        return None;
    }
    // Read first 4 KB — enough to capture the header on any yarn.lock.
    let Ok(head) = std::fs::read(&yarn_lock) else {
        return Some(PackageRunner::Npx); // unreadable → safer fallback
    };
    let prefix_len = head.len().min(4096);
    let head_str = head
        .get(..prefix_len)
        .map_or_else(|| String::from_utf8_lossy(&head), String::from_utf8_lossy);
    if head_str.contains("__metadata:") {
        Some(PackageRunner::YarnDlx)
    } else {
        // Yarn 1.x (`# yarn lockfile v1` header) or unrecognized → use npx.
        Some(PackageRunner::Npx)
    }
}

/// Detect which lint stack a project uses. Priority: explicit `--prefer` wins
/// when its config exists, else falls through to the standard priority order:
/// biome.json → eslint flat-config → bare tsconfig.
/// Returns `None` if the directory has no recognizable frontend config at all.
pub(crate) fn detect_stack(root: &Path, prefer: Prefer) -> Option<FrontendStack> {
    let has_biome = root.join("biome.json").exists() || root.join("biome.jsonc").exists();
    let has_eslint = root.join("eslint.config.js").exists()
        || root.join("eslint.config.mjs").exists()
        || root.join("eslint.config.ts").exists()
        || root.join("eslint.config.cjs").exists();
    let has_tsconfig = root.join("tsconfig.json").exists();

    let explicit = match prefer {
        Prefer::Biome if has_biome => Some(FrontendStack::Biome),
        Prefer::Eslint if has_eslint => Some(FrontendStack::Eslint),
        Prefer::Auto | Prefer::Biome | Prefer::Eslint => None,
    };
    if explicit.is_some() {
        return explicit;
    }

    if has_biome {
        Some(FrontendStack::Biome)
    } else if has_eslint {
        Some(FrontendStack::Eslint)
    } else if has_tsconfig {
        Some(FrontendStack::Tsc)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    fn tmp_root(name: &str) -> PathBuf {
        let mut p = env::temp_dir();
        p.push(format!("kavach-vfe-detect-{name}-{}", std::process::id()));
        fs::remove_dir_all(&p).ok();
        fs::create_dir_all(&p).ok();
        p
    }

    #[test]
    fn detect_returns_none_for_empty_dir() {
        let root = tmp_root("empty");
        assert_eq!(detect_stack(&root, Prefer::Auto), None);
    }

    #[test]
    fn detect_picks_biome_when_only_biome_present() {
        let root = tmp_root("biome");
        fs::write(root.join("biome.json"), "{}").ok();
        assert_eq!(
            detect_stack(&root, Prefer::Auto),
            Some(FrontendStack::Biome)
        );
    }

    #[test]
    fn detect_picks_eslint_when_only_eslint_present() {
        let root = tmp_root("eslint");
        fs::write(root.join("eslint.config.mjs"), "export default [];").ok();
        assert_eq!(
            detect_stack(&root, Prefer::Auto),
            Some(FrontendStack::Eslint)
        );
    }

    #[test]
    fn detect_falls_back_to_tsc_with_only_tsconfig() {
        let root = tmp_root("tsc-only");
        fs::write(root.join("tsconfig.json"), "{}").ok();
        assert_eq!(detect_stack(&root, Prefer::Auto), Some(FrontendStack::Tsc));
    }

    #[test]
    fn detect_biome_wins_over_eslint_in_auto_mode() {
        let root = tmp_root("both");
        fs::write(root.join("biome.json"), "{}").ok();
        fs::write(root.join("eslint.config.mjs"), "export default [];").ok();
        assert_eq!(
            detect_stack(&root, Prefer::Auto),
            Some(FrontendStack::Biome)
        );
    }

    #[test]
    fn detect_eslint_explicit_prefer_overrides_biome() {
        let root = tmp_root("prefer-eslint");
        fs::write(root.join("biome.json"), "{}").ok();
        fs::write(root.join("eslint.config.mjs"), "export default [];").ok();
        assert_eq!(
            detect_stack(&root, Prefer::Eslint),
            Some(FrontendStack::Eslint)
        );
    }

    #[test]
    fn detect_runner_picks_bunx_for_bun_lockb() {
        let root = tmp_root("bun-lockb");
        fs::write(root.join("bun.lockb"), "").ok();
        assert_eq!(detect_runner(&root), PackageRunner::Bunx);
    }

    #[test]
    fn detect_runner_picks_bunx_for_bun_lock_text() {
        let root = tmp_root("bun-lock-text");
        fs::write(root.join("bun.lock"), "").ok();
        assert_eq!(detect_runner(&root), PackageRunner::Bunx);
    }

    #[test]
    fn detect_runner_picks_pnpm_for_pnpm_lock() {
        let root = tmp_root("pnpm");
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: 9.0\n").ok();
        assert_eq!(detect_runner(&root), PackageRunner::PnpmDlx);
    }

    #[test]
    fn detect_runner_picks_yarn_dlx_for_berry_lockfile() {
        let root = tmp_root("yarn-berry");
        // Yarn 2+ (Berry) lockfiles always contain __metadata: block at top.
        fs::write(
            root.join("yarn.lock"),
            "__metadata:\n  version: 6\n  cacheKey: 8\n",
        )
        .ok();
        assert_eq!(detect_runner(&root), PackageRunner::YarnDlx);
    }

    #[test]
    fn detect_runner_falls_back_to_npx_for_yarn_classic() {
        let root = tmp_root("yarn-classic");
        // Yarn 1.x lockfiles start with this comment header — no dlx subcommand.
        fs::write(
            root.join("yarn.lock"),
            "# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n# yarn lockfile v1\n",
        )
        .ok();
        assert_eq!(detect_runner(&root), PackageRunner::Npx);
    }

    #[test]
    fn detect_runner_handles_empty_yarn_lock() {
        // Empty yarn.lock → classic-treatment (no __metadata header) → fallback to npx
        let root = tmp_root("yarn-empty");
        fs::write(root.join("yarn.lock"), "").ok();
        assert_eq!(detect_runner(&root), PackageRunner::Npx);
    }

    #[test]
    fn detect_runner_falls_back_to_npx() {
        let root = tmp_root("no-lockfile");
        assert_eq!(detect_runner(&root), PackageRunner::Npx);
    }

    #[test]
    fn detect_runner_bun_wins_over_pnpm_when_both_present() {
        let root = tmp_root("bun-and-pnpm");
        fs::write(root.join("bun.lockb"), "").ok();
        fs::write(root.join("pnpm-lock.yaml"), "").ok();
        assert_eq!(detect_runner(&root), PackageRunner::Bunx);
    }

    #[test]
    fn package_runner_program_names_match_binaries() {
        assert_eq!(PackageRunner::Bunx.program(), "bunx");
        assert_eq!(PackageRunner::PnpmDlx.program(), "pnpm");
        assert_eq!(PackageRunner::YarnDlx.program(), "yarn");
        assert_eq!(PackageRunner::Npx.program(), "npx");
    }

    #[test]
    fn package_runner_leading_args_correct() {
        assert!(PackageRunner::Bunx.leading_args().is_empty());
        assert!(PackageRunner::Npx.leading_args().is_empty());
        assert_eq!(PackageRunner::PnpmDlx.leading_args(), &["dlx"]);
        assert_eq!(PackageRunner::YarnDlx.leading_args(), &["dlx"]);
    }
}
