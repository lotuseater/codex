//! App-server-facing analytics event-request builders and the wire-shaped
//! `TrackEventRequest` enum. Moved out of `codex-analytics` so that the lower
//! crate stays free of `codex-app-server-protocol`.
//!
//! Several protocol-free building blocks (the fact enums/structs, the
//! `AppServerRpcTransport` transport tag, and the `GuardianReviewEventParams`
//! payload) are re-used from `codex_analytics`. Everything that consumes
//! `codex_app_server_protocol` lives here.

use crate::accepted_lines::AcceptedLineFingerprintEventInput;
use codex_analytics::AcceptedLineFingerprint;
use codex_analytics::AppServerRpcTransport;
use codex_analytics::CodexCompactionEvent;
use codex_analytics::CodexErrKind;
use codex_analytics::CompactionImplementation;
use codex_analytics::CompactionPhase;
use codex_analytics::CompactionReason;
use codex_analytics::CompactionStatus;
use codex_analytics::CompactionStrategy;
use codex_analytics::CompactionTrigger;
use codex_analytics::GuardianReviewEventParams;
use codex_analytics::ThreadInitializationMode;
use codex_analytics::TurnStatus;
use codex_analytics::TurnSteerRejectionReason;
use codex_analytics::TurnSteerResult;
use codex_analytics::TurnSubmissionType;
use codex_app_server_protocol::CodexErrorInfo;
use codex_app_server_protocol::CommandExecutionSource;
use codex_protocol::protocol::ThreadSource;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TrackEventsRequest {
    pub(crate) events: Vec<TrackEventRequest>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum TrackEventRequest {
    SkillInvocation(SkillInvocationEventRequest),
    ThreadInitialized(ThreadInitializedEvent),
    GuardianReview(Box<GuardianReviewEventRequest>),
    AppMentioned(CodexAppMentionedEventRequest),
    AppUsed(CodexAppUsedEventRequest),
    HookRun(CodexHookRunEventRequest),
    Compaction(Box<CodexCompactionEventRequest>),
    TurnEvent(Box<CodexTurnEventRequest>),
    TurnSteer(CodexTurnSteerEventRequest),
    CommandExecution(CodexCommandExecutionEventRequest),
    FileChange(CodexFileChangeEventRequest),
    McpToolCall(CodexMcpToolCallEventRequest),
    DynamicToolCall(CodexDynamicToolCallEventRequest),
    CollabAgentToolCall(CodexCollabAgentToolCallEventRequest),
    WebSearch(CodexWebSearchEventRequest),
    ImageGeneration(CodexImageGenerationEventRequest),
    AcceptedLineFingerprints(Box<CodexAcceptedLineFingerprintsEventRequest>),
    #[allow(dead_code)]
    ReviewEvent(CodexReviewEventRequest),
    PluginUsed(CodexPluginUsedEventRequest),
    PluginInstalled(CodexPluginEventRequest),
    PluginUninstalled(CodexPluginEventRequest),
    PluginEnabled(CodexPluginEventRequest),
    PluginDisabled(CodexPluginEventRequest),
}

impl TrackEventRequest {
    pub(crate) fn should_send_in_isolated_request(&self) -> bool {
        matches!(self, Self::AcceptedLineFingerprints(_))
    }
}

#[derive(Serialize)]
pub(crate) struct CodexAcceptedLineFingerprintsEventParams {
    pub(crate) event_type: &'static str,
    pub(crate) turn_id: String,
    pub(crate) thread_id: String,
    pub(crate) product_surface: Option<String>,
    pub(crate) model_slug: Option<String>,
    pub(crate) completed_at: u64,
    pub(crate) repo_hash: Option<String>,
    pub(crate) accepted_added_lines: u64,
    pub(crate) accepted_deleted_lines: u64,
    pub(crate) line_fingerprints: Vec<AcceptedLineFingerprint>,
}

#[derive(Serialize)]
pub(crate) struct CodexAcceptedLineFingerprintsEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexAcceptedLineFingerprintsEventParams,
}

