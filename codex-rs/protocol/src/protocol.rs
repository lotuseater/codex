//! Defines the protocol for a Codex session between a client and an agent.
//!
//! Uses a SQ (Submission Queue) / EQ (Event Queue) pattern to asynchronously communicate
//! between user and agent.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use crate::AgentPath;
use crate::ThreadId;
use crate::config_types::ApprovalsReviewer;
use crate::config_types::CollaborationMode;
use crate::config_types::ModeKind;
use crate::config_types::MultiAgentMode;
use crate::config_types::Personality;
use crate::config_types::ReasoningSummary as ReasoningSummaryConfig;
use crate::config_types::WindowsSandboxLevel;
use crate::models::ActivePermissionProfile;
use crate::models::AgentMessageInputContent;
use crate::models::ContentItem;
use crate::models::MessagePhase;
use crate::models::PermissionProfile;
use crate::models::ResponseInputItem;
use crate::models::ResponseItem;
use crate::models::ResponseItemMetadata;
use crate::models::SandboxEnforcement;
use crate::models::WebSearchAction;
use crate::num_format::format_with_separators;
use crate::openai_models::ReasoningEffort as ReasoningEffortConfig;
use crate::user_input::UserInput;
use codex_config_types::ContextBudgetMode;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use ts_rs::TS;

pub use crate::approvals::ApplyPatchApprovalRequestEvent;
pub use crate::approvals::ElicitationAction;
pub use crate::approvals::ExecApprovalRequestEvent;
pub use crate::approvals::ExecPolicyAmendment;
pub use crate::approvals::GuardianAssessmentAction;
pub use crate::approvals::GuardianAssessmentDecisionSource;
pub use crate::approvals::GuardianAssessmentEvent;
pub use crate::approvals::GuardianAssessmentOutcome;
pub use crate::approvals::GuardianAssessmentStatus;
pub use crate::approvals::GuardianCommandSource;
pub use crate::approvals::GuardianRiskLevel;
pub use crate::approvals::GuardianUserAuthorization;
pub use crate::approvals::NetworkApprovalContext;
pub use crate::approvals::NetworkApprovalProtocol;
pub use crate::approvals::NetworkPolicyAmendment;
pub use crate::approvals::NetworkPolicyRuleAction;
mod agent_reasoning;
mod collaboration;
mod decision_and_changes;
mod errors_and_warnings;
mod event_msg;
mod exec_command;
mod hooks;
mod mcp_tool;
mod op;
mod patch_and_plan;
mod realtime_conversation;
mod realtime_session;
mod review;
mod rollout;
mod session_config;
mod session_source;
mod skills;
mod thread_goal;
mod token_usage;
mod tool_call_events;
mod turn_items;
mod turn_lifecycle;

pub use decision_and_changes::*;
pub use hooks::*;
pub use realtime_session::*;
pub use rollout::*;
pub use session_config::*;
pub use session_source::*;
pub use skills::*;
pub use thread_goal::*;
pub use token_usage::*;
pub use tool_call_events::*;
pub use turn_items::*;
pub use turn_lifecycle::*;

pub use agent_reasoning::AgentReasoningEvent;
pub use agent_reasoning::AgentReasoningRawContentEvent;
pub use agent_reasoning::AgentReasoningSectionBreakEvent;
pub use collaboration::CollabAgentInteractionBeginEvent;
pub use collaboration::CollabAgentInteractionEndEvent;
pub use collaboration::CollabAgentRef;
pub use collaboration::CollabAgentSpawnBeginEvent;
pub use collaboration::CollabAgentSpawnEndEvent;
pub use collaboration::CollabAgentStatusEntry;
pub use collaboration::CollabCloseBeginEvent;
pub use collaboration::CollabCloseEndEvent;
pub use collaboration::CollabCompactBeginEvent;
pub use collaboration::CollabCompactEndEvent;
pub use collaboration::CollabRestartBeginEvent;
pub use collaboration::CollabRestartEndEvent;
pub use collaboration::CollabResumeBeginEvent;
pub use collaboration::CollabResumeEndEvent;
pub use collaboration::CollabWaitingBeginEvent;
pub use collaboration::CollabWaitingEndEvent;
pub use errors_and_warnings::ErrorEvent;
pub use errors_and_warnings::StreamErrorEvent;
pub use errors_and_warnings::StreamInfoEvent;
pub use errors_and_warnings::WarningEvent;
pub use event_msg::EventMsg;
pub use exec_command::ExecCommandBeginEvent;
pub use exec_command::ExecCommandEndEvent;
pub use exec_command::ExecCommandOutputDeltaEvent;
pub use exec_command::ExecCommandSource;
pub use exec_command::ExecCommandStatus;
pub use exec_command::ExecOutputStream;
pub use exec_command::TerminalInteractionEvent;
pub use exec_command::ViewImageToolCallEvent;
pub use mcp_tool::McpAuthStatus;
pub use mcp_tool::McpStartupCompleteEvent;
pub use mcp_tool::McpStartupFailure;
pub use mcp_tool::McpStartupStatus;
pub use mcp_tool::McpStartupUpdateEvent;
pub use op::Op;
pub use patch_and_plan::PatchApplyBeginEvent;
pub use patch_and_plan::PatchApplyEndEvent;
pub use patch_and_plan::PatchApplyStatus;
pub use patch_and_plan::PatchApplyUpdatedEvent;
pub use realtime_conversation::RealtimeConversationClosedEvent;
pub use realtime_conversation::RealtimeConversationListVoicesResponseEvent;
pub use realtime_conversation::RealtimeConversationRealtimeEvent;
pub use realtime_conversation::RealtimeConversationSdpEvent;
pub use realtime_conversation::RealtimeConversationStartedEvent;
pub use review::ReviewCodeLocation;
pub use review::ReviewDelivery;
pub use review::ReviewFinding;
pub use review::ReviewLineRange;
pub use review::ReviewOutputEvent;
pub use review::ReviewRequest;
pub use review::ReviewTarget;

pub use crate::permissions::FileSystemAccessMode;
pub use crate::permissions::FileSystemPath;
pub use crate::permissions::FileSystemSandboxEntry;
pub use crate::permissions::FileSystemSandboxKind;
pub use crate::permissions::FileSystemSandboxPolicy;
pub use crate::permissions::FileSystemSpecialPath;
pub use crate::permissions::NetworkSandboxPolicy;
pub use crate::request_permissions::RequestPermissionsArgs;
pub use crate::request_user_input::RequestUserInputEvent;

/// Open/close tags for special user-input blocks. Used across crates to avoid
/// duplicated hardcoded strings.
pub const USER_INSTRUCTIONS_OPEN_TAG: &str = "<user_instructions>";
pub const USER_INSTRUCTIONS_CLOSE_TAG: &str = "</user_instructions>";
pub const ENVIRONMENT_CONTEXT_OPEN_TAG: &str = "<environment_context>";
pub const ENVIRONMENT_CONTEXT_CLOSE_TAG: &str = "</environment_context>";
pub const APPS_INSTRUCTIONS_OPEN_TAG: &str = "<apps_instructions>";
pub const APPS_INSTRUCTIONS_CLOSE_TAG: &str = "</apps_instructions>";
pub const SKILLS_INSTRUCTIONS_OPEN_TAG: &str = "<skills_instructions>";
pub const SKILLS_INSTRUCTIONS_CLOSE_TAG: &str = "</skills_instructions>";
pub const PLUGINS_INSTRUCTIONS_OPEN_TAG: &str = "<plugins_instructions>";
pub const PLUGINS_INSTRUCTIONS_CLOSE_TAG: &str = "</plugins_instructions>";
pub const COLLABORATION_MODE_OPEN_TAG: &str = "<collaboration_mode>";
pub const COLLABORATION_MODE_CLOSE_TAG: &str = "</collaboration_mode>";
pub const MULTI_AGENT_MODE_OPEN_TAG: &str = "<multi_agent_mode>";
pub const MULTI_AGENT_MODE_CLOSE_TAG: &str = "</multi_agent_mode>";
pub const REALTIME_CONVERSATION_OPEN_TAG: &str = "<realtime_conversation>";
pub const REALTIME_CONVERSATION_CLOSE_TAG: &str = "</realtime_conversation>";
pub const USER_MESSAGE_BEGIN: &str = "## My request for Codex:";

pub use codex_git_types::GitSha;

pub use codex_config_types::RealtimeVoice;
pub use codex_config_types::RealtimeVoicesList;

