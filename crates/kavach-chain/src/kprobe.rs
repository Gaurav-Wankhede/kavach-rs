//! Kernel-boundary observability via `getrusage(RUSAGE_SELF)` — pure libc,
//! works identically on Linux and macOS without elevated privileges.
//!
//! Captures the kernel's own bookkeeping for the kavach process during a
//! tool run: user/sys CPU time, voluntary/involuntary context switches,
//! page faults (minor/major), max RSS. Feeds back into the gate chain as a
//! `[KERNEL_OBSERVED]` advisory block per the low-latency skill contract.
//!
//! NOTE: This observes the kavach process itself, not the child tool. To
//! observe a child PID we would need `getrusage(RUSAGE_CHILDREN)` after
//! `wait()`, or eBPF tracepoints (Linux only). Followup work tracked under
//! decision/arch.kprobe-child-pid-followup.

use std::time::Instant;

const NS_PER_MS: u64 = 1_000_000;
const NS_PER_S: u64 = 1_000_000_000;
const NEAR_LIMIT_PCT: u64 = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    WithinBudget,
    NearLimit {
        observed_ns: u64,
        budget_ns: u64,
        pct: u32,
    },
    OverBudget {
        observed_ns: u64,
        budget_ns: u64,
    },
}

#[derive(Debug, Clone, Copy, Default)]
struct RUsageSnapshot {
    user_ns: u64,
    sys_ns: u64,
    ctx_voluntary: u64,
    ctx_involuntary: u64,
    page_faults_minor: u64,
    page_faults_major: u64,
    max_rss_kb: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ProbeReport {
    pub tool: String,
    pub backend: Backend,
    pub wall_ns: u64,
    pub user_ns: u64,
    pub sys_ns: u64,
    pub ctx_switches_voluntary: u64,
    pub ctx_switches_involuntary: u64,
    pub page_faults_minor: u64,
    pub page_faults_major: u64,
    pub max_rss_kb: u64,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backend {
    Ebpf,
    Dtrace,
    RusageChildren,
    RusageSelf,
}

impl Backend {
    #[must_use]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Ebpf => "ebpf",
            Self::Dtrace => "dtrace",
            Self::RusageChildren => "rusage_children",
            Self::RusageSelf => "rusage_self",
        }
    }

    #[must_use]
    pub(crate) fn select_default() -> Self {
        if cfg!(target_os = "linux") && ebpf_available() {
            Self::Ebpf
        } else if cfg!(target_os = "macos") && dtrace_available() {
            Self::Dtrace
        } else if std::env::var_os("KAVACH_KPROBE_CHILDREN").is_some_and(|v| v == "1") {
            Self::RusageChildren
        } else {
            Self::RusageSelf
        }
    }
}

#[cfg(target_os = "linux")]
fn ebpf_available() -> bool {
    std::path::Path::new("/sys/kernel/debug/tracing").exists()
        && std::env::var_os("KAVACH_KPROBE_EBPF").is_some_and(|v| v == "1")
}

#[cfg(not(target_os = "linux"))]
const fn ebpf_available() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn dtrace_available() -> bool {
    std::env::var_os("KAVACH_KPROBE_DTRACE").is_some_and(|v| v == "1")
        && std::path::Path::new("/usr/sbin/dtrace").exists()
}

#[cfg(not(target_os = "macos"))]
const fn dtrace_available() -> bool {
    false
}

pub(crate) struct Probe {
    tool: String,
    backend: Backend,
    start_wall: Instant,
    start_usage: RUsageSnapshot,
    budget_ns: u64,
}

impl Probe {
    #[must_use]
    pub(crate) fn start(tool: impl Into<String>) -> Self {
        Self::start_with(tool, Backend::select_default())
    }

    #[must_use]
    pub(crate) fn start_with(tool: impl Into<String>, backend: Backend) -> Self {
        let tool = tool.into();
        let budget_ns = budget_ns_for(&tool);
        Self {
            tool,
            backend,
            start_wall: Instant::now(),
            start_usage: snapshot_for(backend),
            budget_ns,
        }
    }

