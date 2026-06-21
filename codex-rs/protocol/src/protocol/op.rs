use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use codex_config_types::ContextBudgetMode;
use codex_utils_absolute_path::AbsolutePathBuf;

use crate::config_types::ApprovalsReviewer;
use crate::config_types::CollaborationMode;
use crate::config_types::Personality;
use crate::config_types::ReasoningSummary as ReasoningSummaryConfig;
use crate::config_types::WindowsSandboxLevel;
use crate::dynamic_tools::DynamicToolResponse;
use crate::mcp::RequestId;
use crate::models::ActivePermissionProfile;
use crate::models::PermissionProfile;
use crate::openai_models::ReasoningEffort as ReasoningEffortConfig;
use crate::request_permissions::RequestPermissionsResponse;
use crate::request_user_input::RequestUserInputResponse;
use crate::user_input::UserInput;

use super::*;

/// Submission operation
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
pub enum Op {
    /// Abort current task without terminating background terminal processes.
    /// This server sends [`EventMsg::TurnAborted`] in response.
    Interrupt,

    /// Terminate all running background terminal processes for this thread.
    /// Use this when callers intentionally want to stop long-lived background shells.
    CleanBackgroundTerminals,

    /// Start a realtime conversation stream.
    RealtimeConversationStart(ConversationStartParams),

    /// Send audio input to the running realtime conversation stream.
    RealtimeConversationAudio(ConversationAudioParams),

    /// Send text input to the running realtime conversation stream.
    RealtimeConversationText(ConversationTextParams),

    /// Append speakable text to the running realtime conversation stream.
    RealtimeConversationSpeech(ConversationSpeechParams),

    /// Close the running realtime conversation stream.
    RealtimeConversationClose,

    /// Request the list of voices supported by realtime conversation streams.
    RealtimeConversationListVoices,

    /// Legacy user input.
    ///
    /// Prefer [`Op::UserTurn`] so the caller provides full turn context
    /// (cwd/approval/sandbox/model/etc.) for each turn.
    UserInput {
        /// User input items, see `InputItem`
        items: Vec<UserInput>,
        /// Optional turn-scoped environments.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environments: Option<Vec<TurnEnvironmentSelection>>,
        /// Optional JSON Schema used to constrain the final assistant message for this turn.
        #[serde(skip_serializing_if = "Option::is_none")]
        final_output_json_schema: Option<Value>,
        /// Optional turn-scoped Responses API `client_metadata`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        responsesapi_client_metadata: Option<HashMap<String, String>>,
        /// Client-supplied context fragments keyed by an opaque source identifier.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        additional_context: BTreeMap<String, AdditionalContextEntry>,
        /// Persistent thread-settings overrides to apply before the input.
        #[serde(default, flatten)]
        thread_settings: ThreadSettingsOverrides,
    },

    /// Apply persistent thread-settings overrides without starting a turn.
    ThreadSettings {
        #[serde(flatten)]
        thread_settings: ThreadSettingsOverrides,
    },