// fork-local: `TurnEnvironmentSelection` (singular) lives in the `realtime_session`
// submodule and is re-exported via `pub use realtime_session::*`. The plural container
// `TurnEnvironmentSelections` is an upstream addition the fork's core code depends on, so
// it is kept here. Upstream's inline `GitSha` is dropped — the fork re-exports it from
// `codex_git_types` above.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct TurnEnvironmentSelections {
    pub legacy_fallback_cwd: AbsolutePathBuf,
    pub environments: Vec<TurnEnvironmentSelection>,
}

impl TurnEnvironmentSelections {
    pub fn new(
        legacy_fallback_cwd: AbsolutePathBuf,
        environments: Vec<TurnEnvironmentSelection>,
    ) -> Self {
        Self {
            legacy_fallback_cwd,
            environments,
        }
    }
}

/// Submission Queue Entry - requests from user
#[derive(Debug, Clone)]
pub struct Submission {
    /// Unique id for this Submission to correlate with Events
    pub id: String,
    /// Payload
    pub op: Op,
    /// Client-provided id for the user message represented by `Op::UserInput`.
    pub client_user_message_id: Option<String>,
    /// Optional W3C trace carrier propagated across async submission handoffs.
    pub trace: Option<W3cTraceContext>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ConversationTextRole {
    #[default]
    User,
    Developer,
    Assistant,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema, TS)]
pub struct ConversationSpeechParams {
    pub text: String,
}

/// Persistent thread-settings overrides that can be applied before user input or
/// on their own.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct ThreadSettingsOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<AbsolutePathBuf>,

    /// Updated fallback `cwd` and environments supplied together as a complete pair.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environments: Option<TurnEnvironmentSelections>,

    /// Updated runtime workspace roots used to materialize symbolic
    /// `:workspace_roots` filesystem permissions.
    pub workspace_roots: Option<Vec<AbsolutePathBuf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<AskForApproval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_policy: Option<SandboxPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_profile: Option<PermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_permission_profile: Option<ActivePermissionProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows_sandbox_level: Option<WindowsSandboxLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<Option<ReasoningEffortConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ReasoningSummaryConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collaboration_mode: Option<CollaborationMode>,

    /// Updated multi-agent mode for this turn and subsequent turns.
    pub multi_agent_mode: Option<MultiAgentMode>,

    /// Updated personality preference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality: Option<Personality>,
}

/// Source classification for client-supplied context.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalContextKind {
    Untrusted,
    Application,
}

/// Client-supplied context keyed by an opaque source identifier.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
pub struct AdditionalContextEntry {
    pub value: String,
    pub kind: AdditionalContextKind,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThreadMemoryMode {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, TS)]
pub struct InterAgentCommunication {
    pub author: AgentPath,
    pub recipient: AgentPath,
    #[serde(default)]
    pub other_recipients: Vec<AgentPath>,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub encrypted_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub metadata: Option<ResponseItemMetadata>,
    pub trigger_turn: bool,
}

impl InterAgentCommunication {
    pub fn new(
        author: AgentPath,
        recipient: AgentPath,
        other_recipients: Vec<AgentPath>,
        content: String,
        trigger_turn: bool,
    ) -> Self {
        Self {
            author,
            recipient,
            other_recipients,
            content,
            encrypted_content: None,
            metadata: None,
            trigger_turn,
        }
    }

    pub fn new_encrypted(
        author: AgentPath,
        recipient: AgentPath,
        other_recipients: Vec<AgentPath>,
        encrypted_content: String,
        trigger_turn: bool,
    ) -> Self {
        Self {
            author,
            recipient,
            other_recipients,
            content: String::new(),
            encrypted_content: Some(encrypted_content),
            metadata: None,
            trigger_turn,
        }
    }

    pub fn to_response_input_item(&self) -> ResponseInputItem {
        ResponseInputItem::Message {
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: serde_json::to_string(self).unwrap_or_default(),
            }],
            phase: Some(MessagePhase::Commentary),
        }
    }

    pub fn to_model_input_item(&self) -> ResponseItem {
        let content = match &self.encrypted_content {
            Some(encrypted_content) => {
                let message_type = if self.trigger_turn {
                    "NEW_TASK"
                } else {
                    "MESSAGE"
                };
                vec![
                    AgentMessageInputContent::InputText {
                        text: format!(
                            "Message Type: {message_type}\nTask name: {}\nSender: {}\nPayload:\n",
                            self.recipient, self.author
                        ),
                    },
                    AgentMessageInputContent::EncryptedContent {
                        encrypted_content: encrypted_content.clone(),
                    },
                ]
            }
            None => vec![AgentMessageInputContent::InputText {
                text: self.content.clone(),
            }],
        };
        ResponseItem::AgentMessage {
            id: None,
            author: self.author.to_string(),
            recipient: self.recipient.to_string(),
            content,
            metadata: self.metadata.clone(),
        }
    }

    pub fn is_message_content(content: &[ContentItem]) -> bool {
        Self::from_message_content(content).is_some()
    }

    pub fn from_message_content(content: &[ContentItem]) -> Option<Self> {
        match content {
            [ContentItem::InputText { text }] | [ContentItem::OutputText { text }] => {
                serde_json::from_str(text).ok()
            }
            _ => None,
        }
    }
}

pub use codex_permission_types::AskForApproval;
pub use codex_permission_types::GranularApprovalConfig;
pub use codex_permission_types::NetworkAccess;
pub use codex_permission_types::SandboxPolicy;
pub use codex_permission_types::WritableRoot;

/// Event Queue Entry - events from agent
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Event {
    /// Submission `id` that this event is correlated with.
    pub id: String,
    /// Payload
    pub msg: EventMsg,
}

pub use codex_config_types::HookEventName;

pub use codex_config_types::RealtimeConversationVersion;

impl From<SubAgentActivityEvent> for EventMsg {
    fn from(event: SubAgentActivityEvent) -> Self {
        EventMsg::SubAgentActivity(event)
    }
}

/// Agent lifecycle status, derived from emitted events.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS, Default)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is waiting for initialization.
    #[default]
    PendingInit,
    /// Agent is currently running.
    Running,
    /// Agent's current turn was interrupted and it may receive more input.
    Interrupted,
    /// Agent is done. Contains the final assistant message.
    Completed(Option<String>),
    /// Agent encountered an error.
    Errored(String),
    /// Agent has been shutdown.
    Shutdown,
    /// Agent is not found.
    NotFound,
}

/// Turn kinds that reject same-turn steering.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum NonSteerableTurnKind {
    Review,
    Compact,
}

/// Codex errors that we expose to clients.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum CodexErrorInfo {
    ContextWindowExceeded,
    UsageLimitExceeded,
    ServerOverloaded,
    CyberPolicy,
    HttpConnectionFailed {
        http_status_code: Option<u16>,
    },
    /// Failed to connect to the response SSE stream.
    ResponseStreamConnectionFailed {
        http_status_code: Option<u16>,
    },
    InternalServerError,
    Unauthorized,
    BadRequest,
    SandboxError,
    /// The response SSE stream disconnected in the middle of a turnbefore completion.
    ResponseStreamDisconnected {
        http_status_code: Option<u16>,
    },
    /// Reached the retry limit for responses.
    ResponseTooManyFailedAttempts {
        http_status_code: Option<u16>,
    },
    /// Returned when `turn/start` or `turn/steer` is submitted while the current active turn
    /// cannot accept same-turn steering, for example `/review` or manual `/compact`.
    ActiveTurnNotSteerable {
        turn_kind: NonSteerableTurnKind,
    },
    ThreadRollbackFailed,
    Other,
}