pub(crate) fn accepted_line_fingerprint_event_requests(
    input: AcceptedLineFingerprintEventInput,
) -> Vec<TrackEventRequest> {
    let AcceptedLineFingerprintEventInput {
        event_type,
        turn_id,
        thread_id,
        product_surface,
        model_slug,
        completed_at,
        repo_hash,
        accepted_added_lines,
        accepted_deleted_lines,
        line_fingerprints: _line_fingerprints,
    } = input;

    vec![TrackEventRequest::AcceptedLineFingerprints(Box::new(
        CodexAcceptedLineFingerprintsEventRequest {
            event_type: "codex_accepted_line_fingerprints",
            event_params: CodexAcceptedLineFingerprintsEventParams {
                event_type,
                turn_id,
                thread_id,
                product_surface,
                model_slug,
                completed_at,
                repo_hash,
                accepted_added_lines,
                accepted_deleted_lines,
                // Keep computing local fingerprints for parsing tests and future attribution,
                // but do not upload path/line hashes in the analytics event payload.
                line_fingerprints: Vec::new(),
            },
        },
    ))]
}

#[derive(Serialize)]
pub(crate) struct SkillInvocationEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) skill_id: String,
    pub(crate) skill_name: String,
    pub(crate) event_params: SkillInvocationEventParams,
}

#[derive(Serialize)]
pub(crate) struct SkillInvocationEventParams {
    pub(crate) product_client_id: Option<String>,
    pub(crate) skill_scope: Option<String>,
    pub(crate) plugin_id: Option<String>,
    pub(crate) repo_url: Option<String>,
    pub(crate) thread_id: Option<String>,
    pub(crate) turn_id: Option<String>,
    pub(crate) invoke_type: Option<InvocationType>,
    pub(crate) model_slug: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum InvocationType {
    Explicit,
    Implicit,
}

#[derive(Clone, Serialize)]
pub(crate) struct CodexAppServerClientMetadata {
    pub(crate) product_client_id: String,
    pub(crate) client_name: Option<String>,
    pub(crate) client_version: Option<String>,
    pub(crate) rpc_transport: AppServerRpcTransport,
    pub(crate) experimental_api_enabled: Option<bool>,
}

#[derive(Clone, Serialize)]
pub(crate) struct CodexRuntimeMetadata {
    pub(crate) codex_rs_version: String,
    pub(crate) runtime_os: String,
    pub(crate) runtime_os_version: String,
    pub(crate) runtime_arch: String,
}

#[derive(Serialize)]
pub(crate) struct ThreadInitializedEventParams {
    pub(crate) thread_id: String,
    pub(crate) session_id: String,
    pub(crate) app_server_client: CodexAppServerClientMetadata,
    pub(crate) runtime: CodexRuntimeMetadata,
    pub(crate) model: String,
    pub(crate) ephemeral: bool,
    pub(crate) thread_source: Option<ThreadSource>,
    pub(crate) initialization_mode: ThreadInitializationMode,
    pub(crate) subagent_source: Option<String>,
    pub(crate) parent_thread_id: Option<String>,
    pub(crate) created_at: u64,
}

#[derive(Serialize)]
pub(crate) struct ThreadInitializedEvent {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: ThreadInitializedEventParams,
}

#[derive(Serialize)]
pub(crate) struct GuardianReviewEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: GuardianReviewEventPayload,
}