    /// Similar to [`Op::UserInput`], but first applies persistent turn-context
    /// overrides in the same queued operation. This preserves submission order
    /// and prevents the input from starting if the overrides are rejected.
    UserInputWithTurnContext {
        /// User input items, see `InputItem`
        items: Vec<UserInput>,
        /// Optional turn-scoped environment selections.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environments: Option<Vec<TurnEnvironmentSelection>>,
        /// Optional JSON Schema used to constrain the final assistant message for this turn.
        #[serde(skip_serializing_if = "Option::is_none")]
        final_output_json_schema: Option<Value>,
        /// Optional turn-scoped Responses API `client_metadata`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        responsesapi_client_metadata: Option<HashMap<String, String>>,

        /// Updated `cwd` for sandbox/tool calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<PathBuf>,

        /// Updated runtime workspace roots for sandbox/tool calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        workspace_roots: Option<Vec<AbsolutePathBuf>>,

        /// Updated workspace roots contributed by the selected permission profile.
        #[serde(skip_serializing_if = "Option::is_none")]
        profile_workspace_roots: Option<Vec<AbsolutePathBuf>>,

        /// Updated command approval policy.
        #[serde(skip_serializing_if = "Option::is_none")]
        approval_policy: Option<AskForApproval>,

        /// Updated approval reviewer for future approval prompts.
        #[serde(skip_serializing_if = "Option::is_none")]
        approvals_reviewer: Option<ApprovalsReviewer>,

        /// Updated sandbox policy for tool calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        sandbox_policy: Option<SandboxPolicy>,

        /// Updated permissions profile for tool calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        permission_profile: Option<PermissionProfile>,

        /// Named or built-in profile that produced `permission_profile`, if
        /// the update selected a profile rather than supplying raw
        /// permissions.
        #[serde(skip_serializing_if = "Option::is_none")]
        active_permission_profile: Option<ActivePermissionProfile>,

        /// Updated Windows sandbox mode for tool execution.
        #[serde(skip_serializing_if = "Option::is_none")]
        windows_sandbox_level: Option<WindowsSandboxLevel>,

        /// Updated model slug. When set, the model info is derived
        /// automatically.
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,

        /// Updated reasoning effort (honored only for reasoning-capable models).
        ///
        /// Use `Some(Some(_))` to set a specific effort, `Some(None)` to clear
        /// the effort, or `None` to leave the existing value unchanged.
        #[serde(skip_serializing_if = "Option::is_none")]
        effort: Option<Option<ReasoningEffortConfig>>,

        /// Updated reasoning summary preference (honored only for reasoning-capable models).
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<ReasoningSummaryConfig>,

        /// Updated service tier preference for future turns.
        ///
        /// Use `Some(Some(_))` to set a specific tier, `Some(None)` to clear the
        /// preference, or `None` to leave the existing value unchanged.
        #[serde(skip_serializing_if = "Option::is_none")]
        service_tier: Option<Option<String>>,

        /// Updated local context-budget behavior for future turns.
        #[serde(skip_serializing_if = "Option::is_none")]
        context_budget_mode: Option<ContextBudgetMode>,

        /// EXPERIMENTAL - set a pre-set collaboration mode.
        /// Takes precedence over model, effort, and developer instructions if set.
        #[serde(skip_serializing_if = "Option::is_none")]
        collaboration_mode: Option<CollaborationMode>,

        /// Updated personality preference.
        #[serde(skip_serializing_if = "Option::is_none")]
        personality: Option<Personality>,
    },

    /// Similar to [`Op::UserInput`], but contains additional context required
    /// for a turn of a [`crate::codex_thread::CodexThread`].
    UserTurn {
        /// User input items, see `InputItem`
        items: Vec<UserInput>,

        /// `cwd` to use with the [`SandboxPolicy`] and potentially tool calls
        /// such as `local_shell`.
        cwd: PathBuf,

        /// Policy to use for command approval.
        approval_policy: AskForApproval,

        /// Reviewer to use for approval requests raised during this turn.
        ///
        /// When omitted, the session keeps the current setting
        approvals_reviewer: Option<ApprovalsReviewer>,

        /// Policy to use for tool calls such as `local_shell`.
        sandbox_policy: SandboxPolicy,

        /// Full permissions profile to use for tool calls such as `local_shell`.
        ///
        /// When omitted, `sandbox_policy` is used as a legacy compatibility
        /// projection.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        permission_profile: Option<PermissionProfile>,

        /// Must be a valid model slug for the configured client session
        /// associated with this conversation.
        model: String,

        /// Will only be honored if the model is configured to use reasoning.
        #[serde(skip_serializing_if = "Option::is_none")]
        effort: Option<ReasoningEffortConfig>,

        /// Will only be honored if the model is configured to use reasoning.
        ///
        /// When omitted, the session keeps the current setting (which allows core to
        /// fall back to the selected model's default on new sessions).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<ReasoningSummaryConfig>,

        /// Optional service tier override for this turn.
        ///
        /// Use `Some(Some(_))` to set a specific tier for this turn, `Some(None)` to
        /// explicitly clear the tier for this turn, or `None` to keep the existing
        /// session preference.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_tier: Option<Option<String>>,

        /// Optional local context-budget behavior override for this turn.
        ///
        /// When omitted, the session keeps the current setting.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_budget_mode: Option<ContextBudgetMode>,

        // The JSON schema to use for the final assistant message
        final_output_json_schema: Option<Value>,

        /// EXPERIMENTAL - set a pre-set collaboration mode.
        /// Takes precedence over model, effort, and developer instructions if set.
        #[serde(skip_serializing_if = "Option::is_none")]
        collaboration_mode: Option<CollaborationMode>,

        /// Optional personality override for this turn.
        #[serde(skip_serializing_if = "Option::is_none")]
        personality: Option<Personality>,

        /// Optional turn-scoped environments.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environments: Option<Vec<TurnEnvironmentSelection>>,
    },

