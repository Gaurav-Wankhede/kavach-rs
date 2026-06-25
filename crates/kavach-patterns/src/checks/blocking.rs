use crate::config::load;

#[must_use]
pub fn is_blocked(cmd: &str) -> bool {
	let g = load();
	let cl = cmd
		.to_lowercase()
		.split_whitespace()
		.collect::<Vec<_>>()
		.join(" ");
	g.as_ref().is_some_and(|c| {
		c.blocked.iter().any(|p| {
			let pl = p.to_lowercase();
			blocked_match(&cl, &pl)
		})
	})
}

/// Boundary-aware match for blocked command patterns.
/// - Matches inside quoted strings are skipped (SQL values, data args)
/// - "/" ending (rm -rf /): block only root, not /Users/foo
/// - "~" ending (rm -rf ~): block only bare ~, not ~/Downloads
/// - " sh"/" bash" ending: block pipe-to-shell, not | sha256sum
/// - Others: standard substring match
fn blocked_match(cmd: &str, pattern: &str) -> bool {
	let Some(pos) = cmd.find(pattern) else {
		return false;
	};
	if is_inside_quotes(cmd, pos) {
		return false;
	}
	let after = pos.saturating_add(pattern.len());
	let at_end = after >= cmd.len();
	if pattern.ends_with('/') || pattern.ends_with('~') {
		return at_end
			|| cmd
				.as_bytes()
				.get(after)
				.is_some_and(|&b| matches!(b, b'*' | b' '));
	}
	if pattern.ends_with(" sh") || pattern.ends_with(" bash") {
		return at_end
			|| cmd
				.as_bytes()
				.get(after)
				.is_some_and(|&b| matches!(b, b' ' | b'\'' | b'"' | b';' | b'&'));
	}
	true
}

/// Check if position falls inside a quoted string (single or double).
fn is_inside_quotes(s: &str, pos: usize) -> bool {
	let (mut sq, mut dq, mut i) = (false, false, 0);
	let b = s.as_bytes();
	while i < pos.min(b.len()) {
		match b.get(i) {
			Some(&b'\\') if dq => {
				i = i.saturating_add(2);
				continue;
			}
			Some(&b'\'') if !dq => sq = !sq,
			Some(&b'"') if !sq => dq = !dq,
			_ => {}
		}
		i = i.saturating_add(1);
	}
	sq || dq
}