#[derive(Serialize)]
pub(crate) struct GuardianReviewEventPayload {
    pub(crate) session_id: String,
    pub(crate) app_server_client: CodexAppServerClientMetadata,
    pub(crate) runtime: CodexRuntimeMetadata,
    #[serde(flatten)]
    pub(crate) guardian_review: GuardianReviewEventParams,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FinalApprovalOutcome {
    Unknown,
    NotNeeded,
    ConfigAllowed,
    PolicyForbidden,
    GuardianApproved,
    GuardianDenied,
    GuardianAborted,
    UserApproved,
    UserApprovedForSession,
    UserDenied,
    UserAborted,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToolItemTerminalStatus {
    Completed,
    Failed,
    Rejected,
    Interrupted,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToolItemFailureKind {
    ToolError,
    ApprovalDenied,
    ApprovalAborted,
    SandboxDenied,
    PolicyForbidden,
}

#[derive(Serialize)]
pub(crate) struct CodexToolItemEventBase {
    pub(crate) thread_id: String,
    pub(crate) turn_id: String,
    /// App-server ThreadItem.id. For tool-originated items this generally
    /// corresponds to the originating core call_id.
    pub(crate) item_id: String,
    pub(crate) app_server_client: CodexAppServerClientMetadata,
    pub(crate) runtime: CodexRuntimeMetadata,
    pub(crate) thread_source: Option<ThreadSource>,
    pub(crate) subagent_source: Option<String>,
    pub(crate) parent_thread_id: Option<String>,
    pub(crate) tool_name: String,
    pub(crate) started_at_ms: u64,
    pub(crate) completed_at_ms: u64,
    // Observed item lifecycle duration. This may undercount end-to-end execution
    // for tools where app-server only sees part of the upstream flow.
    pub(crate) duration_ms: Option<u64>,
    pub(crate) execution_duration_ms: Option<u64>,
    pub(crate) review_count: u64,
    pub(crate) guardian_review_count: u64,
    pub(crate) user_review_count: u64,
    pub(crate) final_approval_outcome: FinalApprovalOutcome,
    pub(crate) terminal_status: ToolItemTerminalStatus,
    pub(crate) failure_kind: Option<ToolItemFailureKind>,
    pub(crate) requested_additional_permissions: bool,
    pub(crate) requested_network_access: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewSubjectKind {
    CommandExecution,
    FileChange,
    McpToolCall,
    Permissions,
    NetworkAccess,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Reviewer {
    Guardian,
    User,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewTrigger {
    Initial,
    SandboxDenial,
    NetworkPolicyDenial,
    ExecveIntercept,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewStatus {
    Approved,
    Denied,
    Aborted,
    TimedOut,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewResolution {
    None,
    SessionApproval,
    ExecPolicyAmendment,
    NetworkPolicyAmendment,
}

#[derive(Serialize)]
pub(crate) struct CodexReviewEventParams {
    pub(crate) thread_id: String,
    pub(crate) turn_id: String,
    pub(crate) item_id: Option<String>,
    pub(crate) review_id: String,
    pub(crate) app_server_client: CodexAppServerClientMetadata,
    pub(crate) runtime: CodexRuntimeMetadata,
    pub(crate) thread_source: Option<ThreadSource>,
    pub(crate) subagent_source: Option<String>,
    pub(crate) parent_thread_id: Option<String>,
    pub(crate) subject_kind: ReviewSubjectKind,
    pub(crate) subject_name: String,
    pub(crate) reviewer: Reviewer,
    pub(crate) trigger: ReviewTrigger,
    pub(crate) status: ReviewStatus,
    pub(crate) resolution: ReviewResolution,
    pub(crate) started_at_ms: u64,
    pub(crate) completed_at_ms: u64,
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct CodexReviewEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexReviewEventParams,
}
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WebSearchActionKind {
    Search,
    OpenPage,
    FindInPage,
    Other,
}

#[derive(Serialize)]
pub(crate) struct CodexCommandExecutionEventParams {
    #[serde(flatten)]
    pub(crate) base: CodexToolItemEventBase,
    pub(crate) command_execution_source: CommandExecutionSource,
    pub(crate) exit_code: Option<i32>,
    pub(crate) command_total_action_count: u64,
    pub(crate) command_read_action_count: u64,
    pub(crate) command_list_files_action_count: u64,
    pub(crate) command_search_action_count: u64,
    pub(crate) command_unknown_action_count: u64,
}

#[derive(Serialize)]
pub(crate) struct CodexCommandExecutionEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexCommandExecutionEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexFileChangeEventParams {
    #[serde(flatten)]
    pub(crate) base: CodexToolItemEventBase,
    pub(crate) file_change_count: u64,
    pub(crate) file_add_count: u64,
    pub(crate) file_update_count: u64,
    pub(crate) file_delete_count: u64,
    pub(crate) file_move_count: u64,
}

#[derive(Serialize)]
pub(crate) struct CodexFileChangeEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexFileChangeEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexMcpToolCallEventParams {
    #[serde(flatten)]
    pub(crate) base: CodexToolItemEventBase,
    pub(crate) mcp_server_name: String,
    pub(crate) mcp_tool_name: String,
    pub(crate) mcp_error_present: bool,
}

#[derive(Serialize)]
pub(crate) struct CodexMcpToolCallEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexMcpToolCallEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexDynamicToolCallEventParams {
    #[serde(flatten)]
    pub(crate) base: CodexToolItemEventBase,
    pub(crate) dynamic_tool_name: String,
    pub(crate) success: Option<bool>,
    pub(crate) output_content_item_count: Option<u64>,
    pub(crate) output_text_item_count: Option<u64>,
    pub(crate) output_image_item_count: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct CodexDynamicToolCallEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexDynamicToolCallEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexCollabAgentToolCallEventParams {
    #[serde(flatten)]
    pub(crate) base: CodexToolItemEventBase,
    pub(crate) sender_thread_id: String,
    pub(crate) receiver_thread_count: u64,
    pub(crate) receiver_thread_ids: Option<Vec<String>>,
    pub(crate) requested_model: Option<String>,
    pub(crate) requested_reasoning_effort: Option<String>,
    pub(crate) agent_state_count: Option<u64>,
    pub(crate) completed_agent_count: Option<u64>,
    pub(crate) failed_agent_count: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct CodexCollabAgentToolCallEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexCollabAgentToolCallEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexWebSearchEventParams {
    #[serde(flatten)]
    pub(crate) base: CodexToolItemEventBase,
    pub(crate) web_search_action: Option<WebSearchActionKind>,
    pub(crate) query_present: bool,
    pub(crate) query_count: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct CodexWebSearchEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexWebSearchEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexImageGenerationEventParams {
    #[serde(flatten)]
    pub(crate) base: CodexToolItemEventBase,
    pub(crate) revised_prompt_present: bool,
    pub(crate) saved_path_present: bool,
}

#[derive(Serialize)]
pub(crate) struct CodexImageGenerationEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexImageGenerationEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexAppMetadata {
    pub(crate) connector_id: Option<String>,
    pub(crate) thread_id: Option<String>,
    pub(crate) turn_id: Option<String>,
    pub(crate) app_name: Option<String>,
    pub(crate) product_client_id: Option<String>,
    pub(crate) invoke_type: Option<InvocationType>,
    pub(crate) model_slug: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct CodexAppMentionedEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexAppMetadata,
}

#[derive(Serialize)]
pub(crate) struct CodexAppUsedEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexAppMetadata,
}

#[derive(Serialize)]
pub(crate) struct CodexHookRunMetadata {
    pub(crate) thread_id: Option<String>,
    pub(crate) turn_id: Option<String>,
    pub(crate) model_slug: Option<String>,
    pub(crate) hook_name: Option<String>,
    pub(crate) hook_source: Option<&'static str>,
    pub(crate) status: Option<codex_protocol::protocol::HookRunStatus>,
}

#[derive(Serialize)]
pub(crate) struct CodexHookRunEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexHookRunMetadata,
}

#[derive(Serialize)]
pub(crate) struct CodexCompactionEventParams {
    pub(crate) thread_id: String,
    pub(crate) session_id: String,
    pub(crate) turn_id: String,
    pub(crate) app_server_client: CodexAppServerClientMetadata,
    pub(crate) runtime: CodexRuntimeMetadata,
    pub(crate) thread_source: Option<ThreadSource>,
    pub(crate) subagent_source: Option<String>,
    pub(crate) parent_thread_id: Option<String>,
    pub(crate) trigger: CompactionTrigger,
    pub(crate) reason: CompactionReason,
    pub(crate) implementation: CompactionImplementation,
    pub(crate) phase: CompactionPhase,
    pub(crate) strategy: CompactionStrategy,
    pub(crate) status: CompactionStatus,
    pub(crate) error: Option<String>,
    pub(crate) active_context_tokens_before: i64,
    pub(crate) active_context_tokens_after: i64,
    pub(crate) started_at: u64,
    pub(crate) completed_at: u64,
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct CodexCompactionEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexCompactionEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexTurnEventParams {
    pub(crate) thread_id: String,
    pub(crate) session_id: String,
    pub(crate) turn_id: String,
    // TODO(rhan-oai): Populate once queued/default submission type is plumbed from
    // the turn/start callsites instead of always being reported as None.
    pub(crate) submission_type: Option<TurnSubmissionType>,
    pub(crate) app_server_client: CodexAppServerClientMetadata,
    pub(crate) runtime: CodexRuntimeMetadata,
    pub(crate) ephemeral: bool,
    pub(crate) thread_source: Option<ThreadSource>,
    pub(crate) initialization_mode: ThreadInitializationMode,
    pub(crate) subagent_source: Option<String>,
    pub(crate) parent_thread_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) model_provider: String,
    pub(crate) sandbox_policy: Option<&'static str>,
    pub(crate) reasoning_effort: Option<String>,
    pub(crate) reasoning_summary: Option<String>,
    pub(crate) service_tier: String,
    pub(crate) approval_policy: String,
    pub(crate) approvals_reviewer: String,
    pub(crate) sandbox_network_access: bool,
    pub(crate) collaboration_mode: Option<&'static str>,
    pub(crate) personality: Option<String>,
    pub(crate) workspace_kind: Option<String>,
    pub(crate) num_input_images: usize,
    pub(crate) is_first_turn: bool,
    pub(crate) status: Option<TurnStatus>,
    pub(crate) turn_error: Option<CodexErrorInfo>,
    pub(crate) codex_error_kind: Option<CodexErrKind>,
    pub(crate) codex_error_subreason: Option<String>,
    pub(crate) codex_error_http_status_code: Option<u16>,
    pub(crate) steer_count: Option<usize>,
    pub(crate) total_tool_call_count: Option<usize>,
    pub(crate) shell_command_count: Option<usize>,
    pub(crate) file_change_count: Option<usize>,
    pub(crate) mcp_tool_call_count: Option<usize>,
    pub(crate) dynamic_tool_call_count: Option<usize>,
    pub(crate) subagent_tool_call_count: Option<usize>,
    pub(crate) web_search_count: Option<usize>,
    pub(crate) image_generation_count: Option<usize>,
    pub(crate) input_tokens: Option<i64>,
    pub(crate) cached_input_tokens: Option<i64>,
    pub(crate) output_tokens: Option<i64>,
    pub(crate) reasoning_output_tokens: Option<i64>,
    pub(crate) total_tokens: Option<i64>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) started_at: Option<u64>,
    pub(crate) completed_at: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct CodexTurnEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexTurnEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexTurnSteerEventParams {
    pub(crate) thread_id: String,
    pub(crate) session_id: String,
    pub(crate) expected_turn_id: Option<String>,
    pub(crate) accepted_turn_id: Option<String>,
    pub(crate) app_server_client: CodexAppServerClientMetadata,
    pub(crate) runtime: CodexRuntimeMetadata,
    pub(crate) thread_source: Option<ThreadSource>,
    pub(crate) subagent_source: Option<String>,
    pub(crate) parent_thread_id: Option<String>,
    pub(crate) num_input_images: usize,
    pub(crate) result: TurnSteerResult,
    pub(crate) rejection_reason: Option<TurnSteerRejectionReason>,
    pub(crate) created_at: u64,
}

#[derive(Serialize)]
pub(crate) struct CodexTurnSteerEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexTurnSteerEventParams,
}

#[derive(Serialize)]
pub(crate) struct CodexPluginMetadata {
    pub(crate) plugin_id: Option<String>,
    pub(crate) plugin_name: Option<String>,
    pub(crate) marketplace_name: Option<String>,
    pub(crate) has_skills: Option<bool>,
    pub(crate) mcp_server_count: Option<usize>,
    pub(crate) connector_ids: Option<Vec<String>>,
    pub(crate) product_client_id: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct CodexPluginUsedMetadata {
    #[serde(flatten)]
    pub(crate) plugin: CodexPluginMetadata,
    pub(crate) thread_id: Option<String>,
    pub(crate) turn_id: Option<String>,
    pub(crate) model_slug: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct CodexPluginEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexPluginMetadata,
}

#[derive(Serialize)]
pub(crate) struct CodexPluginUsedEventRequest {
    pub(crate) event_type: &'static str,
    pub(crate) event_params: CodexPluginUsedMetadata,
}

pub(crate) fn current_runtime_metadata() -> CodexRuntimeMetadata {
    let os_info = os_info::get();
    CodexRuntimeMetadata {
        codex_rs_version: env!("CARGO_PKG_VERSION").to_string(),
        runtime_os: std::env::consts::OS.to_string(),
        runtime_os_version: os_info.version().to_string(),
        runtime_arch: std::env::consts::ARCH.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Test-only event-request builders.
//
// These mirror the custom-fact → event-request construction that production
// performs in `codex-analytics`'s `CustomFactReducer` (which emits the opaque
// `TrackEvent` over the protocol-free seam). The app-server test suite drives
// the reducer at the `Vec<TrackEventRequest>` level and serializes that, so it
// needs the structured request values rather than the opaque `TrackEvent`.
// Kept `#[cfg(test)]` so they add no production surface and never re-couple the
// lower crate's custom-event building back into this crate.
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) fn plugin_state_event_type(state: codex_analytics::PluginState) -> &'static str {
    use codex_analytics::PluginState;
    match state {
        PluginState::Installed => "codex_plugin_installed",
        PluginState::Uninstalled => "codex_plugin_uninstalled",
        PluginState::Enabled => "codex_plugin_enabled",
        PluginState::Disabled => "codex_plugin_disabled",
    }
}

#[cfg(test)]
pub(crate) fn codex_app_metadata(
    tracking: &codex_analytics::TrackEventsContext,
    app: codex_analytics::AppInvocation,
) -> CodexAppMetadata {
    CodexAppMetadata {
        connector_id: app.connector_id,
        thread_id: Some(tracking.thread_id.clone()),
        turn_id: Some(tracking.turn_id.clone()),
        app_name: app.app_name,
        product_client_id: Some(codex_login::default_client::originator().value),
        invoke_type: match app.invocation_type {
            Some(codex_analytics::InvocationType::Explicit) => {
                Some(crate::events::InvocationType::Explicit)
            }
            Some(codex_analytics::InvocationType::Implicit) => {
                Some(crate::events::InvocationType::Implicit)
            }
            None => None,
        },
        model_slug: Some(tracking.model_slug.clone()),
    }
}

#[cfg(test)]
pub(crate) fn codex_plugin_metadata(
    plugin: codex_plugin::PluginTelemetryMetadata,
) -> CodexPluginMetadata {
    let codex_plugin::PluginTelemetryMetadata {
        plugin_id,
        remote_plugin_id,
        capability_summary,
    } = plugin;
    let event_plugin_id = remote_plugin_id.unwrap_or_else(|| plugin_id.as_key());
    CodexPluginMetadata {
        plugin_id: Some(event_plugin_id),
        plugin_name: Some(plugin_id.plugin_name),
        marketplace_name: Some(plugin_id.marketplace_name),
        has_skills: capability_summary
            .as_ref()
            .map(|summary| summary.has_skills),
        mcp_server_count: capability_summary
            .as_ref()
            .map(|summary| summary.mcp_server_names.len()),
        connector_ids: capability_summary.map(|summary| {
            summary
                .app_connector_ids
                .into_iter()
                .map(|connector_id| connector_id.0)
                .collect()
        }),
        product_client_id: Some(codex_login::default_client::originator().value),
    }
}

#[cfg(test)]
pub(crate) fn codex_plugin_used_metadata(
    tracking: &codex_analytics::TrackEventsContext,
    plugin: codex_plugin::PluginTelemetryMetadata,
) -> CodexPluginUsedMetadata {
    CodexPluginUsedMetadata {
        plugin: codex_plugin_metadata(plugin),
        thread_id: Some(tracking.thread_id.clone()),
        turn_id: Some(tracking.turn_id.clone()),
        model_slug: Some(tracking.model_slug.clone()),
    }
}

#[cfg(test)]
pub(crate) fn codex_hook_run_metadata(
    tracking: &codex_analytics::TrackEventsContext,
    hook: codex_analytics::HookRunFact,
) -> CodexHookRunMetadata {
    use codex_protocol::protocol::HookEventName;
    use codex_protocol::protocol::HookRunStatus;
    use codex_protocol::protocol::HookSource;

    fn analytics_hook_event_name(event_name: HookEventName) -> &'static str {
        match event_name {
            HookEventName::PreToolUse => "PreToolUse",
            HookEventName::PermissionRequest => "PermissionRequest",
            HookEventName::PostToolUse => "PostToolUse",
            HookEventName::PreCompact => "PreCompact",
            HookEventName::PostCompact => "PostCompact",
            HookEventName::SessionStart => "SessionStart",
            HookEventName::UserPromptSubmit => "UserPromptSubmit",
            HookEventName::SubagentStart => "SubagentStart",
            HookEventName::SubagentStop => "SubagentStop",
            HookEventName::Stop => "Stop",
        }
    }

    fn analytics_hook_source(source: HookSource) -> &'static str {
        match source {
            HookSource::System => "system",
            HookSource::User => "user",
            HookSource::Project => "project",
            HookSource::Mdm => "mdm",
            HookSource::SessionFlags => "session_flags",
            HookSource::Plugin => "plugin",
            HookSource::CloudRequirements => "cloud_requirements",
            HookSource::CloudManagedConfig => "cloud_managed_config",
            HookSource::LegacyManagedConfigFile => "legacy_managed_config_file",
            HookSource::LegacyManagedConfigMdm => "legacy_managed_config_mdm",
            HookSource::Unknown => "unknown",
        }
    }

    fn analytics_hook_status(status: HookRunStatus) -> HookRunStatus {
        match status {
            // Running is unexpected here and normalized defensively.
            HookRunStatus::Running => HookRunStatus::Failed,
            other => other,
        }
    }

    CodexHookRunMetadata {
        thread_id: Some(tracking.thread_id.clone()),
        turn_id: Some(tracking.turn_id.clone()),
        model_slug: Some(tracking.model_slug.clone()),
        hook_name: Some(analytics_hook_event_name(hook.event_name).to_owned()),
        hook_source: Some(analytics_hook_source(hook.hook_source)),
        status: Some(analytics_hook_status(hook.status)),
    }
}

#[cfg(test)]
pub(crate) fn subagent_thread_started_event_request(
    input: codex_analytics::SubAgentThreadStartedInput,
) -> ThreadInitializedEvent {
    use codex_protocol::protocol::SubAgentSource;
    use codex_protocol::protocol::ThreadSource;

    fn subagent_source_name(subagent_source: &SubAgentSource) -> String {
        match subagent_source {
            SubAgentSource::Review => "review".to_string(),
            SubAgentSource::Compact => "compact".to_string(),
            SubAgentSource::ThreadSpawn { .. } => "thread_spawn".to_string(),
            SubAgentSource::MemoryConsolidation => "memory_consolidation".to_string(),
            SubAgentSource::Other(other) => other.clone(),
        }
    }

    fn subagent_parent_thread_id(subagent_source: &SubAgentSource) -> Option<String> {
        match subagent_source {
            SubAgentSource::ThreadSpawn {
                parent_thread_id, ..
            } => Some(parent_thread_id.to_string()),
            _ => None,
        }
    }

    let event_params = ThreadInitializedEventParams {
        thread_id: input.thread_id,
        app_server_client: CodexAppServerClientMetadata {
            product_client_id: input.product_client_id,
            client_name: Some(input.client_name),
            client_version: Some(input.client_version),
            rpc_transport: AppServerRpcTransport::InProcess,
            experimental_api_enabled: None,
        },
        runtime: current_runtime_metadata(),
        model: input.model,
        ephemeral: input.ephemeral,
        thread_source: Some(ThreadSource::Subagent),
        initialization_mode: ThreadInitializationMode::New,
        subagent_source: Some(subagent_source_name(&input.subagent_source)),
        parent_thread_id: input
            .parent_thread_id
            .or_else(|| subagent_parent_thread_id(&input.subagent_source)),
        created_at: input.created_at,
    };
    ThreadInitializedEvent {
        event_type: "codex_thread_initialized",
        event_params,
    }
}

pub(crate) fn codex_compaction_event_params(
    input: CodexCompactionEvent,
    session_id: String,
    app_server_client: CodexAppServerClientMetadata,
    runtime: CodexRuntimeMetadata,
    thread_source: Option<ThreadSource>,
    subagent_source: Option<String>,
    parent_thread_id: Option<String>,
) -> CodexCompactionEventParams {
    CodexCompactionEventParams {
        thread_id: input.thread_id,
        session_id,
        turn_id: input.turn_id,
        app_server_client,
        runtime,
        thread_source,
        subagent_source,
        parent_thread_id,
        trigger: input.trigger,
        reason: input.reason,
        implementation: input.implementation,
        phase: input.phase,
        strategy: input.strategy,
        status: input.status,
        error: input.error,
        active_context_tokens_before: input.active_context_tokens_before,
        active_context_tokens_after: input.active_context_tokens_after,
        started_at: input.started_at,
        completed_at: input.completed_at,
        duration_ms: input.duration_ms,
    }
}
