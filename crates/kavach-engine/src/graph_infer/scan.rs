//! Word-boundary substring scan: does `content` mention `key` as a whole token?

/// True iff `key` appears in `content` bounded by non-word bytes on both sides.
pub(super) fn mentions_key(content: &str, key: &str) -> bool {
    let bytes = content.as_bytes();
    let kb = key.as_bytes();
    if bytes.len() < kb.len() {
        return false;
    }
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bytes.len() > kb.len() proved by guard; subtraction safe"
    )]
    let max_i = bytes.len() - kb.len();
    for i in 0..=max_i {
        match bytes.get(i..i.saturating_add(kb.len())) {
            Some(slice) if slice == kb => {
                let before_ok = i
                    .checked_sub(1)
                    .and_then(|p| bytes.get(p))
                    .is_none_or(|b| !is_word_byte(*b));
                let after = i.saturating_add(kb.len());
                let after_ok = bytes.get(after).is_none_or(|b| !is_word_byte(*b));
                if before_ok && after_ok {
                    return true;
                }
            }
            Some(_) | None => {}
        }
    }
    false
}

const fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}
