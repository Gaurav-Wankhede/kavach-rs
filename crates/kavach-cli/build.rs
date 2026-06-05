// Build script: emit timestamp + git SHA as compile-time env vars so the
// kavach binary can report which build it is via `kavach --version` and
// `kavach status`.
//
// SOURCE: https://doc.rust-lang.org/cargo/reference/build-scripts.html
//
// Honors SOURCE_DATE_EPOCH for reproducible release builds. Falls back to
// wallclock time when unset.

const SECS_PER_DAY: i64 = 86_400;
const SECS_PER_HOUR: u32 = 3600;
const SECS_PER_MINUTE: u32 = 60;

fn main() {
    println!("cargo::rerun-if-changed=.git/HEAD");
    println!("cargo::rerun-if-changed=.git/refs");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=SOURCE_DATE_EPOCH");

    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo::rustc-env=KAVACH_GIT_SHA={git_sha}");

    let epoch = std::env::var("SOURCE_DATE_EPOCH").ok().unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or_else(|_| "0".to_owned(), |d| d.as_secs().to_string())
    });
    println!("cargo::rustc-env=KAVACH_BUILD_EPOCH={epoch}");

    let iso = format_iso(&epoch);
    println!("cargo::rustc-env=KAVACH_BUILD_TIMESTAMP={iso}");

    // Combined version string for clap: "<iso>+git:<sha>"
    println!("cargo::rustc-env=KAVACH_VERSION={iso}+git:{git_sha}");
}

fn format_iso(epoch_str: &str) -> String {
    let secs: i64 = epoch_str.parse().unwrap_or(0);
    let (y, mo, d, h, mi, s) = epoch_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn epoch_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    if secs <= 0 {
        return (1970, 1, 1, 0, 0, 0);
    }
    let days = secs.checked_div(SECS_PER_DAY).unwrap_or(0);
    let rem = secs.rem_euclid(SECS_PER_DAY).try_into().unwrap_or(0_u32);
    let h = rem.checked_div(SECS_PER_HOUR).unwrap_or(0);
    let mi = (rem % SECS_PER_HOUR)
        .checked_div(SECS_PER_MINUTE)
        .unwrap_or(0);
    let s = rem % SECS_PER_MINUTE;
    let (y, mo, d) = days_to_ymd(days);
    (y, mo, d, h, mi, s)
}

fn days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    let mut year: i32 = 1970;
    loop {
        let len = if is_leap(year) { 366 } else { 365 };
        if days < len {
            break;
        }
        days = days.saturating_sub(len);
        year = year.saturating_add(1);
    }
    let months = [31u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month_idx = 0usize;
    let remaining_days = u32::try_from(days).unwrap_or(0);
    let mut remaining = remaining_days;
    // Iterator + enumerate is the canonical panic-free pattern (vs direct
    // months[idx] which the /rust skill flags). Each yielded pair is
    // bounds-validated by the iterator itself.
    for (idx, &month_len) in months.iter().enumerate() {
        let len = if idx == 1 && is_leap(year) {
            29
        } else {
            month_len
        };
        if remaining < len {
            month_idx = idx;
            break;
        }
        remaining = remaining.saturating_sub(len);
        month_idx = idx.saturating_add(1);
    }
    let month_num = u32::try_from(month_idx).unwrap_or(11).saturating_add(1);
    let day = remaining.saturating_add(1);
    (year, month_num, day)
}

const fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
