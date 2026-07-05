// SOURCE: github.com/JuliusBrussee/compact README (fetched 2026-07-06)
use super::Level;

const fn is_sentinel(c: char) -> bool {
    c == '\u{0}'
}

fn drop_word(text: &str, word: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut buf = String::new();
    while let Some(c) = chars.next() {
        if is_sentinel(c) {
            buf.push(c);
            while let Some(&n) = chars.peek() {
                buf.push(n);
                chars.next();
                if is_sentinel(n) && buf.matches(is_sentinel).count() >= 2 {
                    break;
                }
            }
            out.push_str(&buf);
            buf.clear();
            continue;
        }
        if c.is_alphanumeric() {
            buf.push(c);
        } else {
            flush_word(&mut out, &buf, word);
            buf.clear();
            out.push(c);
        }
    }
    flush_word(&mut out, &buf, word);
    out
}

fn flush_word(out: &mut String, buf: &str, word: &str) {
    if buf.eq_ignore_ascii_case(word) {
        return;
    }
    out.push_str(buf);
}

fn collapse_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        if c == ' ' {
            if !prev_space {
                out.push(c);
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim().to_owned()
}

const HEDGES: &[&str] = &["please", "kindly", "just", "simply", "really", "very"];
const ARTICLES: &[&str] = &["a", "an", "the"];
const COPULAS: &[&str] = &["is", "are", "be", "was", "were", "am"];
const ULTRA_DROPS: &[&str] = &[
    "that", "which", "of", "to", "with", "for", "will", "must", "should", "do", "does",
];

fn apply_lite(masked: &str) -> String {
    let mut out = masked
        .replace("in order to", "to")
        .replace("I'd be happy to", "");
    for word in HEDGES {
        out = drop_word(&out, word);
    }
    collapse_spaces(&out)
}

fn apply_full(masked: &str) -> String {
    let mut out = apply_lite(masked);
    out = out.replace("and then", "\u{2192}");
    out = out.replace(" then ", " \u{2192} ");
    for word in ARTICLES.iter().chain(COPULAS.iter()) {
        out = drop_word(&out, word);
    }
    collapse_spaces(&out)
}

fn apply_ultra(masked: &str) -> String {
    let mut out = apply_full(masked);
    for word in ULTRA_DROPS {
        out = drop_word(&out, word);
    }
    collapse_spaces(&out)
}

pub(super) fn drop_grammar(masked: &str, level: Level) -> String {
    match level {
        Level::Lite => apply_lite(masked),
        Level::Full => apply_full(masked),
        Level::Ultra => apply_ultra(masked),
    }
}