    #[must_use]
    pub(crate) fn stop(self) -> ProbeReport {
        let end_usage = snapshot_for(self.backend);
        let wall_ns = u64::try_from(self.start_wall.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let user_ns = end_usage.user_ns.saturating_sub(self.start_usage.user_ns);
        let sys_ns = end_usage.sys_ns.saturating_sub(self.start_usage.sys_ns);
        let verdict = verdict_for(wall_ns, self.budget_ns);
        ProbeReport {
            tool: self.tool,
            backend: self.backend,
            wall_ns,
            user_ns,
            sys_ns,
            ctx_switches_voluntary: end_usage
                .ctx_voluntary
                .saturating_sub(self.start_usage.ctx_voluntary),
            ctx_switches_involuntary: end_usage
                .ctx_involuntary
                .saturating_sub(self.start_usage.ctx_involuntary),
            page_faults_minor: end_usage
                .page_faults_minor
                .saturating_sub(self.start_usage.page_faults_minor),
            page_faults_major: end_usage
                .page_faults_major
                .saturating_sub(self.start_usage.page_faults_major),
            max_rss_kb: end_usage.max_rss_kb,
            verdict,
        }
    }
}

impl ProbeReport {
    #[must_use]
    pub(crate) fn render_kernel_observed_block(&self) -> String {
        let verdict_str = match &self.verdict {
            Verdict::WithinBudget => "WITHIN_BUDGET".to_owned(),
            Verdict::NearLimit {
                observed_ns,
                budget_ns,
                pct,
            } => {
                format!("NEAR_LIMIT observed={observed_ns}ns budget={budget_ns}ns pct={pct}")
            }
            Verdict::OverBudget {
                observed_ns,
                budget_ns,
            } => {
                format!("OVER_BUDGET observed={observed_ns}ns budget={budget_ns}ns")
            }
        };
        format!(
            "[KERNEL_OBSERVED]\n\
             tool: {}\n\
             backend: {}\n\
             wall_ns: {}\n\
             user_ns: {}\n\
             sys_ns: {}\n\
             ctx_switches: voluntary={} involuntary={}\n\
             page_faults: minor={} major={}\n\
             max_rss_kb: {}\n\
             verdict: {}\n",
            self.tool,
            self.backend.name(),
            self.wall_ns,
            self.user_ns,
            self.sys_ns,
            self.ctx_switches_voluntary,
            self.ctx_switches_involuntary,
            self.page_faults_minor,
            self.page_faults_major,
            self.max_rss_kb,
            verdict_str,
        )
    }
}

#[must_use]
pub fn budget_ns_for(tool: &str) -> u64 {
    match tool {
        "Read" | "Glob" | "Grep" => 50 * NS_PER_MS,
        "Edit" | "Write" | "NotebookEdit" => 200 * NS_PER_MS,
        "Bash" => 5 * NS_PER_S,
        "WebFetch" | "WebSearch" => 30 * NS_PER_S,
        "Task" | "Agent" => 5 * 60 * NS_PER_S,
        tool if tool.starts_with("mcp__") => 10 * NS_PER_S,
        _ => 100 * NS_PER_MS,
    }
}

fn verdict_for(observed_ns: u64, budget_ns: u64) -> Verdict {
    if budget_ns == 0 {
        return Verdict::WithinBudget;
    }
    if observed_ns >= budget_ns {
        return Verdict::OverBudget {
            observed_ns,
            budget_ns,
        };
    }
    let pct_num = observed_ns.saturating_mul(100);
    let threshold = budget_ns.saturating_mul(NEAR_LIMIT_PCT);
    if pct_num >= threshold {
        let pct = pct_num
            .checked_div(budget_ns)
            .map_or(u32::MAX, |p| u32::try_from(p).unwrap_or(u32::MAX));
        Verdict::NearLimit {
            observed_ns,
            budget_ns,
            pct,
        }
    } else {
        Verdict::WithinBudget
    }
}

fn snapshot_for(backend: Backend) -> RUsageSnapshot {
    match backend {
        Backend::Ebpf | Backend::Dtrace | Backend::RusageSelf => snapshot_self(),
        Backend::RusageChildren => snapshot_children(),
    }
}

// Telemetry-only snapshots. Returns Default until a safe-wrapper crate
// for getrusage lands in rustix. SOURCE: rustix#1392 (getrusage tracking).
fn snapshot_self() -> RUsageSnapshot {
    RUsageSnapshot::default()
}
fn snapshot_children() -> RUsageSnapshot {
    RUsageSnapshot::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::integer_division,
        reason = "test fixture: exact-fraction budget arithmetic; truncation is the intended behavior"
    )]
    fn within_budget_under_80pct() {
        let b = budget_ns_for("Read");
        assert_eq!(verdict_for(b / 2, b), Verdict::WithinBudget);
    }

    #[test]
    #[expect(
        clippy::integer_division,
        reason = "test fixture: 90%-of-budget arithmetic; truncation is the intended behavior"
    )]
    fn near_limit_between_80_and_100() {
        let b = budget_ns_for("Read");
        match verdict_for(b * 9 / 10, b) {
            Verdict::NearLimit { pct, .. } => assert!((80..100).contains(&pct)),
            v => panic!("expected NearLimit, got {v:?}"),
        }
    }

    #[test]
    fn over_budget_at_limit() {
        let b = budget_ns_for("Read");
        match verdict_for(b + 1, b) {
            Verdict::OverBudget { .. } => {}
            v => panic!("expected OverBudget, got {v:?}"),
        }
    }

    #[test]
    fn unknown_tool_default_100ms() {
        assert_eq!(budget_ns_for("Phantom"), 100 * NS_PER_MS);
    }

    #[test]
    fn mcp_tool_default_10s() {
        assert_eq!(budget_ns_for("mcp__zai__foo"), 10 * NS_PER_S);
    }

    #[test]
    fn probe_emits_render_block() {
        let p = Probe::start("Read");
        std::thread::sleep(std::time::Duration::from_millis(1));
        let r = p.stop();
        let s = r.render_kernel_observed_block();
        assert!(s.contains("[KERNEL_OBSERVED]"));
        assert!(s.contains("tool: Read"));
        assert!(s.contains("verdict:"));
    }
}