    /// Inter-agent communication that should be recorded as assistant history
    /// while still using the normal thread submission lifecycle.
    InterAgentCommunication {
        communication: InterAgentCommunication,
    },

    /// Override parts of the persistent turn context for subsequent turns.
    ///
    /// All fields are optional; when omitted, the existing value is preserved.
    /// This does not enqueue any input – it only updates defaults used for
    /// turns that rely on persistent session-level context (for example,
    /// [`Op::UserInput`]).
    OverrideTurnContext {
        /// Updated `cwd` for sandbox/tool calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<PathBuf>,

        /// Updated command approval policy.
        #[serde(skip_serializing_if = "Option::is_none")]
        approval_policy: Option<AskForApproval>,

        /// Updated approval reviewer for future approval prompts.
        #[serde(skip_serializing_if = "Option::is_none")]
        approvals_reviewer: Option<ApprovalsReviewer>,

        /// Updated sandbox policy for tool calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        sandbox_policy: Option<SandboxPolicy>,

        /// Updated permissions profile for tool calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        permission_profile: Option<PermissionProfile>,

        /// Updated Windows sandbox mode for tool execution.
        #[serde(skip_serializing_if = "Option::is_none")]
        windows_sandbox_level: Option<WindowsSandboxLevel>,

        /// Updated model slug. When set, the model info is derived
        /// automatically.
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,

        /// Updated reasoning effort (honored only for reasoning-capable models).
        ///
        /// Use `Some(Some(_))` to set a specific effort, `Some(None)` to clear
        /// the effort, or `None` to leave the existing value unchanged.
        #[serde(skip_serializing_if = "Option::is_none")]
        effort: Option<Option<ReasoningEffortConfig>>,

        /// Updated reasoning summary preference (honored only for reasoning-capable models).
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<ReasoningSummaryConfig>,

        /// Updated service tier preference for future turns.
        ///
        /// Use `Some(Some(_))` to set a specific tier, `Some(None)` to clear the
        /// preference, or `None` to leave the existing value unchanged.
        #[serde(skip_serializing_if = "Option::is_none")]
        service_tier: Option<Option<String>>,

        /// Updated local context-budget behavior for future turns.
        #[serde(skip_serializing_if = "Option::is_none")]
        context_budget_mode: Option<ContextBudgetMode>,

        /// EXPERIMENTAL - set a pre-set collaboration mode.
        /// Takes precedence over model, effort, and developer instructions if set.
        #[serde(skip_serializing_if = "Option::is_none")]
        collaboration_mode: Option<CollaborationMode>,

        /// Updated personality preference.
        #[serde(skip_serializing_if = "Option::is_none")]
        personality: Option<Personality>,
    },

    /// Approve a command execution
    ExecApproval {
        /// The id of the submission we are approving
        id: String,
        /// Turn id associated with the approval event, when available.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        turn_id: Option<String>,
        /// The user's decision in response to the request.
        decision: ReviewDecision,
    },

    /// Approve a code patch
    PatchApproval {
        /// The id of the submission we are approving
        id: String,
        /// The user's decision in response to the request.
        decision: ReviewDecision,
    },

    /// Resolve an MCP elicitation request.
    ResolveElicitation {
        /// Name of the MCP server that issued the request.
        server_name: String,
        /// Request identifier from the MCP server.
        request_id: RequestId,
        /// User's decision for the request.
        decision: ElicitationAction,
        /// Structured user input supplied for accepted elicitations.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<Value>,
        /// Optional client metadata associated with the elicitation response.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meta: Option<Value>,
    },

    /// Resolve a request_user_input tool call.
    #[serde(rename = "user_input_answer", alias = "request_user_input_response")]
    UserInputAnswer {
        /// Turn id for the in-flight request.
        id: String,
        /// User-provided answers.
        response: RequestUserInputResponse,
    },

