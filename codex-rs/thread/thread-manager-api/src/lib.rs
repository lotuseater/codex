//! Thread manager command boundary.
//!
//! This crate owns the command/result DTOs for thread orchestration without
//! depending on concrete sessions, stores, app-server protocol, or `codex-core`.

use std::future::Future;
use std::pin::Pin;

use codex_config_types::ContextBudgetMode;
use codex_protocol::ThreadId;
use codex_protocol::config_types::ApprovalsReviewer;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::MultiAgentMode;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadSource;
use codex_thread_api::ThreadIdentity;
use codex_utils_absolute_path::AbsolutePathBuf;

/// Boxed future used by object-safe thread manager ports.
pub type ThreadManagerFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Snapshot of effective thread settings after manager validation.
#[derive(Clone, Debug)]
pub struct ThreadConfigSnapshot {
    pub model: String,
    pub model_provider_id: String,
    /// History source thread when this thread was forked, for analytics lineage.
    pub forked_from_thread_id: Option<ThreadId>,
    pub parent_thread_id: Option<ThreadId>,
    pub service_tier: Option<String>,
    pub approval_policy: AskForApproval,
    pub approvals_reviewer: ApprovalsReviewer,
    pub permission_profile: PermissionProfile,
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub cwd: AbsolutePathBuf,
    pub workspace_roots: Vec<AbsolutePathBuf>,
    pub profile_workspace_roots: Vec<AbsolutePathBuf>,
    pub ephemeral: bool,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub reasoning_summary: Option<ReasoningSummary>,
    pub personality: Option<Personality>,
    pub collaboration_mode: CollaborationMode,
    pub multi_agent_mode: Option<MultiAgentMode>,
    pub session_source: SessionSource,
    pub thread_source: Option<ThreadSource>,
}

impl ThreadConfigSnapshot {
    pub fn sandbox_policy(&self) -> SandboxPolicy {
        codex_sandboxing::compatibility_sandbox_policy_for_permission_profile(
            &self.permission_profile,
            self.cwd.as_path(),
        )
    }
}

/// Thread settings overrides that app-server validates before starting a turn.
#[derive(Clone, Default)]
pub struct CodexThreadSettingsOverrides {
    pub cwd: Option<AbsolutePathBuf>,
    pub workspace_roots: Option<Vec<AbsolutePathBuf>>,
    pub profile_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    pub approval_policy: Option<AskForApproval>,
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub sandbox_policy: Option<SandboxPolicy>,
    pub permission_profile: Option<PermissionProfile>,
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub windows_sandbox_level: Option<WindowsSandboxLevel>,
    pub model: Option<String>,
    pub effort: Option<Option<ReasoningEffort>>,
    pub summary: Option<ReasoningSummary>,
    pub service_tier: Option<Option<String>>,
    pub context_budget_mode: Option<ContextBudgetMode>,
    pub collaboration_mode: Option<CollaborationMode>,
    pub multi_agent_mode: Option<MultiAgentMode>,
    pub personality: Option<Personality>,
}

pub type CodexThreadTurnContextOverrides = CodexThreadSettingsOverrides;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartThreadCommand {
    pub identity: ThreadIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeThreadCommand {
    pub identity: ThreadIdentity,
}

#[derive(Debug, thiserror::Error)]
pub enum ThreadManagerError {
    #[error("thread manager operation is unsupported: {operation}")]
    Unsupported { operation: &'static str },
}

pub type ThreadManagerResult<T> = Result<T, ThreadManagerError>;

/// Port for starting and resuming threads.
pub trait ThreadManagerPort: Send + Sync {
    fn start_thread<'a>(
        &'a self,
        command: StartThreadCommand,
    ) -> ThreadManagerFuture<'a, ThreadManagerResult<ThreadIdentity>>;

    fn resume_thread<'a>(
        &'a self,
        command: ResumeThreadCommand,
    ) -> ThreadManagerFuture<'a, ThreadManagerResult<ThreadIdentity>>;
}