impl CodexErrorInfo {
    /// Whether this error should mark the current turn as failed when replaying history.
    pub fn affects_turn_status(&self) -> bool {
        match self {
            Self::ThreadRollbackFailed | Self::ActiveTurnNotSteerable { .. } => false,
            Self::ContextWindowExceeded
            | Self::UsageLimitExceeded
            | Self::ServerOverloaded
            | Self::CyberPolicy
            | Self::HttpConnectionFailed { .. }
            | Self::ResponseStreamConnectionFailed { .. }
            | Self::InternalServerError
            | Self::Unauthorized
            | Self::BadRequest
            | Self::SandboxError
            | Self::ResponseStreamDisconnected { .. }
            | Self::ResponseTooManyFailedAttempts { .. }
            | Self::Other => true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct ThreadSettingsSnapshot {
    pub model: String,
    pub model_provider_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    pub approval_policy: AskForApproval,
    pub approvals_reviewer: ApprovalsReviewer,
    pub permission_profile: PermissionProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub active_permission_profile: Option<ActivePermissionProfile>,
    pub cwd: AbsolutePathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffortConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_summary: Option<ReasoningSummaryConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality: Option<Personality>,
    pub collaboration_mode: CollaborationMode,
    #[serde(default)]
    pub multi_agent_mode: Option<MultiAgentMode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, TS)]
pub struct ThreadSettingsAppliedEvent {
    pub thread_settings: ThreadSettingsSnapshot,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema, TS)]
pub struct RateLimitSnapshot {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
    pub credits: Option<CreditsSnapshot>,
    pub individual_limit: Option<SpendControlLimitSnapshot>,
    pub plan_type: Option<crate::account::PlanType>,
    pub rate_limit_reached_type: Option<RateLimitReachedType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RateLimitReachedType {
    RateLimitReached,
    WorkspaceOwnerCreditsDepleted,
    WorkspaceMemberCreditsDepleted,
    WorkspaceOwnerUsageLimitReached,
    WorkspaceMemberUsageLimitReached,
}

impl FromStr for RateLimitReachedType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "rate_limit_reached" => Ok(Self::RateLimitReached),
            "workspace_owner_credits_depleted" => Ok(Self::WorkspaceOwnerCreditsDepleted),
            "workspace_member_credits_depleted" => Ok(Self::WorkspaceMemberCreditsDepleted),
            "workspace_owner_usage_limit_reached" => Ok(Self::WorkspaceOwnerUsageLimitReached),
            "workspace_member_usage_limit_reached" => Ok(Self::WorkspaceMemberUsageLimitReached),
            other => Err(format!("unknown rate limit reached type: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema, TS)]
pub struct RateLimitWindow {
    /// Percentage (0-100) of the window that has been consumed.
    pub used_percent: f64,
    /// Rolling window duration, in minutes.
    #[ts(type = "number | null")]
    pub window_minutes: Option<i64>,
    /// Unix timestamp (seconds since epoch) when the window resets.
    #[ts(type = "number | null")]
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema, TS)]
pub struct CreditsSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

// fork-local: the bulk of the protocol types upstream defines inline here have
// been extracted into the seam modules under `protocol/` (token_usage.rs,
// session_config.rs, session_source.rs, rollout.rs, review.rs, exec_command.rs,
// skills.rs, thread_goal.rs, collaboration.rs, etc.) and are re-exported from
// `mod.rs`. We drop the inline duplicates to avoid double definitions, but keep
// the two types upstream newly added that the fork's seam files do not yet own
// and that kept code in this module still references.

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema, TS)]
pub struct SpendControlLimitSnapshot {
    pub limit: String,
    pub used: String,
    pub remaining_percent: i32,
    pub resets_at: i64,
}

// Includes prompts, tools and space to call compact.
const BASELINE_TOKENS: i64 = 12000;

fn session_cwd_from_items(items: &[RolloutItem]) -> Option<PathBuf> {
    items.iter().find_map(|item| match item {
        RolloutItem::SessionMeta(meta_line) => Some(meta_line.meta.cwd.clone()),
        _ => None,
    })
}

fn multi_agent_version_from_items(
    items: &[RolloutItem],
    thread_id: Option<ThreadId>,
) -> Option<MultiAgentVersion> {
    let session_meta_version = items.iter().rev().find_map(|item| match item {
        RolloutItem::SessionMeta(meta_line)
            if thread_id.is_none_or(|thread_id| meta_line.meta.id == thread_id) =>
        {
            meta_line.meta.multi_agent_version
        }
        _ => None,
    });

    session_meta_version.or_else(|| {
        items.iter().rev().find_map(|item| match item {
            RolloutItem::TurnContext(turn_context) => turn_context.multi_agent_version,
            RolloutItem::SessionMeta(_)
            | RolloutItem::ResponseItem(_)
            | RolloutItem::InterAgentCommunication(_)
            | RolloutItem::Compacted(_)
            | RolloutItem::EventMsg(_) => None,
        })
    })
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum MultiAgentVersion {
    Disabled,
    V1,
    V2,
}

pub const MAX_THREAD_GOAL_OBJECTIVE_CHARS: usize = 4_000;

pub fn validate_thread_goal_objective(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("goal objective must not be empty".to_string());
    }
    if value.chars().count() > MAX_THREAD_GOAL_OBJECTIVE_CHARS {
        return Err(format!(
            "goal objective must be at most {MAX_THREAD_GOAL_OBJECTIVE_CHARS} characters"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SubAgentActivityKind {
    Started,
    Interacted,
    Interrupted,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
pub struct SubAgentActivityEvent {
    pub event_id: String,
    #[serde(default)]
    pub occurred_at_ms: i64,
    /// Thread ID of the affected sub-agent.
    pub agent_thread_id: ThreadId,
    /// Canonical v2 path of the affected sub-agent.
    pub agent_path: AgentPath,
    pub kind: SubAgentActivityKind,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::FileSystemAccessMode;
    use crate::permissions::FileSystemPath;
    use crate::permissions::FileSystemSandboxEntry;
    use crate::permissions::FileSystemSandboxPolicy;
    use crate::permissions::FileSystemSpecialPath;
    use crate::permissions::NetworkSandboxPolicy;
    use anyhow::Result;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use codex_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::path::Path;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn feature_thread_source_serializes_as_its_app_owned_label() -> Result<()> {
        let source = ThreadSource::Feature("automation".to_string());

        assert_eq!(serde_json::to_value(&source)?, json!("automation"));
        assert_eq!(
            serde_json::from_value::<ThreadSource>(json!("automation"))?,
            source
        );
        Ok(())
    }

    #[test]
    fn session_meta_normalizes_legacy_dynamic_tools() -> Result<()> {
        let mut value = serde_json::to_value(SessionMeta::default())?;
        value["dynamic_tools"] = json!([
            {
                "namespace": "legacy_app",
                "name": "lookup_ticket",
                "description": "Look up a ticket",
                "inputSchema": {"type": "object", "properties": {}},
                "exposeToContext": false
            },
            {
                "namespace": "legacy_app",
                "name": "update_ticket",
                "description": "Update a ticket",
                "inputSchema": {"type": "object", "properties": {}},
                "deferLoading": false,
                "exposeToContext": false
            }
        ]);

        let meta: SessionMeta = serde_json::from_value(value)?;

        assert_eq!(
            meta.dynamic_tools,
            Some(vec![DynamicToolSpec::Namespace(
                crate::dynamic_tools::DynamicToolNamespaceSpec {
                    name: "legacy_app".to_string(),
                    description: String::new(),
                    tools: vec![
                        crate::dynamic_tools::DynamicToolNamespaceTool::Function(
                            crate::dynamic_tools::DynamicToolFunctionSpec {
                                name: "lookup_ticket".to_string(),
                                description: "Look up a ticket".to_string(),
                                input_schema: json!({"type": "object", "properties": {}}),
                                defer_loading: true,
                            },
                        ),
                        crate::dynamic_tools::DynamicToolNamespaceTool::Function(
                            crate::dynamic_tools::DynamicToolFunctionSpec {
                                name: "update_ticket".to_string(),
                                description: "Update a ticket".to_string(),
                                input_schema: json!({"type": "object", "properties": {}}),
                                defer_loading: false,
                            },
                        ),
                    ],
                },
            )])
        );
        Ok(())
    }

    fn sorted_writable_roots(roots: Vec<WritableRoot>) -> Vec<(PathBuf, Vec<PathBuf>)> {
        let mut sorted_roots: Vec<(PathBuf, Vec<PathBuf>)> = roots
            .into_iter()
            .map(|root| {
                let mut read_only_subpaths: Vec<PathBuf> = root
                    .read_only_subpaths
                    .into_iter()
                    .map(|path| path.to_path_buf())
                    .collect();
                read_only_subpaths.sort();
                (root.root.to_path_buf(), read_only_subpaths)
            })
            .collect();
        sorted_roots.sort_by(|left, right| left.0.cmp(&right.0));
        sorted_roots
    }

    fn sandbox_policy_allows_read(policy: &SandboxPolicy, _path: &Path, _cwd: &Path) -> bool {
        policy.has_full_disk_read_access()
    }

    fn sandbox_policy_allows_write(policy: &SandboxPolicy, path: &Path, cwd: &Path) -> bool {
        if policy.has_full_disk_write_access() {
            return true;
        }

        policy
            .get_writable_roots_with_cwd(cwd)
            .iter()
            .any(|root| root.is_path_writable(path))
    }

    #[test]
    fn inter_agent_communication_response_input_item_preserves_commentary_phase() {
        let communication = InterAgentCommunication {
            author: AgentPath::root(),
            recipient: AgentPath::root().join("reviewer").expect("recipient path"),
            other_recipients: vec![AgentPath::root().join("worker").expect("recipient path")],
            content: "review the diff".to_string(),
            encrypted_content: None,
            metadata: None,
            trigger_turn: true,
        };

        assert_eq!(
            communication.to_response_input_item(),
            ResponseInputItem::Message {
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: serde_json::to_string(&communication).expect("serialize communication"),
                }],
                phase: Some(MessagePhase::Commentary),
            }
        );
    }

    #[test]
    fn queued_encrypted_inter_agent_communication_renders_message_envelope() {
        let communication = InterAgentCommunication::new_encrypted(
            AgentPath::root().join("worker").expect("author path"),
            AgentPath::root(),
            Vec::new(),
            "encrypted payload".to_string(),
            /*trigger_turn*/ false,
        );

        assert_eq!(
            communication.to_model_input_item(),
            ResponseItem::AgentMessage {
                id: None,
                author: "/root/worker".to_string(),
                recipient: "/root".to_string(),
                content: vec![
                    AgentMessageInputContent::InputText {
                        text: "Message Type: MESSAGE\nTask name: /root\nSender: /root/worker\nPayload:\n"
                            .to_string(),
                    },
                    AgentMessageInputContent::EncryptedContent {
                        encrypted_content: "encrypted payload".to_string(),
                    },
                ],
                metadata: None,
            }
        );
    }

    #[test]
    fn session_source_from_startup_arg_normalizes_custom_values() {
        assert_eq!(
            SessionSource::from_startup_arg("atlas").unwrap(),
            SessionSource::Custom("atlas".to_string())
        );
        assert_eq!(
            SessionSource::from_startup_arg(" Atlas ").unwrap(),
            SessionSource::Custom("atlas".to_string())
        );
    }

    #[test]
    fn session_source_restriction_product_defaults_non_subagent_sources_to_codex() {
        assert_eq!(
            SessionSource::Cli.restriction_product(),
            Some(Product::Codex)
        );
        assert_eq!(
            SessionSource::VSCode.restriction_product(),
            Some(Product::Codex)
        );
        assert_eq!(
            SessionSource::Exec.restriction_product(),
            Some(Product::Codex)
        );
        assert_eq!(
            SessionSource::Mcp.restriction_product(),
            Some(Product::Codex)
        );
        assert_eq!(
            SessionSource::Unknown.restriction_product(),
            Some(Product::Codex)
        );
    }

    #[test]
    fn session_source_restriction_product_does_not_guess_subagent_products() {
        assert_eq!(
            SessionSource::SubAgent(SubAgentSource::Review).restriction_product(),
            None
        );
        assert_eq!(
            SessionSource::Internal(InternalSessionSource::MemoryConsolidation)
                .restriction_product(),
            None
        );
    }

    #[test]
    fn session_source_restriction_product_maps_custom_sources_to_products() {
        assert_eq!(
            SessionSource::Custom("chatgpt".to_string()).restriction_product(),
            Some(Product::Chatgpt)
        );
        assert_eq!(
            SessionSource::Custom("ATLAS".to_string()).restriction_product(),
            Some(Product::Atlas)
        );
        assert_eq!(
            SessionSource::Custom("codex".to_string()).restriction_product(),
            Some(Product::Codex)
        );
        assert_eq!(
            SessionSource::Custom("atlas-dev".to_string()).restriction_product(),
            None
        );
    }

    #[test]
    fn session_source_matches_product_restriction() {
        assert!(
            SessionSource::Custom("chatgpt".to_string())
                .matches_product_restriction(&[Product::Chatgpt])
        );
        assert!(
            !SessionSource::Custom("chatgpt".to_string())
                .matches_product_restriction(&[Product::Codex])
        );
        assert!(SessionSource::VSCode.matches_product_restriction(&[Product::Codex]));
        assert!(
            !SessionSource::Custom("atlas-dev".to_string())
                .matches_product_restriction(&[Product::Atlas])
        );
        assert!(SessionSource::Custom("atlas-dev".to_string()).matches_product_restriction(&[]));
    }

    fn sandbox_policy_probe_paths(policy: &SandboxPolicy, cwd: &Path) -> Vec<PathBuf> {
        let mut paths = vec![cwd.to_path_buf()];
        for root in policy.get_writable_roots_with_cwd(cwd) {
            paths.push(root.root.to_path_buf());
            paths.extend(
                root.read_only_subpaths
                    .into_iter()
                    .map(|path| path.to_path_buf()),
            );
        }
        paths.sort();
        paths.dedup();
        paths
    }

    fn assert_same_sandbox_policy_semantics(
        expected: &SandboxPolicy,
        actual: &SandboxPolicy,
        cwd: &Path,
    ) {
        assert_eq!(
            actual.has_full_disk_read_access(),
            expected.has_full_disk_read_access()
        );
        assert_eq!(
            actual.has_full_disk_write_access(),
            expected.has_full_disk_write_access()
        );
        assert_eq!(
            actual.has_full_network_access(),
            expected.has_full_network_access()
        );
        let mut probe_paths = sandbox_policy_probe_paths(expected, cwd);
        probe_paths.extend(sandbox_policy_probe_paths(actual, cwd));
        probe_paths.sort();
        probe_paths.dedup();

        for path in probe_paths {
            assert_eq!(
                sandbox_policy_allows_read(actual, &path, cwd),
                sandbox_policy_allows_read(expected, &path, cwd),
                "read access mismatch for {}",
                path.display()
            );
            assert_eq!(
                sandbox_policy_allows_write(actual, &path, cwd),
                sandbox_policy_allows_write(expected, &path, cwd),
                "write access mismatch for {}",
                path.display()
            );
        }
    }

    #[test]
    fn external_sandbox_reports_full_access_flags() {
        let restricted = SandboxPolicy::ExternalSandbox {
            network_access: NetworkAccess::Restricted,
        };
        assert!(restricted.has_full_disk_write_access());
        assert!(!restricted.has_full_network_access());

        let enabled = SandboxPolicy::ExternalSandbox {
            network_access: NetworkAccess::Enabled,
        };
        assert!(enabled.has_full_disk_write_access());
        assert!(enabled.has_full_network_access());
    }

    #[test]
    fn read_only_reports_network_access_flags() {
        let restricted = SandboxPolicy::new_read_only_policy();
        assert!(!restricted.has_full_network_access());

        let enabled = SandboxPolicy::ReadOnly {
            network_access: true,
        };
        assert!(enabled.has_full_network_access());
    }

    #[test]
    fn granular_approval_config_mcp_elicitation_flag_is_field_driven() {
        assert!(
            GranularApprovalConfig {
                sandbox_approval: false,
                rules: false,
                skill_approval: false,
                request_permissions: false,
                mcp_elicitations: true,
            }
            .allows_mcp_elicitations()
        );
        assert!(
            !GranularApprovalConfig {
                sandbox_approval: false,
                rules: false,
                skill_approval: false,
                request_permissions: false,
                mcp_elicitations: false,
            }
            .allows_mcp_elicitations()
        );
    }

    #[test]
    fn granular_approval_config_skill_approval_flag_is_field_driven() {
        assert!(
            GranularApprovalConfig {
                sandbox_approval: false,
                rules: false,
                skill_approval: true,
                request_permissions: false,
                mcp_elicitations: false,
            }
            .allows_skill_approval()
        );
        assert!(
            !GranularApprovalConfig {
                sandbox_approval: false,
                rules: false,
                skill_approval: false,
                request_permissions: false,
                mcp_elicitations: false,
            }
            .allows_skill_approval()
        );
    }

    #[test]
    fn granular_approval_config_request_permissions_flag_is_field_driven() {
        assert!(
            GranularApprovalConfig {
                sandbox_approval: false,
                rules: false,
                skill_approval: false,
                request_permissions: true,
                mcp_elicitations: false,
            }
            .allows_request_permissions()
        );
        assert!(
            !GranularApprovalConfig {
                sandbox_approval: false,
                rules: false,
                skill_approval: false,
                request_permissions: false,
                mcp_elicitations: false,
            }
            .allows_request_permissions()
        );
    }

    #[test]
    fn granular_approval_config_defaults_missing_optional_flags_to_false() {
        let decoded = serde_json::from_value::<GranularApprovalConfig>(serde_json::json!({
            "sandbox_approval": true,
            "rules": false,
            "mcp_elicitations": true,
        }))
        .expect("granular approval config should deserialize");

        assert_eq!(
            decoded,
            GranularApprovalConfig {
                sandbox_approval: true,
                rules: false,
                skill_approval: false,
                request_permissions: false,
                mcp_elicitations: true,
            }
        );
    }

    #[test]
    fn restricted_file_system_policy_reports_full_access_from_root_entries() {
        let read_only = FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::Root,
            },
            access: FileSystemAccessMode::Read,
        }]);
        assert!(read_only.has_full_disk_read_access());
        assert!(!read_only.has_full_disk_write_access());
        assert!(!read_only.include_platform_defaults());

        let writable = FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::Root,
            },
            access: FileSystemAccessMode::Write,
        }]);
        assert!(writable.has_full_disk_read_access());
        assert!(writable.has_full_disk_write_access());
    }

    #[test]
    fn restricted_file_system_policy_treats_root_with_carveouts_as_scoped_access() {
        let cwd = TempDir::new().expect("tempdir");
        let canonical_cwd = codex_utils_absolute_path::canonicalize_preserving_symlinks(cwd.path())
            .expect("canonicalize cwd");
        let root = AbsolutePathBuf::from_absolute_path(&canonical_cwd)
            .expect("absolute canonical tempdir")
            .as_path()
            .ancestors()
            .last()
            .and_then(|path| AbsolutePathBuf::from_absolute_path(path).ok())
            .expect("filesystem root");
        let blocked = AbsolutePathBuf::resolve_path_against_base("blocked", cwd.path());
        let expected_blocked = AbsolutePathBuf::from_absolute_path(
            codex_utils_absolute_path::canonicalize_preserving_symlinks(cwd.path())
                .expect("canonicalize cwd")
                .join("blocked"),
        )
        .expect("canonical blocked");
        let policy = FileSystemSandboxPolicy::restricted(vec![
            FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::Root,
                },
                access: FileSystemAccessMode::Write,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Path { path: blocked },
                access: FileSystemAccessMode::None,
            },
        ]);

        assert!(!policy.has_full_disk_read_access());
        assert!(!policy.has_full_disk_write_access());
        assert_eq!(
            policy.get_readable_roots_with_cwd(cwd.path()),
            vec![root.clone()]
        );
        assert_eq!(
            policy.get_unreadable_roots_with_cwd(cwd.path()),
            vec![expected_blocked.clone()]
        );

        let writable_roots = policy.get_writable_roots_with_cwd(cwd.path());
        assert_eq!(writable_roots.len(), 1);
        assert_eq!(writable_roots[0].root, root);
        assert!(
            writable_roots[0]
                .read_only_subpaths
                .iter()
                .any(|path| path.as_path() == expected_blocked.as_path())
        );
    }

    #[test]
    fn restricted_file_system_policy_derives_effective_paths() {
        let cwd = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(cwd.path().join(".agents")).expect("create .agents");
        std::fs::create_dir_all(cwd.path().join(".codex")).expect("create .codex");
        let canonical_cwd = codex_utils_absolute_path::canonicalize_preserving_symlinks(cwd.path())
            .expect("canonicalize cwd");
        let cwd_absolute =
            AbsolutePathBuf::from_absolute_path(&canonical_cwd).expect("absolute tempdir");
        let secret = AbsolutePathBuf::resolve_path_against_base("secret", cwd.path());
        let expected_secret = AbsolutePathBuf::from_absolute_path(canonical_cwd.join("secret"))
            .expect("canonical secret");
        let expected_agents = AbsolutePathBuf::from_absolute_path(canonical_cwd.join(".agents"))
            .expect("canonical .agents");
        let expected_codex = AbsolutePathBuf::from_absolute_path(canonical_cwd.join(".codex"))
            .expect("canonical .codex");
        let policy = FileSystemSandboxPolicy::restricted(vec![
            FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::Minimal,
                },
                access: FileSystemAccessMode::Read,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
                },
                access: FileSystemAccessMode::Write,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Path { path: secret },
                access: FileSystemAccessMode::None,
            },
        ]);

        assert!(!policy.has_full_disk_read_access());
        assert!(!policy.has_full_disk_write_access());
        assert!(policy.include_platform_defaults());
        assert_eq!(
            policy.get_readable_roots_with_cwd(cwd.path()),
            vec![cwd_absolute.clone()]
        );
        assert_eq!(
            policy.get_unreadable_roots_with_cwd(cwd.path()),
            vec![expected_secret.clone()]
        );

        let writable_roots = policy.get_writable_roots_with_cwd(cwd.path());
        assert_eq!(writable_roots.len(), 1);
        assert_eq!(writable_roots[0].root, cwd_absolute);
        assert!(
            writable_roots[0]
                .read_only_subpaths
                .iter()
                .any(|path| path.as_path() == expected_secret.as_path())
        );
        assert!(
            writable_roots[0]
                .read_only_subpaths
                .iter()
                .any(|path| path.as_path() == expected_agents.as_path())
        );
        assert!(
            writable_roots[0]
                .read_only_subpaths
                .iter()
                .any(|path| path.as_path() == expected_codex.as_path())
        );
    }

    #[test]
    fn restricted_file_system_policy_treats_read_entries_as_read_only_subpaths() {
        let cwd = TempDir::new().expect("tempdir");
        let canonical_cwd = codex_utils_absolute_path::canonicalize_preserving_symlinks(cwd.path())
            .expect("canonicalize cwd");
        let docs = AbsolutePathBuf::resolve_path_against_base("docs", cwd.path());
        let docs_public = AbsolutePathBuf::resolve_path_against_base("docs/public", cwd.path());
        let expected_docs = AbsolutePathBuf::from_absolute_path(canonical_cwd.join("docs"))
            .expect("canonical docs");
        let expected_docs_public =
            AbsolutePathBuf::from_absolute_path(canonical_cwd.join("docs/public"))
                .expect("canonical docs/public");
        let expected_dot_codex = AbsolutePathBuf::from_absolute_path(canonical_cwd.join(".codex"))
            .expect("canonical .codex");
        let policy = FileSystemSandboxPolicy::restricted(vec![
            FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
                },
                access: FileSystemAccessMode::Write,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Path { path: docs },
                access: FileSystemAccessMode::Read,
            },
            FileSystemSandboxEntry {
                path: FileSystemPath::Path { path: docs_public },
                access: FileSystemAccessMode::Write,
            },
        ]);

        assert!(!policy.has_full_disk_write_access());
        assert_eq!(
            sorted_writable_roots(policy.get_writable_roots_with_cwd(cwd.path())),
            vec![
                (
                    canonical_cwd,
                    vec![
                        expected_dot_codex.to_path_buf(),
                        expected_docs.to_path_buf()
                    ],
                ),
                (expected_docs_public.to_path_buf(), Vec::new()),
            ]
        );
    }

    #[test]
    fn file_system_policy_rejects_legacy_bridge_for_non_workspace_writes() {
        let cwd = if cfg!(windows) {
            Path::new(r"C:\workspace")
        } else {
            Path::new("/tmp/workspace")
        };
        let external_write_path = if cfg!(windows) {
            AbsolutePathBuf::from_absolute_path(r"C:\temp").expect("absolute windows temp path")
        } else {
            AbsolutePathBuf::from_absolute_path("/tmp").expect("absolute tmp path")
        };
        let policy = FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: external_write_path,
            },
            access: FileSystemAccessMode::Write,
        }]);

        let err = policy
            .to_legacy_sandbox_policy(NetworkSandboxPolicy::Restricted, cwd)
            .expect_err("non-workspace writes should be rejected");

        assert!(
            err.to_string()
                .contains("filesystem writes outside the workspace root"),
            "{err}"
        );
    }

    #[test]
    fn legacy_sandbox_policy_semantics_survive_split_bridge() {
        let cwd = TempDir::new().expect("tempdir");
        let writable_root = AbsolutePathBuf::resolve_path_against_base("writable", cwd.path());
        let policies = [
            SandboxPolicy::DangerFullAccess,
            SandboxPolicy::ExternalSandbox {
                network_access: NetworkAccess::Restricted,
            },
            SandboxPolicy::ExternalSandbox {
                network_access: NetworkAccess::Enabled,
            },
            SandboxPolicy::ReadOnly {
                network_access: false,
            },
            SandboxPolicy::WorkspaceWrite {
                writable_roots: vec![],
                network_access: false,
                exclude_tmpdir_env_var: true,
                exclude_slash_tmp: true,
            },
            SandboxPolicy::WorkspaceWrite {
                writable_roots: vec![writable_root],
                network_access: true,
                exclude_tmpdir_env_var: false,
                exclude_slash_tmp: true,
            },
        ];

        for expected in policies {
            let actual =
                FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(&expected, cwd.path())
                    .to_legacy_sandbox_policy(NetworkSandboxPolicy::from(&expected), cwd.path())
                    .expect("legacy bridge should preserve legacy policy semantics");

            assert_same_sandbox_policy_semantics(&expected, &actual, cwd.path());
        }
    }

    #[test]
    fn item_started_event_from_web_search_emits_begin_event() {
        let event = ItemStartedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            item: TurnItem::WebSearch(WebSearchItem {
                id: "search-1".into(),
                query: "find docs".into(),
                action: WebSearchAction::Search {
                    query: Some("find docs".into()),
                    queries: None,
                },
            }),
            started_at_ms: 0,
        };

        let legacy_events = event.as_legacy_events(/*show_raw_agent_reasoning*/ false);
        assert_eq!(legacy_events.len(), 1);
        match &legacy_events[0] {
            EventMsg::WebSearchBegin(event) => assert_eq!(event.call_id, "search-1"),
            _ => panic!("expected WebSearchBegin event"),
        }
    }

    #[test]
    fn item_started_event_from_non_web_search_emits_no_legacy_events() {
        let event = ItemStartedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            item: TurnItem::UserMessage(UserMessageItem::new(&[])),
            started_at_ms: 0,
        };

        assert!(
            event
                .as_legacy_events(/*show_raw_agent_reasoning*/ false)
                .is_empty()
        );
    }

    #[test]
    fn item_started_event_from_image_generation_emits_begin_event() {
        let event = ItemStartedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            item: TurnItem::ImageGeneration(ImageGenerationItem {
                id: "ig-1".into(),
                status: "in_progress".into(),
                revised_prompt: None,
                result: String::new(),
                saved_path: None,
            }),
            started_at_ms: 0,
        };

        let legacy_events = event.as_legacy_events(/*show_raw_agent_reasoning*/ false);
        assert_eq!(legacy_events.len(), 1);
        match &legacy_events[0] {
            EventMsg::ImageGenerationBegin(event) => assert_eq!(event.call_id, "ig-1"),
            _ => panic!("expected ImageGenerationBegin event"),
        }
    }

    #[test]
    fn item_started_event_from_file_change_emits_patch_begin_event() {
        let event = ItemStartedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            started_at_ms: 0,
            item: TurnItem::FileChange(FileChangeItem {
                id: "patch-1".into(),
                changes: [(
                    PathBuf::from("new.txt"),
                    FileChange::Add {
                        content: "hello".into(),
                    },
                )]
                .into_iter()
                .collect(),
                status: None,
                auto_approved: Some(true),
                stdout: None,
                stderr: None,
            }),
        };

        let legacy_events = event.as_legacy_events(/*show_raw_agent_reasoning*/ false);
        assert_eq!(legacy_events.len(), 1);
        match &legacy_events[0] {
            EventMsg::PatchApplyBegin(event) => {
                assert_eq!(event.call_id, "patch-1");
                assert_eq!(event.turn_id, "turn-1");
                assert!(event.auto_approved);
                assert!(event.changes.contains_key(&PathBuf::from("new.txt")));
            }
            _ => panic!("expected PatchApplyBegin event"),
        }
    }

    #[test]
    fn item_started_event_from_mcp_tool_call_emits_begin_event() {
        let event = ItemStartedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            started_at_ms: 0,
            item: TurnItem::McpToolCall(McpToolCallItem {
                id: "mcp-1".into(),
                server: "server".into(),
                tool: "tool".into(),
                arguments: json!({"arg": "value"}),
                connector_id: Some("connector".into()),
                mcp_app_resource_uri: Some("app://connector".into()),
                link_id: Some("link_123".into()),
                plugin_id: Some("sample@test".into()),
                status: McpToolCallStatus::InProgress,
                result: None,
                error: None,
                duration: None,
            }),
        };

        let legacy_events = event.as_legacy_events(/*show_raw_agent_reasoning*/ false);
        assert_eq!(legacy_events.len(), 1);
        match &legacy_events[0] {
            EventMsg::McpToolCallBegin(event) => {
                assert_eq!(event.call_id, "mcp-1");
                assert_eq!(event.invocation.server, "server");
                assert_eq!(event.invocation.tool, "tool");
                assert_eq!(event.connector_id.as_deref(), Some("connector"));
                assert_eq!(
                    event.mcp_app_resource_uri.as_deref(),
                    Some("app://connector")
                );
                assert_eq!(event.link_id.as_deref(), Some("link_123"));
                assert_eq!(event.plugin_id.as_deref(), Some("sample@test"));
            }
            _ => panic!("expected McpToolCallBegin event"),
        }
    }

    #[test]
    fn item_completed_event_from_image_generation_emits_end_event() {
        let event = ItemCompletedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            item: TurnItem::ImageGeneration(ImageGenerationItem {
                id: "ig-1".into(),
                status: "completed".into(),
                revised_prompt: Some("A tiny blue square".into()),
                result: "Zm9v".into(),
                saved_path: Some(test_path_buf("/tmp/ig-1.png").abs()),
            }),
            completed_at_ms: 0,
        };

        let legacy_events = event.as_legacy_events(/*show_raw_agent_reasoning*/ false);
        assert_eq!(legacy_events.len(), 1);
        match &legacy_events[0] {
            EventMsg::ImageGenerationEnd(event) => {
                assert_eq!(event.call_id, "ig-1");
                assert_eq!(event.status, "completed");
                assert_eq!(event.revised_prompt.as_deref(), Some("A tiny blue square"));
                assert_eq!(event.result, "Zm9v");
                assert_eq!(
                    event.saved_path.as_ref().map(AbsolutePathBuf::as_path),
                    Some(test_path_buf("/tmp/ig-1.png").as_path())
                );
            }
            _ => panic!("expected ImageGenerationEnd event"),
        }
    }

    #[test]
    fn item_completed_event_from_file_change_emits_patch_end_event() {
        let event = ItemCompletedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            completed_at_ms: 0,
            item: TurnItem::FileChange(FileChangeItem {
                id: "patch-1".into(),
                changes: [(
                    PathBuf::from("new.txt"),
                    FileChange::Add {
                        content: "hello".into(),
                    },
                )]
                .into_iter()
                .collect(),
                status: Some(PatchApplyStatus::Completed),
                auto_approved: None,
                stdout: Some("Done!".into()),
                stderr: Some(String::new()),
            }),
        };

        let legacy_events = event.as_legacy_events(/*show_raw_agent_reasoning*/ false);
        assert_eq!(legacy_events.len(), 1);
        match &legacy_events[0] {
            EventMsg::PatchApplyEnd(event) => {
                assert_eq!(event.call_id, "patch-1");
                assert_eq!(event.turn_id, "turn-1");
                assert_eq!(event.stdout, "Done!");
                assert!(event.success);
                assert_eq!(event.status, PatchApplyStatus::Completed);
                assert!(event.changes.contains_key(&PathBuf::from("new.txt")));
            }
            _ => panic!("expected PatchApplyEnd event"),
        }
    }

    #[test]
    fn item_completed_event_from_mcp_tool_call_emits_end_event() {
        let event = ItemCompletedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            completed_at_ms: 0,
            item: TurnItem::McpToolCall(McpToolCallItem {
                id: "mcp-1".into(),
                server: "server".into(),
                tool: "tool".into(),
                arguments: json!({"arg": "value"}),
                connector_id: Some("connector".into()),
                mcp_app_resource_uri: Some("app://connector".into()),
                link_id: Some("link_123".into()),
                plugin_id: Some("sample@test".into()),
                status: McpToolCallStatus::Completed,
                result: Some(CallToolResult {
                    content: vec![json!({"type": "text", "text": "ok"})],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
                error: None,
                duration: Some(Duration::from_millis(42)),
            }),
        };

        let legacy_events = event.as_legacy_events(/*show_raw_agent_reasoning*/ false);
        assert_eq!(legacy_events.len(), 1);
        match &legacy_events[0] {
            EventMsg::McpToolCallEnd(event) => {
                assert_eq!(event.call_id, "mcp-1");
                assert_eq!(event.invocation.server, "server");
                assert_eq!(event.invocation.tool, "tool");
                assert_eq!(event.connector_id.as_deref(), Some("connector"));
                assert_eq!(
                    event.mcp_app_resource_uri.as_deref(),
                    Some("app://connector")
                );
                assert_eq!(event.link_id.as_deref(), Some("link_123"));
                assert_eq!(event.plugin_id.as_deref(), Some("sample@test"));
                assert_eq!(event.duration, Duration::from_millis(42));
                assert!(event.is_success());
            }
            _ => panic!("expected McpToolCallEnd event"),
        }
    }

    #[test]
    fn item_started_event_requires_started_at_ms() {
        let mut value = serde_json::to_value(ItemStartedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            item: TurnItem::UserMessage(UserMessageItem::new(&[])),
            started_at_ms: 123,
        })
        .unwrap();
        value.as_object_mut().unwrap().remove("started_at_ms");

        assert!(serde_json::from_value::<ItemStartedEvent>(value).is_err());
    }

    #[test]
    fn item_completed_event_defaults_missing_completed_at_ms() {
        let mut value = serde_json::to_value(ItemCompletedEvent {
            thread_id: ThreadId::new(),
            turn_id: "turn-1".into(),
            item: TurnItem::UserMessage(UserMessageItem::new(&[])),
            completed_at_ms: 123,
        })
        .unwrap();
        value.as_object_mut().unwrap().remove("completed_at_ms");

        let event = serde_json::from_value::<ItemCompletedEvent>(value).unwrap();
        assert_eq!(event.completed_at_ms, 0);
    }
    #[test]
    fn rollback_failed_error_does_not_affect_turn_status() {
        let event = ErrorEvent {
            message: "rollback failed".into(),
            codex_error_info: Some(CodexErrorInfo::ThreadRollbackFailed),
        };
        assert!(!event.affects_turn_status());
    }

    #[test]
    fn active_turn_not_steerable_error_does_not_affect_turn_status() {
        let event = ErrorEvent {
            message: "cannot steer a review turn".into(),
            codex_error_info: Some(CodexErrorInfo::ActiveTurnNotSteerable {
                turn_kind: NonSteerableTurnKind::Review,
            }),
        };
        assert!(!event.affects_turn_status());
    }

    #[test]
    fn generic_error_affects_turn_status() {
        let event = ErrorEvent {
            message: "generic".into(),
            codex_error_info: Some(CodexErrorInfo::Other),
        };
        assert!(event.affects_turn_status());
    }

    #[test]
    fn realtime_conversation_started_event_uses_realtime_session_id() {
        let event = RealtimeConversationStartedEvent {
            realtime_session_id: Some("conv_1".to_string()),
            version: RealtimeConversationVersion::V2,
        };

        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            json!({
                "realtime_session_id": "conv_1",
                "version": "v2"
            })
        );
    }

    #[test]
    fn realtime_voice_list_is_stable() {
        assert_eq!(
            RealtimeVoicesList::builtin(),
            RealtimeVoicesList {
                v1: vec![
                    RealtimeVoice::Juniper,
                    RealtimeVoice::Maple,
                    RealtimeVoice::Spruce,
                    RealtimeVoice::Ember,
                    RealtimeVoice::Vale,
                    RealtimeVoice::Breeze,
                    RealtimeVoice::Arbor,
                    RealtimeVoice::Sol,
                    RealtimeVoice::Cove,
                ],
                v2: vec![
                    RealtimeVoice::Alloy,
                    RealtimeVoice::Ash,
                    RealtimeVoice::Ballad,
                    RealtimeVoice::Coral,
                    RealtimeVoice::Echo,
                    RealtimeVoice::Sage,
                    RealtimeVoice::Shimmer,
                    RealtimeVoice::Verse,
                    RealtimeVoice::Marin,
                    RealtimeVoice::Cedar,
                ],
                default_v1: RealtimeVoice::Cove,
                default_v2: RealtimeVoice::Marin,
            }
        );
    }

    #[test]
    fn user_input_serialization_omits_final_output_json_schema_when_none() -> Result<()> {
        let op = Op::UserInput {
            environments: None,
            items: Vec::new(),
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides::default(),
        };

        let json_op = serde_json::to_value(op)?;
        assert_eq!(json_op, json!({ "type": "user_input", "items": [] }));

        Ok(())
    }

    #[test]
    fn user_input_deserializes_without_final_output_json_schema_field() -> Result<()> {
        let op: Op = serde_json::from_value(json!({ "type": "user_input", "items": [] }))?;

        assert_eq!(
            op,
            Op::UserInput {
                environments: None,
                items: Vec::new(),
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
                additional_context: Default::default(),
                thread_settings: ThreadSettingsOverrides::default(),
            }
        );

        Ok(())
    }

    #[test]
    fn user_turn_context_budget_mode_is_optional_for_backcompat() -> Result<()> {
        let user_turn = |context_budget_mode| Op::UserTurn {
            environments: None,
            items: Vec::new(),
            cwd: test_path_buf("/tmp/project"),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            permission_profile: None,
            model: "gpt-5.4".to_string(),
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode,
            final_output_json_schema: None,
            collaboration_mode: None,
            personality: None,
        };

        let omitted_json = serde_json::to_value(user_turn(None))?;
        assert_eq!(omitted_json.get("context_budget_mode"), None);
        assert_eq!(serde_json::from_value::<Op>(omitted_json)?, user_turn(None));

        let slow_json = serde_json::to_value(user_turn(Some(ContextBudgetMode::Slow)))?;
        assert_eq!(slow_json.get("context_budget_mode"), Some(&json!("slow")));
        let Op::UserTurn {
            context_budget_mode,
            ..
        } = serde_json::from_value::<Op>(slow_json)?
        else {
            panic!("expected user turn");
        };
        assert_eq!(context_budget_mode, Some(ContextBudgetMode::Slow));

        Ok(())
    }

    #[test]
    fn user_input_serialization_includes_final_output_json_schema_when_some() -> Result<()> {
        let schema = json!({
            "type": "object",
            "properties": {
                "answer": { "type": "string" }
            },
            "required": ["answer"],
            "additionalProperties": false
        });
        let op = Op::UserInput {
            environments: None,
            items: Vec::new(),
            final_output_json_schema: Some(schema.clone()),
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides::default(),
        };

        let json_op = serde_json::to_value(op)?;
        assert_eq!(
            json_op,
            json!({
                "type": "user_input",
                "items": [],
                "final_output_json_schema": schema,
            })
        );

        Ok(())
    }

    #[test]
    fn user_input_with_responsesapi_client_metadata_round_trips() -> Result<()> {
        let op = Op::UserInput {
            environments: None,
            items: Vec::new(),
            final_output_json_schema: None,
            responsesapi_client_metadata: Some(HashMap::from([(
                "fiber_run_id".to_string(),
                "fiber-123".to_string(),
            )])),
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides::default(),
        };

        let json_op = serde_json::to_value(&op)?;
        assert_eq!(
            json_op,
            json!({
                "type": "user_input",
                "items": [],
                "responsesapi_client_metadata": {
                    "fiber_run_id": "fiber-123",
                }
            })
        );
        assert_eq!(serde_json::from_value::<Op>(json_op)?, op);

        Ok(())
    }

    #[test]
    fn user_input_text_serializes_empty_text_elements() -> Result<()> {
        let input = UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        };

        let json_input = serde_json::to_value(input)?;
        assert_eq!(
            json_input,
            json!({
                "type": "text",
                "text": "hello",
                "text_elements": [],
            })
        );

        Ok(())
    }

    #[test]
    fn user_message_event_serializes_empty_metadata_vectors() -> Result<()> {
        let event = UserMessageEvent {
            client_id: None,
            message: "hello".to_string(),
            images: None,
            image_details: Vec::new(),
            local_images: Vec::new(),
            local_image_details: Vec::new(),
            text_elements: Vec::new(),
        };

        let json_event = serde_json::to_value(event)?;
        assert_eq!(
            json_event,
            json!({
                "message": "hello",
                "image_details": [],
                "local_images": [],
                "local_image_details": [],
                "text_elements": [],
            })
        );

        Ok(())
    }

    #[test]
    fn user_message_event_deserializes_without_image_detail_fields() -> Result<()> {
        let event: UserMessageEvent = serde_json::from_value(json!({
            "message": "hello",
            "images": ["https://example.com/image.png"],
            "local_images": ["/tmp/local.png"],
            "text_elements": [],
        }))?;

        assert_eq!(event.message, "hello");
        assert_eq!(
            event.images,
            Some(vec!["https://example.com/image.png".to_string()])
        );
        assert_eq!(event.image_details, Vec::<Option<ImageDetail>>::new());
        assert_eq!(event.local_images, vec![PathBuf::from("/tmp/local.png")]);
        assert_eq!(event.local_image_details, Vec::<Option<ImageDetail>>::new());
        assert_eq!(event.text_elements, Vec::new());

        Ok(())
    }

    #[test]
    fn user_message_item_legacy_event_preserves_image_details() {
        let local_path = PathBuf::from("/tmp/local.png");
        let mut item = UserMessageItem::new(&[
            crate::user_input::UserInput::Image {
                image_url: "https://example.com/first.png".to_string(),
                detail: Some(ImageDetail::Original),
            },
            crate::user_input::UserInput::Image {
                image_url: "https://example.com/second.png".to_string(),
                detail: None,
            },
            crate::user_input::UserInput::LocalImage {
                path: local_path.clone(),
                detail: Some(ImageDetail::Original),
            },
        ]);
        item.client_id = Some("client-message-1".to_string());

        let EventMsg::UserMessage(event) = item.as_legacy_event() else {
            panic!("expected user message event");
        };

        assert_eq!(
            event.images,
            Some(vec![
                "https://example.com/first.png".to_string(),
                "https://example.com/second.png".to_string(),
            ])
        );
        assert_eq!(event.image_details, vec![Some(ImageDetail::Original)]);
        assert_eq!(event.local_images, vec![local_path]);
        assert_eq!(event.local_image_details, vec![Some(ImageDetail::Original)]);
    }

    #[test]
    fn turn_aborted_event_deserializes_without_turn_id() -> Result<()> {
        let event: EventMsg = serde_json::from_value(json!({
            "type": "turn_aborted",
            "reason": "interrupted",
        }))?;

        match event {
            EventMsg::TurnAborted(TurnAbortedEvent {
                turn_id, reason, ..
            }) => {
                assert_eq!(turn_id, None);
                assert_eq!(reason, TurnAbortReason::Interrupted);
            }
            _ => panic!("expected turn_aborted event"),
        }

        Ok(())
    }

    #[test]
    fn turn_context_item_deserializes_without_network() -> Result<()> {
        let item: TurnContextItem = serde_json::from_value(json!({
            "cwd": test_path_buf("/tmp"),
            "approval_policy": "never",
            "sandbox_policy": { "type": "danger-full-access" },
            "model": "gpt-5",
            "summary": "auto",
        }))?;

        assert_eq!(item.trace_id, None);
        assert_eq!(item.network, None);
        assert_eq!(item.file_system_sandbox_policy, None);
        assert_eq!(item.comp_hash, None);
        Ok(())
    }

    #[test]
    fn multi_agent_version_uses_newest_present_session_meta_value() -> Result<()> {
        let thread_id = ThreadId::from_string("67e55044-10b1-426f-9247-bb680e5fe0c8")?;
        let older_meta = SessionMetaLine {
            meta: SessionMeta {
                id: thread_id,
                multi_agent_version: Some(MultiAgentVersion::V2),
                ..Default::default()
            },
            git: None,
        };
        let newer_meta_without_version = SessionMetaLine {
            meta: SessionMeta {
                id: thread_id,
                multi_agent_version: None,
                ..Default::default()
            },
            git: None,
        };

        assert_eq!(
            multi_agent_version_from_items(
                &[
                    RolloutItem::SessionMeta(older_meta),
                    RolloutItem::SessionMeta(newer_meta_without_version),
                ],
                Some(thread_id),
            ),
            Some(MultiAgentVersion::V2)
        );
        Ok(())
    }

    #[test]
    fn latest_effective_multi_agent_mode_uses_latest_turn_context_even_when_unset() -> Result<()> {
        let turn_context_item = |multi_agent_mode| -> Result<RolloutItem> {
            let mut value = json!({
                "cwd": test_path_buf("/tmp"),
                "approval_policy": "never",
                "sandbox_policy": { "type": "danger-full-access" },
                "model": "gpt-5",
                "summary": "auto",
            });
            value["multi_agent_mode"] = serde_json::to_value(multi_agent_mode)?;
            Ok(RolloutItem::TurnContext(serde_json::from_value(value)?))
        };

        assert_eq!(
            InitialHistory::Forked(vec![
                turn_context_item(Some(MultiAgentMode::Proactive))?,
                turn_context_item(/*multi_agent_mode*/ None)?,
            ])
            .get_latest_effective_multi_agent_mode(),
            None
        );
        Ok(())
    }

    #[test]
    fn turn_context_item_serializes_network_when_present() -> Result<()> {
        let item = TurnContextItem {
            turn_id: None,
            trace_id: None,
            cwd: test_path_buf("/tmp").abs(),
            workspace_roots: None,
            current_date: None,
            timezone: None,
            approval_policy: AskForApproval::Never,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            permission_profile: None,
            network: Some(TurnContextNetworkItem {
                allowed_domains: vec!["api.example.com".to_string()],
                denied_domains: vec!["blocked.example.com".to_string()],
            }),
            file_system_sandbox_policy: Some(FileSystemSandboxPolicy::restricted(vec![
                FileSystemSandboxEntry {
                    path: FileSystemPath::GlobPattern {
                        pattern: "/tmp/private/**/*.txt".to_string(),
                    },
                    access: FileSystemAccessMode::None,
                },
            ])),
            model: "gpt-5".to_string(),
            comp_hash: None,
            personality: None,
            collaboration_mode: None,
            multi_agent_version: None,
            multi_agent_mode: None,
            realtime_active: None,
            effort: None,
            summary: ReasoningSummaryConfig::Auto,
            user_instructions: None,
            developer_instructions: None,
            final_output_json_schema: None,
            truncation_policy: None,
        };

        let value = serde_json::to_value(item)?;
        assert_eq!(
            value["network"],
            json!({
                "allowed_domains": ["api.example.com"],
                "denied_domains": ["blocked.example.com"],
            })
        );
        assert_eq!(
            value["file_system_sandbox_policy"],
            json!({
                "kind": "restricted",
                "entries": [{
                    "path": {
                        "type": "glob_pattern",
                        "pattern": "/tmp/private/**/*.txt"
                    },
                    "access": "none"
                }]
            })
        );
        assert_eq!(value["summary"], json!("auto"));
        Ok(())
    }

    /// Serialize Event to verify that its JSON representation has the expected
    /// amount of nesting.
    #[test]
    fn serialize_event() -> Result<()> {
        let session_id = SessionId::from_string("67e55044-10b1-426f-9247-bb680e5fe0c7")?;
        let thread_id = ThreadId::from_string("67e55044-10b1-426f-9247-bb680e5fe0c8")?;
        let rollout_file = NamedTempFile::new()?;
        let permission_profile = PermissionProfile::read_only();
        let event = Event {
            id: "1234".to_string(),
            msg: EventMsg::SessionConfigured(SessionConfiguredEvent {
                session_id,
                thread_id,
                forked_from_id: None,
                parent_thread_id: None,
                thread_source: None,
                thread_name: None,
                model: "codex-mini-latest".to_string(),
                model_provider_id: "openai".to_string(),
                service_tier: None,
                approval_policy: AskForApproval::Never,
                approvals_reviewer: ApprovalsReviewer::User,
                permission_profile: permission_profile.clone(),
                active_permission_profile: None,
                cwd: test_path_buf("/home/user/project").abs(),
                reasoning_effort: Some(ReasoningEffortConfig::default()),
                initial_messages: None,
                network_proxy: None,
                rollout_path: Some(rollout_file.path().to_path_buf()),
            }),
        };

        let expected = json!({
            "id": "1234",
            "msg": {
                "type": "session_configured",
                "session_id": "67e55044-10b1-426f-9247-bb680e5fe0c7",
                "thread_id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
                "model": "codex-mini-latest",
                "model_provider_id": "openai",
                "approval_policy": "never",
                "approvals_reviewer": "user",
                "permission_profile": permission_profile,
                "cwd": test_path_buf("/home/user/project"),
                "reasoning_effort": "medium",
                "rollout_path": format!("{}", rollout_file.path().display()),
            }
        });
        assert_eq!(expected, serde_json::to_value(&event)?);
        Ok(())
    }

    #[test]
    fn deserialize_legacy_session_configured_event_uses_sandbox_policy() -> Result<()> {
        let cwd = test_path_buf("/home/user/project");
        let value = json!({
            "session_id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
            "model": "codex-mini-latest",
            "model_provider_id": "openai",
            "approval_policy": "never",
            "approvals_reviewer": "user",
            "sandbox_policy": {
                "type": "read-only"
            },
            "cwd": cwd,
        });

        let event: SessionConfiguredEvent = serde_json::from_value(value)?;
        assert_eq!(event.permission_profile, PermissionProfile::read_only());
        Ok(())
    }

    #[test]
    fn vec_u8_as_base64_serialization_and_deserialization() -> Result<()> {
        let event = ExecCommandOutputDeltaEvent {
            call_id: "call21".to_string(),
            stream: ExecOutputStream::Stdout,
            chunk: vec![1, 2, 3, 4, 5],
        };
        let serialized = serde_json::to_string(&event)?;
        assert_eq!(
            r#"{"call_id":"call21","stream":"stdout","chunk":"AQIDBAU="}"#,
            serialized,
        );

        let deserialized: ExecCommandOutputDeltaEvent = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, event);
        Ok(())
    }
}
