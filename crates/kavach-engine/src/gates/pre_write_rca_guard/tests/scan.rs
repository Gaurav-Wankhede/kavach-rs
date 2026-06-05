//! Transcript-scanner regression tests for the complete-record read that
//! replaced the broken 32 KB byte-tail (rca.rca-gate-32kb-line-split).
use super::super::scan_transcript_for_rca;
use std::io::Write;

/// REGRESSION rca.rca-gate-32kb-line-split: a transcript whose single assistant
/// JSONL record is >32 KB with `[RCA]` near its START must be detected. The old
/// byte-tail sliced such a record on both ends and returned false. Now the
/// scanner reads complete records: the whole >32 KB single line is the only
/// record, so dropping a partial leading line leaves it intact and `[RCA]` is found.
#[test]
fn scan_detects_rca_in_oversize_jsonl_record() {
    let filler = "x".repeat(64 * 1024);
    let line = format!(
        r#"{{"role":"assistant","content":[{{"type":"text","text":"[RCA]\nsymptom: oversize record {filler}"}}]}}"#
    );
    let mut p = std::env::temp_dir();
    p.push(format!("kavach_rca_scan_test_{}.jsonl", std::process::id()));
    {
        let mut f = std::fs::File::create(&p).expect("create temp transcript");
        writeln!(f, r#"{{"role":"user","content":"go"}}"#).expect("write user line");
        writeln!(f, "{line}").expect("write oversize assistant line");
    }
    let found = scan_transcript_for_rca(&p.to_string_lossy());
    std::fs::remove_file(&p).ok();
    assert!(
        found,
        "scan must find [RCA] in a >32KB single JSONL record (byte-tail \
         bug would slice it and return false)"
    );
}

/// A genuinely RCA-free transcript still returns false (no over-block regression
/// from the complete-record read).
#[test]
fn scan_returns_false_when_no_rca_in_oversize_record() {
    let filler = "y".repeat(64 * 1024);
    let line = format!(
        r#"{{"role":"assistant","content":[{{"type":"text","text":"just analysis, no marker {filler}"}}]}}"#
    );
    let mut p = std::env::temp_dir();
    p.push(format!("kavach_rca_scan_neg_{}.jsonl", std::process::id()));
    {
        let mut f = std::fs::File::create(&p).expect("create temp transcript");
        writeln!(f, "{line}").expect("write oversize assistant line");
    }
    let found = scan_transcript_for_rca(&p.to_string_lossy());
    std::fs::remove_file(&p).ok();
    assert!(
        !found,
        "no [RCA] / rca. decision present ⇒ must return false"
    );
}
