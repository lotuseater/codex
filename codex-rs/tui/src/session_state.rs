//! Canonical TUI session state shared across app-server routing, chat display, and status UI.
//!
//! The app-server API is the boundary for session lifecycle events. Once those responses enter
//! TUI, this module holds the small internal state shape used by app orchestration and widgets.

use std::path::PathBuf;

use codex_app_server_protocol::AskForApproval;
use codex_protocol::ThreadId;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::Personality;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::PermissionProfile;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionNetworkProxyRuntime {
    pub(crate) http_addr: String,
    pub(crate) socks_addr: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MessageHistoryMetadata {
    pub(crate) log_id: u64,
    pub(crate) entry_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThreadSessionState {
    pub(crate) thread_id: ThreadId,
    pub(crate) forked_from_id: Option<ThreadId>,
    pub(crate) fork_parent_title: Option<String>,
    pub(crate) thread_name: Option<String>,
    pub(crate) model: String,
    pub(crate) model_provider_id: String,
    pub(crate) service_tier: Option<String>,
    pub(crate) approval_policy: AskForApproval,
    pub(crate) approvals_reviewer: codex_protocol::config_types::ApprovalsReviewer,
    /// Permission snapshot used by TUI display surfaces. Legacy app-server
    /// responses are converted to a profile at ingestion time using the
    /// response cwd so cached sessions do not reinterpret cwd-bound grants.
    /// Turn requests must not treat this snapshot as a local permission
    /// override unless the user explicitly changed permissions in the TUI.
    pub(crate) permission_profile: PermissionProfile,
    /// Named or implicit built-in profile that produced `permission_profile`,
    /// when the server knows it.
    pub(crate) active_permission_profile: Option<ActivePermissionProfile>,
    pub(crate) cwd: AbsolutePathBuf,
    pub(crate) runtime_workspace_roots: Vec<AbsolutePathBuf>,
    pub(crate) instruction_source_paths: Vec<PathUri>,
    pub(crate) reasoning_effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    pub(crate) collaboration_mode: Option<Box<CollaborationMode>>,
    pub(crate) personality: Option<Personality>,
    pub(crate) message_history: Option<MessageHistoryMetadata>,
    pub(crate) network_proxy: Option<SessionNetworkProxyRuntime>,
    pub(crate) rollout_path: Option<PathBuf>,
}

impl ThreadSessionState {
    pub(crate) fn set_cwd_retargeting_implicit_runtime_workspace_root(
        &mut self,
        cwd: AbsolutePathBuf,
    ) {
        let previous_cwd = std::mem::replace(&mut self.cwd, cwd.clone());
        if !self.runtime_workspace_roots.contains(&previous_cwd) {
            return;
        }

        let previous_roots = std::mem::take(&mut self.runtime_workspace_roots);
        self.runtime_workspace_roots.push(cwd);
        for root in previous_roots {
            if root != previous_cwd && !self.runtime_workspace_roots.contains(&root) {
                self.runtime_workspace_roots.push(root);
            }
        }
    }

    /// Project this TUI-side session state onto the render crate's
    /// [`codex_tui_render::session_state::ThreadSessionState`] used by history
    /// cells such as `new_session_info`.
    ///
    /// The two structs carry the same field set; only `approval_policy`
    /// (app-server vs core `AskForApproval`) and `instruction_source_paths`
    /// (`PathUri` vs `AbsolutePathBuf`) differ in type. `instruction_source_paths`
    /// is not read by the render layer, but is converted faithfully (non-native
    /// URIs are dropped) rather than fabricated.
    pub(crate) fn to_render(&self) -> codex_tui_render::session_state::ThreadSessionState {
        codex_tui_render::session_state::ThreadSessionState {
            thread_id: self.thread_id,
            forked_from_id: self.forked_from_id,
            fork_parent_title: self.fork_parent_title.clone(),
            thread_name: self.thread_name.clone(),
            model: self.model.clone(),
            model_provider_id: self.model_provider_id.clone(),
            service_tier: self.service_tier.clone(),
            approval_policy: self.approval_policy.to_core(),
            approvals_reviewer: self.approvals_reviewer,
            permission_profile: self.permission_profile.clone(),
            active_permission_profile: self.active_permission_profile.clone(),
            cwd: self.cwd.clone(),
            runtime_workspace_roots: self.runtime_workspace_roots.clone(),
            instruction_source_paths: self
                .instruction_source_paths
                .iter()
                .filter_map(|path| path.to_abs_path().ok())
                .collect(),
            reasoning_effort: self.reasoning_effort.clone(),
            collaboration_mode: self.collaboration_mode.clone(),
            personality: self.personality,
            message_history: self.message_history.map(|history| {
                codex_tui_render::session_state::MessageHistoryMetadata {
                    log_id: history.log_id,
                    entry_count: history.entry_count,
                }
            }),
            network_proxy: self.network_proxy.as_ref().map(|proxy| {
                codex_tui_render::session_state::SessionNetworkProxyRuntime {
                    http_addr: proxy.http_addr.clone(),
                    socks_addr: proxy.socks_addr.clone(),
                }
            }),
            rollout_path: self.rollout_path.clone(),
        }
    }
}
