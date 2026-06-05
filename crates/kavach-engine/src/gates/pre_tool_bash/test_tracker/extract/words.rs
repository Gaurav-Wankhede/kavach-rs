//! Quote-aware shell-word tokenizer (POSIX XCU §2: quoted text is literal).

/// Whitespace-delimited tokens of `seg`, honoring single/double quotes
/// (chars inside quotes are literal and never start/extend a token past
/// the quote).
pub(super) fn segment_words(seg: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in seg.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else {
                    cur.push(c);
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                } else if c.is_whitespace() {
                    if !cur.is_empty() {
                        words.push(std::mem::take(&mut cur));
                    }
                } else {
                    cur.push(c);
                }
            }
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}
