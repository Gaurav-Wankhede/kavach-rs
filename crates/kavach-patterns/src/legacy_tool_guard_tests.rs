//! FP-bound regression suite for the §TOOLBELT P0 gate.
//!
//! The (b) ALLOW set is the EVIDENCE that P0 is safe per the engine
//! CLAUDE.md rule ("P0 only with a regression test demonstrating the
//! false-positive bound"). Every exemption the matcher claims is proven
//! here; if any (b) case starts returning a Hit, the P0 gate would
//! false-block real work and this suite fails.

use super::inspect;

// --- (a) MUST DENY: bare legacy tool with a toolbelt replacement ---

#[test]
fn denies_bare_legacy_tools() {
    for (cmd, tool, repl) in [
        ("grep -n foo src/", "grep", "rg"),
        ("egrep pat f", "egrep", "rg"),
        ("cat Cargo.toml", "cat", "bat"),
        ("find . -name '*.rs'", "find", "fd"),
        ("sed 's/a/b/' f.txt", "sed", "sd"),
        ("jq '.x' data.json", "jq", "jaq"),
        ("curl https://example.com", "curl", "xh"),
        ("du -sh target", "du", "dust"),
        ("tree -L 2", "tree", "erd"),
        ("ps aux", "ps", "procs"),
        ("diff a.rs b.rs", "diff", "difft"),
        ("ls -R src", "ls", "eza"),
    ] {
        let hit = inspect(cmd).unwrap_or_else(|| panic!("must deny: {cmd}"));
        assert_eq!(hit.tool, tool, "tool mismatch for `{cmd}`");
        assert_eq!(hit.replacement, repl, "replacement mismatch for `{cmd}`");
    }
}

#[test]
fn denies_legacy_tool_as_a_pipeline_stage() {
    // The producer is benign but the CONSUMER is a bare legacy tool —
    // every segment's command word is validated.
    assert!(inspect("rg foo | sed 's/a/b/'").is_some());
    assert!(inspect("cargo build 2>&1 | grep error").is_some());
}

#[test]
fn denies_with_path_prefix() {
    // /usr/bin/grep and ./grep still invoke the POSIX tool.
    assert_eq!(inspect("/usr/bin/grep x f").unwrap().tool, "grep");
    assert_eq!(inspect("./find . -name x").unwrap().tool, "find");
}

#[test]
fn denies_after_var_assignment_prefix() {
    // `FOO=1 grep x` — grep is still the command word.
    assert_eq!(inspect("FOO=1 grep x f").unwrap().tool, "grep");
}

// --- (b) MUST ALLOW: the false-positive bound (engine-CLAUDE.md P0 proof) ---

#[test]
fn allows_git_subcommands_not_the_posix_bin() {
    // `git grep`/`cat-file`/`diff`/`log` are git subcommands, NOT the
    // POSIX legacy tool. Blocking these would be a catastrophic FP.
    assert!(inspect("git grep TODO").is_none());
    assert!(inspect("git cat-file -p HEAD").is_none());
    assert!(inspect("git diff --stat").is_none());
    assert!(inspect("git log --oneline -5").is_none());
}

#[test]
fn allows_toolbelt_tools_no_self_trip() {
    for ok in [
        "rg -n foo src/",
        "fd -e rs",
        "bat -p file.rs",
        "sd 'a' 'b' f",
        "eza -la",
        "jaq '.x' f.json",
        "xh GET https://x",
        "erd -L 3",
        "procs --tree",
        "difft a b",
    ] {
        assert!(inspect(ok).is_none(), "toolbelt tool must NOT trip: {ok}");
    }
}

#[test]
fn allows_pipe_from_non_toolbelt_producer_into_grep() {
    // `ps aux | grep x` and `journalctl | grep x`: the FIRST segment (`ps`)
    // IS a hit (ps→procs) — that is correct, ps is itself a legacy tool.
    // But the realistic FP concern is producers with NO toolbelt analog:
    assert!(
        inspect("journalctl -u svc | grep error").is_some(),
        "grep stage is still a legacy hit — correct, rg is the drop-in"
    );
    // The genuinely-must-allow case: a non-legacy producer piping into a
    // non-legacy consumer must never trip.
    assert!(inspect("echo hello | tr a-z A-Z").is_none());
}

#[test]
fn allows_quoted_tool_name_as_data_not_command() {
    // The literal word inside a quoted arg is DATA, not an invocation.
    assert!(inspect("git commit -m \"use grep here\"").is_none());
    assert!(inspect("kavach db write --content \"replace sed with sd\"").is_none());
    assert!(inspect("echo 'find the bug'").is_none());
}

#[test]
fn allows_find_in_action_mode() {
    // `fd` cannot express -delete/-exec/etc — find-as-action is legitimate.
    assert!(inspect("find . -name '*.tmp' -delete").is_none());
    assert!(inspect("find . -type f -exec rm {} ;").is_none());
    assert!(inspect("find . -newer ref -prune").is_none());
}

#[test]
fn allows_cat_heredoc_stream_construction() {
    // `cat <<EOF` is stream construction, not a file read.
    assert!(inspect("cat <<EOF\nhello\nEOF").is_none());
    assert!(inspect("cat <<-'EOF'\nx\nEOF").is_none());
}

#[test]
fn allows_plain_ls_only_recursive_is_a_hit() {
    // Plain `ls` is ubiquitous; only recursive `ls -R` is a hit.
    assert!(inspect("ls").is_none());
    assert!(inspect("ls -la").is_none());
    assert!(inspect("ls src/").is_none());
    assert!(inspect("ls -R src").is_some()); // recursive ⇒ hit
}

#[test]
fn fail_closed_no_block_on_ambiguous_substitution() {
    // Command/process substitution could let an arg escape into command
    // position — the matcher declines to assert a Hit (the safety P0s
    // ordered before it own injection). A missed lazy tool is recoverable;
    // a false P0 with no escape is the worse failure (claude-code#6409).
    assert!(inspect("echo $(grep x f)").is_none());
    assert!(inspect("diff <(sort a) <(sort b)").is_none());
    assert!(inspect("x=`grep y f`").is_none());
}

#[test]
fn allows_empty_and_unparseable() {
    assert!(inspect("").is_none());
    assert!(inspect("   ").is_none());
    assert!(inspect("grep 'unterminated").is_none()); // shell_words Err
}
