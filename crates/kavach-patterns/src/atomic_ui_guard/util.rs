use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use super::types::Level;

pub(super) fn regex_matches(opt_re: Option<&Option<Regex>>, text: &str) -> bool {
    opt_re
        .and_then(|o| o.as_ref())
        .is_some_and(|re| re.is_match(text))
}

pub(super) fn regex_find_any(
    opt_re: Option<&Option<Regex>>,
    text: &str,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    opt_re
        .and_then(|o| o.as_ref())
        .is_some_and(|re| re.find_iter(text).any(|m| predicate(m.as_str())))
}

pub(super) fn classify_path(path: &str) -> Level {
    let p = path.to_lowercase();
    if p.contains("/atoms/")
        || p.contains("\\atoms\\")
        || p.ends_with("/atoms.rs")
        || p.contains("/atom/")
    {
        return Level::Atom;
    }
    if p.contains("/molecules/")
        || p.contains("\\molecules\\")
        || p.ends_with("/molecules.rs")
        || p.contains("/molecule/")
    {
        return Level::Molecule;
    }
    if p.contains("/organisms/")
        || p.contains("\\organisms\\")
        || p.ends_with("/organisms.rs")
        || p.contains("/organism/")
    {
        return Level::Organism;
    }
    if p.contains("/templates/") || p.contains("\\templates\\") {
        return Level::Template;
    }
    if p.contains("/pages/") || p.contains("\\pages\\") || p.contains("/routes/") {
        return Level::Page;
    }
    Level::Unknown
}

pub(super) static PATTERNS: LazyLock<Vec<Option<Regex>>> = LazyLock::new(|| {
    vec![
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*molecules?(?:/|['"]|::)"#).ok(),
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*organisms?(?:/|['"]|::)"#).ok(),
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*templates?(?:/|['"]|::)"#).ok(),
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*pages?(?:/|['"]|::)"#).ok(),
        Regex::new(r"(?:useStore|useSelector|useDispatch|useAtom|useRecoilState|useRecoilValue|defineStore|createStore|GlobalSignal|use_global)").ok(),
        Regex::new(r"(?:fetch\s*\(|axios\.(?:get|post|put|patch|delete)|ky\.(?:get|post|put|delete)|\$fetch\s*\(|reqwest::|surf::|reqwasm::)").ok(),
        Regex::new(r"style\s*=\s*\{?\{[^}]*(?:#[0-9a-fA-F]{3,8}|rgb\s*\(|rgba\s*\(|hsl\s*\()").ok(),
        Regex::new(r#"(?:className|class|class:list)\s*=\s*[`"'][^`"']*\[\d+px\]"#).ok(),
        Regex::new(r"<img\b[^>]*>").ok(),
        Regex::new(r"<button\b[^>]*>\s*(?:<svg|<i\s+class|<Icon\b)").ok(),
        Regex::new(r"(?s)(?:\.map\s*\(|v-for|#each|\{#each)[^<]*<[a-zA-Z]\w*[^>]*>").ok(),
        Regex::new(r#"(?:className|class|color|bg|background)\s*[:=]\s*[`"']?[^`"']*#[0-9a-fA-F]{3,8}\b"#).ok(),
        Regex::new(r#"(?:className|class)\s*=\s*[`"'][^`"']*\bbg-white\b[^`"']*[`"']"#).ok(),
        Regex::new(r"(?:localStorage|sessionStorage)\s*\.\s*(?:setItem|getItem|removeItem)").ok(),
        Regex::new(r"(?:console\.(?:log|debug|trace)|tracing::debug!|log::debug!)").ok(),
    ]
});

pub(super) fn is_ui_file(path: &str, content: &str) -> bool {
    let p = path.to_lowercase();
    let ext = Path::new(&p).extension().and_then(|e| e.to_str());
    if matches!(ext, Some("tsx" | "jsx" | "vue" | "svelte" | "astro"))
        || (ext == Some("ts") && (p.contains("/components/") || p.contains("/ui/")))
        || (ext == Some("js") && (p.contains("/components/") || p.contains("/ui/")))
        || (ext == Some("rs")
            && (p.contains("/components/")
                || p.contains("/ui/")
                || p.contains("/atoms/")
                || p.contains("/molecules/")
                || p.contains("/organisms/")))
    {
        return true;
    }
    if ext != Some("rs") {
        return false;
    }
    let stripped: String = content
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    DIOXUS_MARKER
        .as_ref()
        .is_some_and(|re| re.is_match(&stripped))
        || stripped.contains("dioxus::prelude")
}

static DIOXUS_MARKER: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\brsx!\s*[{(\[]|#\[component\]").ok());