    /// Resolve a request_permissions tool call.
    RequestPermissionsResponse {
        /// Call id for the in-flight request.
        id: String,
        /// User-granted permissions.
        response: RequestPermissionsResponse,
    },

    /// Resolve a dynamic tool call request.
    DynamicToolResponse {
        /// Call id for the in-flight request.
        id: String,
        /// Tool output payload.
        response: DynamicToolResponse,
    },

    /// Request MCP servers to reinitialize and refresh cached tool lists.
    RefreshMcpServers { config: McpServerRefreshConfig },

    /// Reload user config layer overrides for the active session.
    ///
    /// This updates runtime config-derived behavior (for example app
    /// enable/disable state) without restarting the thread.
    ReloadUserConfig,

    /// Request the agent to summarize the current conversation context.
    /// The agent will use its existing context (either conversation history or previous response id)
    /// to generate a summary which will be returned as an AgentMessage event.
    Compact,

    /// Set whether the thread remains eligible for memory generation.
    ///
    /// This persists thread-level memory mode metadata without involving the
    /// model.
    SetThreadMemoryMode { mode: ThreadMemoryMode },

    /// Request Codex to drop the last N user turns from in-memory context.
    ///
    /// This does not attempt to revert local filesystem changes. Clients are
    /// responsible for undoing any edits on disk.
    ThreadRollback { num_turns: u32 },

    /// Request a code review from the agent.
    Review { review_request: ReviewRequest },

    /// Record that the user approved one retry of a concrete Guardian-denied action.
    ApproveGuardianDeniedAction { event: GuardianAssessmentEvent },

    /// Request to shut down codex instance.
    Shutdown,

    /// Execute a user-initiated one-off shell command (triggered by "!cmd").
    ///
    /// The command string is executed using the user's default shell and may
    /// include shell syntax (pipes, redirects, etc.). Output is streamed via
    /// `ExecCommand*` events and the UI regains control upon `TurnComplete`.
    RunUserShellCommand {
        /// The raw command string after '!'
        command: String,
    },
}

impl From<Vec<UserInput>> for Op {
    fn from(value: Vec<UserInput>) -> Self {
        Op::UserInput {
            environments: None,
            items: value,
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides::default(),
        }
    }
}

impl Op {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Interrupt => "interrupt",
            Self::CleanBackgroundTerminals => "clean_background_terminals",
            Self::RealtimeConversationStart(_) => "realtime_conversation_start",
            Self::RealtimeConversationAudio(_) => "realtime_conversation_audio",
            Self::RealtimeConversationText(_) => "realtime_conversation_text",
            Self::RealtimeConversationSpeech(_) => "realtime_conversation_speech",
            Self::RealtimeConversationClose => "realtime_conversation_close",
            Self::RealtimeConversationListVoices => "realtime_conversation_list_voices",
            Self::UserInput { .. } => "user_input",
            Self::ThreadSettings { .. } => "thread_settings",
            Self::UserInputWithTurnContext { .. } => "user_input_with_turn_context",
            Self::UserTurn { .. } => "user_turn",
            Self::InterAgentCommunication { .. } => "inter_agent_communication",
            Self::OverrideTurnContext { .. } => "override_turn_context",
            Self::ExecApproval { .. } => "exec_approval",
            Self::PatchApproval { .. } => "patch_approval",
            Self::ResolveElicitation { .. } => "resolve_elicitation",
            Self::UserInputAnswer { .. } => "user_input_answer",
            Self::RequestPermissionsResponse { .. } => "request_permissions_response",
            Self::DynamicToolResponse { .. } => "dynamic_tool_response",
            Self::RefreshMcpServers { .. } => "refresh_mcp_servers",
            Self::ReloadUserConfig => "reload_user_config",
            Self::Compact => "compact",
            Self::SetThreadMemoryMode { .. } => "set_thread_memory_mode",
            Self::ThreadRollback { .. } => "thread_rollback",
            Self::Review { .. } => "review",
            Self::ApproveGuardianDeniedAction { .. } => "approve_guardian_denied_action",
            Self::Shutdown => "shutdown",
            Self::RunUserShellCommand { .. } => "run_user_shell_command",
        }
    }
}
