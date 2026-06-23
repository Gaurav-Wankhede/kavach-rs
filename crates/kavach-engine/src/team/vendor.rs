//! Cross-vendor headless dispatch — the Fugu/TRINITY-inspired orchestration leg.
//!
//! Where the intra-process [`super::Spawner`] fans native CC Team agents over a
//! DAG, [`VendorBackend`] wraps the cross-PROCESS headless invocation of a whole
//! agent harness (Claude Code, Codex, OpenCode, Gemini) as a swappable pool
//! member. Roles follow TRINITY (Thinker/Worker/Verifier); the Verifier role is
//! kavach's existing three-witness gates, not a vendor call.
//!
//! SOURCE: decision.fugu-orchestration-layer · https://sakana.ai/trinity/
#[path = "vendor/argv.rs"]
mod argv;

use argv::{cc_argv, codex_argv, gemini_argv, opencode_argv};
use crate::error::EngineError;
use std::process::Command;

/// TRINITY role a dispatched agent plays this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AgentRole {
    /// Decomposes / plans (route to a high-capability backend).
    Thinker,
    /// Executes one step (route to a cost-efficient backend).
    Worker,
    /// Checks output — kavach gates, not a vendor call (kept for routing parity).
    Verifier,
}

/// One unit of cross-vendor work.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct VendorRequest {
    /// Role this dispatch plays.
    pub role: AgentRole,
    /// The task prompt handed to the headless agent.
    pub prompt: String,
    /// Project slug for context scoping.
    pub project: String,
    /// Hard turn cap (fail-closed bound on agentic loops).
    pub max_turns: u32,
}

impl VendorRequest {
    /// Build a request (the struct is `#[non_exhaustive]`, so this is the
    /// cross-crate constructor).
    #[must_use]
    pub const fn new(role: AgentRole, prompt: String, project: String, max_turns: u32) -> Self {
        Self {
            role,
            prompt,
            project,
            max_turns,
        }
    }
}

/// Captured result of a headless run.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct VendorOutput {
    /// Which backend produced it.
    pub vendor: String,
    /// Combined stdout.
    pub stdout: String,
    /// Process exit code (0 = success).
    pub exit_code: i32,
}

/// A swappable pool member: turns a [`VendorRequest`] into a running agent.
pub trait VendorBackend {
    /// Stable vendor id (`cc` | `codex` | `opencode` | `gemini`).
    fn id(&self) -> &str;

    /// Dispatch one request headlessly. Fail-closed: a non-zero exit is an
    /// `Err`, never a silent `Ok`.
    ///
    /// # Errors
    /// [`EngineError::Session`] on spawn failure or non-zero exit.
    fn dispatch(&self, req: &VendorRequest) -> Result<VendorOutput, EngineError>;
}

/// A [`VendorBackend`] that shells out via `std::process::Command`. The argv
/// builder is a fn-pointer so a vendor's contract is one line to swap.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct CommandBackend {
    /// Stable vendor id.
    pub vendor: &'static str,
    /// Pure argv builder for this vendor's headless contract.
    pub argv: fn(&VendorRequest) -> Vec<String>,
}

impl CommandBackend {
    /// Claude Code headless backend.
    #[must_use]
    pub fn cc() -> Self {
        Self { vendor: "cc", argv: cc_argv }
    }
    /// OpenAI Codex headless backend.
    #[must_use]
    pub fn codex() -> Self {
        Self { vendor: "codex", argv: codex_argv }
    }
    /// OpenCode headless backend.
    #[must_use]
    pub fn opencode() -> Self {
        Self { vendor: "opencode", argv: opencode_argv }
    }
    /// Gemini CLI headless backend.
    #[must_use]
    pub fn gemini() -> Self {
        Self { vendor: "gemini", argv: gemini_argv }
    }
    /// Backend with an explicit id + argv builder (the cross-crate constructor;
    /// the struct is `#[non_exhaustive]`). Used to register a custom vendor.
    #[must_use]
    pub fn raw(vendor: &'static str, argv: fn(&VendorRequest) -> Vec<String>) -> Self {
        Self { vendor, argv }
    }
}

impl VendorBackend for CommandBackend {
    fn id(&self) -> &str {
        self.vendor
    }

    fn dispatch(&self, req: &VendorRequest) -> Result<VendorOutput, EngineError> {
        let argv = (self.argv)(req);
        let (program, rest) = argv
            .split_first()
            .ok_or_else(|| EngineError::Session("empty vendor argv".into()))?;
        let out = Command::new(program)
            .args(rest)
            .output()
            .map_err(|e| EngineError::Session(format!("{} spawn failed: {e}", self.vendor)))?;
        let code = out.status.code().unwrap_or(-1);
        if code != 0 {
            return Err(EngineError::Session(format!(
                "{} exited {code}: {}",
                self.vendor,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(VendorOutput {
            vendor: self.vendor.to_owned(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            exit_code: code,
        })
    }
}

#[cfg(test)]
#[path = "vendor/argv_test.rs"]
mod argv_test;
