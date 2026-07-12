//! Built-in *external-session overflow* seam.
//!
//! When the in-process multi-agent pool is saturated
//! ([`CodexErr::AgentLimitReached`](codex_protocol::error::CodexErr::AgentLimitReached)),
//! codex can hand a leftover delegation slice to an **external `codex exec`
//! process** instead of dropping or re-queuing it. That external process is a
//! separate OS process, so it never passes through the in-process
//! `AgentExecutionLimiter` and therefore adds *real* capacity rather than merely
//! renaming the work.
//!
//! This module owns only the *port* (a dependency-inversion seam): a plain-data
//! request, an outcome, a narrow `Send + Sync` dispatcher trait, and a
//! process-global registry. The concrete implementation — which writes the
//! prompt file, invokes the canonical `start-codex-workers.ps1` launcher,
//! bootstraps a launcher stub when one is missing, and enforces the
//! gate/bound/recursion guard — lives in the fork-owned `codex-agent-overflow`
//! crate and is registered once at the composition root (e.g. interactive TUI
//! startup). `codex-core` therefore never depends on the implementing crate: the
//! dependency edge points `codex-core <- codex-agent-overflow`, never the
//! reverse (honoring the "resist adding to codex-core" boundary rule).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

/// Plain-data description of a leftover delegation slice that could not be
/// executed in-process because the agent thread pool is saturated.
#[derive(Debug, Clone)]
pub struct AgentOverflowRequest {
    /// Repository root the external `codex exec` session should run in
    /// (`--cd <repo_root>`); also the root under which the launcher and the
    /// generated `.codex/workflow/agents/<name>.prompt.md` file live.
    pub repo_root: PathBuf,
    /// Short, human-meaningful label for the slice; the implementation sanitizes
    /// it into a unique worker name.
    pub label: String,
    /// Full task/instructions handed to the external worker.
    pub prompt: String,
}

/// Outcome of an overflow attempt, used for logging and for the caller's
/// fallback decision.
#[derive(Debug)]
pub enum AgentOverflowOutcome {
    /// An external session was launched. `handoff_path` is where the coordinator
    /// lands the external worker's result once it finishes.
    Spawned { handoff_path: PathBuf },
    /// Overflow was intentionally not attempted (disabled by config, per-process
    /// bound exhausted, or already running inside an external session). The
    /// caller should fall back to its previous behavior.
    Declined,
    /// An overflow was attempted but failed (launcher missing and bootstrap
    /// failed, or the process could not be spawned). The caller should fall back
    /// to its previous behavior.
    Failed,
}

/// Narrow port implemented by the fork-owned `codex-agent-overflow` crate and
/// registered once at the composition root.
pub trait AgentOverflowDispatcher: Send + Sync {
    /// Attempt to hand `request` off to an external `codex exec` session,
    /// fire-and-forget. Implementations MUST NOT block the agent loop waiting on
    /// the spawned child process.
    fn dispatch_overflow(&self, request: AgentOverflowRequest) -> AgentOverflowOutcome;
}

static OVERFLOW_DISPATCHER: OnceLock<Arc<dyn AgentOverflowDispatcher>> = OnceLock::new();

/// Register the process-wide overflow dispatcher. Intended to be called once at
/// the composition root (e.g. interactive TUI startup) when the
/// `auto_external_overflow` config gate is enabled. Subsequent calls are no-ops,
/// so the first registration wins.
pub fn register_overflow_dispatcher(dispatcher: Arc<dyn AgentOverflowDispatcher>) {
    let _ = OVERFLOW_DISPATCHER.set(dispatcher);
}

/// Whether an overflow dispatcher has been registered for this process.
pub fn overflow_dispatcher_registered() -> bool {
    OVERFLOW_DISPATCHER.get().is_some()
}

/// Attempt to overflow `request` to an external session. Returns `None` when no
/// dispatcher is registered (feature disabled or not wired), in which case the
/// caller keeps its previous behavior.
pub fn try_dispatch_overflow(request: AgentOverflowRequest) -> Option<AgentOverflowOutcome> {
    OVERFLOW_DISPATCHER
        .get()
        .map(|dispatcher| dispatcher.dispatch_overflow(request))
}
