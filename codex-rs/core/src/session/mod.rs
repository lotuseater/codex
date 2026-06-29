use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::agent::AgentControl;
use crate::agent::AgentStatus;
use crate::agent::Mailbox;
use crate::agent::MailboxReceiver;
use crate::agent::agent_status_from_event;
use crate::agent::status::is_final;
use crate::agents_md::LoadedAgentsMd;
use crate::attestation::AttestationProvider;
use crate::build_available_skills;
use crate::compact;
use crate::config::ManagedFeatures;
use crate::config::resolve_tool_suggest_config_from_layer_stack;
use crate::connectors;
use crate::context::ApprovedCommandPrefixSaved;
use crate::context::AppsInstructions;
use crate::context::AvailablePluginsInstructions;
use crate::context::AvailableSkillsInstructions;
use crate::context::BatchMiniProgrammingInstructions;
use crate::context::CollaborationModeInstructions;
use crate::context::ContextualUserFragment;
use crate::context::MultiAgentModeInstructions;
use crate::context::NetworkRuleSaved;
use crate::context::PermissionsInstructions;
use crate::context::PersonalitySpecInstructions;
use crate::context::RecommendedPluginsInstructions;
use crate::context_reduction_adapter::compaction_reason_to_context_reduction_reason;
use crate::context_reduction_adapter::semantic_compact_turn_input;
use crate::context_reduction_adapter::token_context_percent_used;
use crate::current_time::TimeProvider;
use crate::default_skill_metadata_budget;
use crate::environment_selection::TurnEnvironmentSnapshot;
use crate::exec_policy::ExecPolicyManager;
use crate::image_preparation::prepare_response_items;
use crate::parse_turn_item;
use crate::realtime_conversation::RealtimeConversationManager;
use crate::session::turn_context::TurnEnvironment;
use crate::session_prefix::format_inter_agent_completion_message;
use crate::skills::SkillRenderSideEffects;
use crate::skills_load_input_from_config;
use crate::turn_metadata::TurnMetadataState;
use crate::turn_timing::now_unix_timestamp_ms;
use async_channel::Receiver;
use async_channel::Sender;
use chrono::Local;
use chrono::Utc;
use codex_analytics::AnalyticsEventsClient;
use codex_analytics::CompactionReason;
use codex_analytics::SubAgentThreadStartedInput;
use codex_analytics::TurnCodexErrorFact;
use codex_config::types::AuthKeyringBackendKind;
use codex_config::types::OAuthCredentialsStoreMode;
use codex_exec_server::Environment;
use codex_exec_server::EnvironmentManager;
use codex_extension_api::ExtensionDataInit;
use codex_extension_api::LoadedUserInstructions;
use codex_extension_api::PromptFragment;
use codex_extension_api::PromptSlot;
use codex_extension_api::TurnContextContributionInput;
use codex_features::FEATURES;
use codex_features::Feature;
use codex_features::unstable_features_warning_event;
use codex_hooks::Hooks;
use codex_hooks::HooksConfig;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::auth_env_telemetry::collect_auth_env_telemetry;
use codex_login::default_client::originator;
use codex_mcp::McpConnectionManager;
use codex_mcp::McpResourceClient;
use codex_mcp::McpRuntimeContext;
use codex_mcp::codex_apps_tools_cache_key;
use codex_mcp_elicitation_api::McpServerElicitationRequest;
use codex_mcp_elicitation_api::McpServerElicitationRequestParams;
use codex_models_manager::manager::RefreshStrategy;
use codex_models_manager::manager::SharedModelsManager;
use codex_network_proxy::NetworkProxy;
use codex_network_proxy::NetworkProxyAuditMetadata;
use codex_network_proxy::normalize_host;
use codex_otel::current_span_trace_id;
use codex_otel::current_span_w3c_trace_context;
use codex_otel::set_parent_from_w3c_trace_context;
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::account::PlanType as AccountPlanType;
use codex_protocol::approvals::ElicitationRequestEvent;
use codex_protocol::approvals::ExecPolicyAmendment;
use codex_protocol::approvals::NetworkPolicyAmendment;
use codex_protocol::approvals::NetworkPolicyRuleAction;
use codex_protocol::config_types::ApprovalsReviewer;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::MultiAgentMode;
use codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use codex_protocol::config_types::Settings;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::dynamic_tools::DynamicToolResponse;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::items::AgentMessageContent;
use codex_protocol::items::TurnItem;
use codex_protocol::items::UserMessageItem;
use codex_protocol::mcp::CallToolResult;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::AdditionalPermissionProfile;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::SandboxEnforcement;
use codex_protocol::models::format_allow_prefixes;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::permissions::FileSystemSandboxPolicy;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_protocol::protocol::AdditionalContextEntry;
use codex_protocol::protocol::FileChange;
use codex_protocol::protocol::HasLegacyEvent;
use codex_protocol::protocol::InterAgentCommunication;
use codex_protocol::protocol::ItemCompletedEvent;
use codex_protocol::protocol::ItemStartedEvent;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::RawResponseItemEvent;
use codex_protocol::protocol::ReviewRequest;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TurnAbortReason;
use codex_protocol::protocol::TurnContextItem;
use codex_protocol::protocol::TurnContextNetworkItem;
use codex_protocol::protocol::TurnEnvironmentSelection;
use codex_protocol::protocol::TurnEnvironmentSelections;
use codex_protocol::protocol::W3cTraceContext;
use codex_protocol::request_permissions::PermissionGrantScope;
use codex_protocol::request_permissions::RequestPermissionProfile;
use codex_protocol::request_permissions::RequestPermissionsArgs;
use codex_protocol::request_permissions::RequestPermissionsEvent;
use codex_protocol::request_permissions::RequestPermissionsResponse;
use codex_protocol::request_user_input::RequestUserInputArgs;
use codex_protocol::request_user_input::RequestUserInputResponse;
use codex_rmcp_client::ElicitationResponse;
use codex_rollout::state_db;
use codex_rollout_trace::AgentResultTracePayload;
use codex_rollout_trace::ThreadStartedTraceMetadata;
use codex_rollout_trace::ThreadTraceContext;
use codex_sandboxing::policy_transforms::intersect_permission_profiles;
use codex_session_api::PreviousTurnSettings;
use codex_shell_command::parse_command::parse_command;
use codex_terminal_detection::user_agent;
use codex_thread_store_api::CreateThreadParams;
use codex_thread_store_api::LiveThreadFactory;
use codex_thread_store_api::LiveThreadHandle;
use codex_thread_store_api::ReadThreadDynamicToolsParams;
use codex_thread_store_api::ReadThreadParams;
use codex_thread_store_api::ResumeThreadParams;
use codex_thread_store_api::ThreadEventPersistenceMode;
use codex_thread_store_api::ThreadPersistenceMetadata;
use codex_thread_store_api::ThreadStore;
use codex_tools::UnifiedExecShellMode;
use codex_utils_output_truncation::TruncationPolicy;
use futures::future::BoxFuture;
use futures::future::Shared;
use futures::prelude::*;
use rmcp::model::ElicitationCapability;
use rmcp::model::FormElicitationCapability;
use rmcp::model::ListResourceTemplatesResult;
use rmcp::model::ListResourcesResult;
use rmcp::model::PaginatedRequestParams;
use rmcp::model::ReadResourceRequestParams;
use rmcp::model::ReadResourceResult;
use rmcp::model::RequestId;
use rmcp::model::UrlElicitationCapability;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use toml::Value as TomlValue;
use tracing::Instrument;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::info_span;
use tracing::instrument;
use tracing::warn;
use uuid::Uuid;

use crate::client::ModelClient;
use crate::codex_thread::ThreadConfigSnapshot;
#[cfg(test)]
use crate::compact::collect_user_messages;
use crate::config::Config;
use crate::config::Constrained;
use crate::config::ConstraintResult;
use crate::config::StartedNetworkProxy;
use crate::config::resolve_web_search_mode_for_turn;
use crate::config::resolved_permission_profile::PermissionProfileState;
use crate::context_manager::ContextManager;
use crate::thread_rollout_truncation::initial_history_has_prior_user_turns;
use codex_config::CONFIG_TOML_FILE;
use codex_config::types::McpServerConfig;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
#[cfg(test)]
use codex_protocol::exec_output::StreamOutput;

mod approvals;
pub(crate) mod blackboard;
mod checkpoint_git;
mod checkpoint_scratchpad;
mod codex_handle;
mod config_lock;
mod context_budget;
mod context_budget_adapter;
mod desktop_automation;
mod first_moves;
mod fork_features;
mod handlers;
mod initial_context;
mod inject;
mod input_queue;
mod mcp;
pub(crate) mod multi_agents;
mod review;
mod rollout_budget;
mod rollout_reconstruction;
#[allow(clippy::module_inception)]
pub(crate) mod session;
mod session_events;
mod session_history;
mod session_lifecycle;
mod session_mailbox;
mod session_network_proxy;
mod session_settings;
pub(crate) mod stream_resilience;
pub(crate) mod time_reminder;
mod token_budget;
pub(crate) mod turn;
pub(crate) mod turn_context;
pub(crate) mod usage_hint_reminder;
// upstream-added split modules (files exist on disk; consumed by sibling modules)
mod code_mode_warning;
pub(crate) mod context_window;
pub(crate) mod step_context;
mod world_state;
use self::config_lock::export_config_lock_if_configured;
use self::config_lock::validate_config_lock_if_configured;
pub(crate) use self::fork_features::ForkFeaturesState;
pub(crate) use self::fork_features::ForkFeaturesUpdate;
#[cfg(test)]
use self::handlers::submission_dispatch_span;
use self::handlers::submission_loop;
pub(crate) use self::input_queue::InputQueue;
pub(crate) use self::input_queue::InputQueueActivity;
pub(crate) use self::input_queue::TurnInput;
pub(crate) use self::input_queue::TurnInputQueue;
use self::review::spawn_review_thread;
use self::session::AppServerClientMetadata;
use self::session::Session;
use self::session::SessionConfiguration;
pub(crate) use self::session::SessionSettingsUpdate;
#[cfg(test)]
use self::turn::AssistantMessageStreamParsers;
#[cfg(test)]
use self::turn::collect_explicit_app_ids_from_skill_items;
#[cfg(test)]
use self::turn::filter_connectors_for_input;
use self::turn::realtime_text_for_event;
use self::turn_context::TurnContext;
use self::turn_context::TurnSkillsContext;
// imports for upstream methods relocated onto Session in this slim mod.rs
use self::world_state::build_world_state_from_environment_snapshot;
use crate::context::world_state::WorldState;
use crate::session::step_context::StepContext;
pub use codex_handle::Codex;
pub(crate) use codex_handle::CodexSpawnArgs;
pub use codex_handle::CodexSpawnOk;
pub(crate) use codex_handle::INITIAL_SUBMIT_ID;
pub(crate) use codex_handle::SUBMISSION_CHANNEL_CAPACITY;
pub(crate) use codex_handle::SessionLoopTermination;
pub use codex_handle::SteerInputError;
use codex_handle::*;
#[cfg(test)]
pub(crate) use session_lifecycle::completed_session_loop_termination;
pub(crate) use session_lifecycle::emit_subagent_session_started;
use session_lifecycle::*;
#[cfg(test)]
mod rollout_reconstruction_tests;

#[cfg(test)]
use crate::SkillMetadata;
use crate::SkillsService;
use crate::agents_md::load_project_instructions;
use crate::exec_policy::ExecPolicyUpdateError;
use crate::guardian::GuardianReviewSessionManager;
use crate::mcp::McpManager;
use crate::network_policy_decision::execpolicy_network_rule_amendment;
use crate::rollout::map_session_init_error;
use crate::session_startup_prewarm::SessionStartupPrewarmHandle;
use crate::shell;
use crate::shell_snapshot::ShellSnapshot;
#[cfg(test)]
use crate::skills::SkillLoadOutcome;
use crate::state::ActiveTurn;
use crate::state::AutoCompactWindowSnapshot;
use crate::state::PendingRequestPermissions;
use crate::state::SessionServices;
use crate::state::SessionState;
#[cfg(test)]
use crate::stream_events_utils::HandleOutputCtx;
#[cfg(test)]
use crate::stream_events_utils::handle_output_item_done;
use crate::tasks::ReviewTask;
use crate::tools::network_approval::NetworkApprovalService;
use crate::tools::network_approval::build_blocked_request_observer;
use crate::tools::network_approval::build_network_policy_decider;
#[cfg(test)]
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::sandboxing::ApprovalStore;
use crate::turn_timing::TurnTimingState;
use crate::turn_timing::record_turn_ttfm_metric;
use crate::unified_exec::UnifiedExecProcessManager;
use crate::windows_sandbox::WindowsSandboxLevelExt;
use codex_core_plugins::PluginsManager;
use codex_core_plugins::RecommendedPluginCandidatesInput;
use codex_git_utils::get_git_repo_root;
use codex_mcp::McpConfig;
use codex_mcp::compute_auth_statuses;
use codex_mcp::effective_mcp_servers_from_configured;
use codex_mcp::host_owned_codex_apps_enabled;
use codex_otel::SessionTelemetry;
use codex_otel::THREAD_STARTED_METRIC;
use codex_otel::TelemetryAuthMode;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::models::LocalImagePreparation;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_protocol::protocol::ApplyPatchApprovalRequestEvent;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::CodexErrorInfo;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::DeprecationNoticeEvent;
use codex_protocol::protocol::ErrorEvent;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ExecApprovalRequestEvent;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::McpServerRefreshConfig;
use codex_protocol::protocol::ModelRerouteEvent;
use codex_protocol::protocol::ModelRerouteReason;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::ModelVerificationEvent;
use codex_protocol::protocol::NetworkApprovalContext;
use codex_protocol::protocol::NonSteerableTurnKind;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::RequestUserInputEvent;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::protocol::SessionConfiguredEvent;
use codex_protocol::protocol::SessionNetworkProxyRuntime;
use codex_protocol::protocol::StreamErrorEvent;
use codex_protocol::protocol::Submission;
use codex_protocol::protocol::ThreadMemoryMode;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::protocol::TokenCountEvent;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageInfo;
use codex_protocol::protocol::TurnModerationMetadataEvent;
use codex_protocol::protocol::WarningEvent;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
#[cfg(test)]
use codex_utils_stream_parser::ProposedPlanSegment;

impl Session {
    // --- upstream-only methods relocated onto Session ---
    // These live ONLY in upstream's monolithic session/mod.rs; the fork's split
    // siblings do not own them, but call-sites in compact*/turn/tests still need
    // them on `Session`, so keep them here (mirrors the existing relocated helpers).
    pub(crate) async fn take_new_context_window_request(&self) -> bool {
        let mut state = self.state.lock().await;
        state.take_new_context_window_request()
    }

    pub(crate) async fn start_new_context_window(
        &self,
        turn_context: &TurnContext,
        world_state: Arc<WorldState>,
    ) -> u64 {
        let window = {
            let mut state = self.state.lock().await;
            state.start_new_context_window()
        };
        let (window_number, window_ids) = window;
        let context_items = self
            .build_initial_context_with_world_state(turn_context, world_state.as_ref())
            .await;
        let turn_context_item = turn_context.to_turn_context_item();
        // Keep the freshly-built world state as the live diff baseline for subsequent turns.
        self.state
            .lock()
            .await
            .history
            .set_world_state_baseline(Arc::clone(&world_state));
        self.replace_compacted_history(
            context_items,
            Some(turn_context_item),
            CompactedItem {
                message: String::new(),
                replacement_history: None,
                window_number: Some(window_number),
                first_window_id: Some(window_ids.first_window_id.to_string()),
                previous_window_id: window_ids.previous_window_id.map(|id| id.to_string()),
                window_id: Some(window_ids.window_id.to_string()),
            },
        )
        .await;
        self.recompute_token_usage(turn_context).await;
        window_number
    }

    pub(crate) async fn build_initial_context_with_world_state(
        &self,
        turn_context: &TurnContext,
        world_state: &WorldState,
    ) -> Vec<ResponseItem> {
        let mut developer_sections = Vec::<String>::with_capacity(8);
        let mut contextual_user_sections = Vec::<String>::with_capacity(2);
        let mut separate_developer_sections = Vec::<String>::new();
        let (
            reference_context_item,
            previous_turn_settings,
            collaboration_mode,
            base_instructions,
            session_source,
            auto_compact_window_ids,
        ) = {
            let state = self.state.lock().await;
            (
                state.reference_context_item(),
                state.previous_turn_settings(),
                state.session_configuration.collaboration_mode.clone(),
                state.session_configuration.base_instructions.clone(),
                state.session_configuration.session_source.clone(),
                state.auto_compact_window_ids(),
            )
        };
        if let Some(model_switch_message) =
            crate::context_manager::updates::build_model_instructions_update_item(
                previous_turn_settings.as_ref(),
                turn_context,
            )
        {
            developer_sections.push(model_switch_message);
        }
        if turn_context.config.include_permissions_instructions {
            developer_sections.push(
                PermissionsInstructions::from_permission_profile(
                    &turn_context.permission_profile,
                    turn_context.approval_policy.value(),
                    turn_context.config.approvals_reviewer,
                    self.services.exec_policy.current().as_ref(),
                    #[allow(deprecated)]
                    &turn_context.cwd,
                    turn_context
                        .config
                        .features
                        .enabled(Feature::ExecPermissionApprovals),
                    turn_context
                        .config
                        .features
                        .enabled(Feature::RequestPermissionsTool),
                )
                .render(),
            );
        }
        let separate_guardian_developer_message =
            crate::guardian::is_guardian_reviewer_source(&session_source);
        // Keep the guardian policy prompt out of the aggregated developer bundle so it
        // stays isolated as its own top-level developer message for guardian subagents.
        if !separate_guardian_developer_message
            && let Some(developer_instructions) = turn_context.developer_instructions.as_deref()
            && !developer_instructions.is_empty()
        {
            developer_sections.push(developer_instructions.to_string());
        }
        // Add developer instructions from collaboration_mode if they exist and are non-empty
        if turn_context.config.include_collaboration_mode_instructions
            && let Some(collab_instructions) =
                CollaborationModeInstructions::from_collaboration_mode(&collaboration_mode)
        {
            developer_sections.push(collab_instructions.render());
        }
        if let Some(realtime_update) = crate::context_manager::updates::build_initial_realtime_item(
            reference_context_item.as_ref(),
            previous_turn_settings.as_ref(),
            turn_context,
        ) {
            developer_sections.push(realtime_update);
        }
        if self.features.enabled(Feature::Personality)
            && let Some(personality) = turn_context.personality
        {
            let model_info = turn_context.model_info.clone();
            let has_baked_personality = model_info.supports_personality()
                && base_instructions == model_info.get_model_instructions(Some(personality));
            if !has_baked_personality
                && let Some(personality_message) =
                    crate::context_manager::updates::personality_message_for(
                        &model_info,
                        personality,
                    )
            {
                developer_sections
                    .push(PersonalitySpecInstructions::new(personality_message).render());
            }
        }
        if turn_context.config.include_apps_instructions && turn_context.apps_enabled() {
            let mcp_connection_manager = self.services.mcp_connection_manager.load_full();
            let accessible_and_enabled_connectors =
                connectors::list_accessible_and_enabled_connectors_from_manager(
                    &mcp_connection_manager,
                    &turn_context.config,
                )
                .await;
            if let Some(apps_instructions) =
                AppsInstructions::from_connectors(&accessible_and_enabled_connectors)
            {
                developer_sections.push(apps_instructions.render());
            }
        }
        if turn_context.config.include_skill_instructions {
            let available_skills = build_available_skills(
                turn_context.turn_skills.snapshot.outcome(),
                default_skill_metadata_budget(turn_context.model_info.context_window),
                SkillRenderSideEffects::ThreadStart {
                    session_telemetry: &self.services.session_telemetry,
                },
            );
            if let Some(available_skills) = available_skills {
                let warning_message = available_skills.warning_message.clone();
                let skills_instructions = AvailableSkillsInstructions::from(available_skills);
                if let Some(warning_message) = warning_message {
                    self.send_event_raw(Event {
                        id: String::new(),
                        msg: EventMsg::Warning(WarningEvent {
                            message: warning_message,
                        }),
                    })
                    .await;
                }
                developer_sections.push(skills_instructions.render());
            }
        }
        let loaded_plugins = self
            .services
            .plugins_manager
            .plugins_for_config(&turn_context.config.plugins_config_input())
            .await;
        let recommended_plugin_candidates =
            if crate::tools::spec_plan::tool_suggest_enabled(turn_context) {
                let auth = self.services.auth_manager.auth().await;
                let plugins_config = turn_context.config.plugins_config_input();
                self.services
                    .plugins_manager
                    .recommended_plugin_candidates_for_config(RecommendedPluginCandidatesInput {
                        plugins_config: &plugins_config,
                        loaded_plugins: &loaded_plugins,
                        auth: auth.as_ref(),
                        disabled_tools: &turn_context.config.tool_suggest.disabled_tools,
                        app_server_client_name: turn_context.app_server_client_name.as_deref(),
                    })
                    .await
            } else {
                None
            };
        if let Some(recommended_plugins) = recommended_plugin_candidates
            .as_deref()
            .and_then(RecommendedPluginsInstructions::from_plugins)
        {
            contextual_user_sections.push(recommended_plugins.render());
        }
        if let Some(plugin_instructions) =
            AvailablePluginsInstructions::from_plugins(loaded_plugins.capability_summaries())
        {
            developer_sections.push(plugin_instructions.render());
        }
        let context_contributors = self.services.extensions.context_contributors().to_vec();
        for contributor in &context_contributors {
            for fragment in contributor
                .contribute_thread_context(
                    &self.services.session_extension_data,
                    &self.services.thread_extension_data,
                )
                .await
            {
                push_prompt_fragment(
                    fragment,
                    &mut developer_sections,
                    &mut contextual_user_sections,
                    &mut separate_developer_sections,
                );
            }
        }
        for contributor in &context_contributors {
            for fragment in contributor
                .contribute_turn_context(TurnContextContributionInput {
                    thread_id: self.thread_id(),
                    turn_id: turn_context.sub_id.as_str(),
                    session_store: &self.services.session_extension_data,
                    thread_store: &self.services.thread_extension_data,
                    turn_store: turn_context.extension_data.as_ref(),
                    model_context_window: turn_context.model_context_window(),
                })
                .await
            {
                push_prompt_fragment(
                    fragment,
                    &mut developer_sections,
                    &mut contextual_user_sections,
                    &mut separate_developer_sections,
                );
            }
        }
        if let Some(user_instructions) = turn_context.user_instructions.as_deref() {
            contextual_user_sections.push(user_instructions.to_string());
        }
        // This is full-context metadata. Steady-state context diffs should not re-emit it.
        if turn_context.config.features.enabled(Feature::TokenBudget)
            && turn_context.model_context_window().is_some()
        {
            let mcp_result = self
                .call_tool(
                    "notes",
                    "thread_hint",
                    /*arguments*/ None,
                    Some(serde_json::json!({
                        "threadId": self.thread_id().to_string(),
                    })),
                )
                .await
                .ok()
                .and_then(|result| {
                    let text = result
                        .content
                        .iter()
                        .filter_map(|content| {
                            content.get("text").and_then(serde_json::Value::as_str)
                        })
                        .filter(|text| !text.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    (!text.is_empty()).then_some(text)
                });
            developer_sections.push(
                crate::context::TokenBudgetContext::new(
                    self.thread_id(),
                    auto_compact_window_ids.first_window_id,
                    auto_compact_window_ids.previous_window_id,
                    auto_compact_window_ids.window_id,
                    mcp_result,
                )
                .render(),
            );
        }
        for fragment in world_state.render_full() {
            match fragment.role() {
                "developer" => developer_sections.push(fragment.render()),
                "user" => contextual_user_sections.push(fragment.render()),
                _ => {}
            }
        }

        let multi_agent_v2_usage_hint_text =
            multi_agents::usage_hint_text(turn_context, &session_source);

        let mut items = Vec::with_capacity(4);
        if let Some(developer_message) =
            crate::context_manager::updates::build_developer_update_item(developer_sections)
        {
            items.push(developer_message);
        }
        for section in separate_developer_sections {
            if let Some(developer_message) =
                crate::context_manager::updates::build_developer_update_item(vec![section])
            {
                items.push(developer_message);
            }
        }
        if let Some(usage_hint_text) = multi_agent_v2_usage_hint_text
            && let Some(usage_hint_message) =
                crate::context_manager::updates::build_developer_update_item(vec![
                    usage_hint_text.to_string(),
                ])
        {
            items.push(usage_hint_message);
        }
        match multi_agents::effective_multi_agent_mode(
            turn_context.multi_agent_version,
            &session_source,
            turn_context.multi_agent_mode,
        ) {
            Some(
                multi_agent_mode
                @ (MultiAgentMode::ExplicitRequestOnly | MultiAgentMode::Proactive),
            ) => {
                items.push(ContextualUserFragment::into(
                    MultiAgentModeInstructions::new(multi_agent_mode),
                ));
            }
            Some(MultiAgentMode::None) | None => {}
        }
        if let Some(contextual_user_message) =
            crate::context_manager::updates::build_contextual_user_message(contextual_user_sections)
        {
            items.push(contextual_user_message);
        }
        // Emit the guardian policy prompt as a separate developer item so the guardian
        // subagent sees a distinct, easy-to-audit instruction block.
        if separate_guardian_developer_message
            && let Some(developer_instructions) = turn_context.developer_instructions.as_deref()
            && !developer_instructions.is_empty()
            && let Some(guardian_developer_message) =
                crate::context_manager::updates::build_developer_update_item(vec![
                    developer_instructions.to_string(),
                ])
        {
            items.push(guardian_developer_message);
        }
        // New context windows and compaction install these items directly into replacement history.
        for item in &mut items {
            item.set_turn_id_if_missing(&turn_context.sub_id);
        }
        items
    }

    pub(crate) async fn build_world_state_for_environments(
        &self,
        turn_context: &TurnContext,
        environments: &TurnEnvironmentSnapshot,
    ) -> WorldState {
        let environment_subagents = if turn_context.config.include_environment_context {
            self.services
                .agent_control
                .format_environment_context_subagents(self.thread_id)
                .await
        } else {
            String::new()
        };
        build_world_state_from_environment_snapshot(
            turn_context,
            environments,
            &environment_subagents,
        )
    }

    pub(crate) async fn capture_step_context(
        &self,
        turn_context: Arc<TurnContext>,
    ) -> Arc<StepContext> {
        // Keep the old turn-frozen view unless deferred executors are explicitly enabled.
        let environments = if turn_context
            .config
            .features
            .enabled(Feature::DeferredExecutor)
        {
            self.services.turn_environments.snapshot().await
        } else {
            turn_context.environments.clone()
        };
        Arc::new(StepContext::new(turn_context, environments))
    }

    pub(crate) async fn record_step_environment_context_if_changed(
        &self,
        previous_world_state: &Arc<WorldState>,
        step_context: &step_context::StepContext,
    ) -> Arc<WorldState> {
        let turn_context = step_context.turn.as_ref();
        // Render model-visible state from the same step used to build and run tools.
        let world_state = Arc::new(
            self.build_world_state_for_environments(turn_context, &step_context.environments)
                .await,
        );
        let items = crate::context_manager::updates::merge_contextual_fragments(
            world_state.render_diff(previous_world_state.as_ref()),
        );
        if !items.is_empty() {
            self.record_conversation_items(turn_context, &items).await;
        }

        // ContextManager remembers this for later turns; run_turn owns the live value.
        self.state
            .lock()
            .await
            .history
            .set_world_state_baseline(Arc::clone(&world_state));
        world_state
    }

    pub(crate) fn response_item_from_user_input(&self, input: Vec<UserInput>) -> ResponseItem {
        ResponseItem::from(ResponseInputItem::from_user_input(
            input,
            LocalImagePreparation::Defer,
        ))
    }

    fn assign_missing_response_item_ids(items: Cow<'_, [ResponseItem]>) -> Cow<'_, [ResponseItem]> {
        if items.iter().all(|item| item.id().is_some()) {
            return items;
        }
        let mut items = items;
        for item in items.to_mut() {
            Self::assign_missing_response_item_id(item);
        }
        items
    }

    fn assign_missing_response_item_id(item: &mut ResponseItem) {
        if item.id().is_some() {
            return;
        }
        let prefix = match item {
            ResponseItem::AdditionalTools { .. } => "at",
            ResponseItem::Message { .. } => "msg",
            ResponseItem::Reasoning { .. } => "rs",
            ResponseItem::LocalShellCall { .. } => "lsh",
            ResponseItem::FunctionCall { .. } => "fc",
            ResponseItem::ToolSearchCall { .. } => "tsc",
            ResponseItem::FunctionCallOutput { .. } => "fco",
            ResponseItem::CustomToolCall { .. } => "ctc",
            ResponseItem::CustomToolCallOutput { .. } => "ctco",
            ResponseItem::ToolSearchOutput { .. } => "tso",
            ResponseItem::WebSearchCall { .. } => "ws",
            ResponseItem::ImageGenerationCall { .. } => "ig",
            ResponseItem::Compaction { .. } | ResponseItem::ContextCompaction { .. } => "cmp",
            ResponseItem::AgentMessage { .. } => "amsg",
            ResponseItem::CompactionTrigger { .. } | ResponseItem::Other => return,
        };
        item.set_id(Some(format!("{prefix}_{}", Uuid::now_v7())));
    }

    #[cfg(test)]
    pub(crate) async fn codex_home(&self) -> AbsolutePathBuf {
        let state = self.state.lock().await;
        state.session_configuration.codex_home().clone()
    }

    pub(crate) fn subscribe_out_of_band_elicitation_pause_state(&self) -> watch::Receiver<bool> {
        self.out_of_band_elicitation_paused.subscribe()
    }

    pub(crate) fn set_out_of_band_elicitation_pause_state(&self, paused: bool) {
        self.out_of_band_elicitation_paused.send_replace(paused);
    }

    pub(crate) fn get_tx_event(&self) -> Sender<Event> {
        self.tx_event.clone()
    }

    pub(crate) fn state_db(&self) -> Option<state_db::StateDbHandle> {
        self.services.state_db.clone()
    }

    pub(crate) fn live_thread_for_persistence(
        &self,
        operation: &str,
    ) -> anyhow::Result<&Arc<dyn LiveThreadHandle>> {
        self.live_thread()
            .ok_or_else(|| anyhow::anyhow!("Session persistence is disabled; cannot {operation}."))
    }

    pub(crate) fn live_thread(&self) -> Option<&Arc<dyn LiveThreadHandle>> {
        self.services.live_thread.as_ref()
    }

    pub(crate) fn track_turn_codex_error(&self, turn_context: &TurnContext, error: &CodexErr) {
        self.services
            .analytics_events_client
            .track_turn_codex_error(TurnCodexErrorFact::from_codex_err(
                self.thread_id.to_string(),
                turn_context.sub_id.clone(),
                error,
            ));
    }

    pub(crate) fn multi_agent_version(&self) -> Option<MultiAgentVersion> {
        self.multi_agent_version.get().copied()
    }

    pub(crate) fn set_multi_agent_version_if_unset(
        &self,
        multi_agent_version: MultiAgentVersion,
    ) -> MultiAgentVersion {
        *self.multi_agent_version.get_or_init(|| multi_agent_version)
    }

    pub(crate) fn resolve_multi_agent_version_for_model(
        &self,
        model_info: &ModelInfo,
        config: &Config,
    ) -> MultiAgentVersion {
        if let Some(v) = self.multi_agent_version() {
            return v;
        }
        let selected = model_info
            .multi_agent_version
            .unwrap_or_else(|| config.multi_agent_version_from_features());
        self.set_multi_agent_version_if_unset(selected)
    }

    /// Flush rollout writes and return the final durability-barrier result.
    #[instrument(name = "session.flush_rollout", level = "trace", skip_all)]
    pub(crate) async fn flush_rollout(&self) -> std::io::Result<()> {
        if let Some(live_thread) = self.live_thread() {
            live_thread.flush().await.map_err(std::io::Error::other)
        } else {
            Ok(())
        }
    }

    pub(crate) async fn try_ensure_rollout_materialized(&self) -> std::io::Result<()> {
        if let Some(live_thread) = self.live_thread() {
            live_thread.persist().await.map_err(std::io::Error::other)?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_rollout_materialized(&self) {
        if let Err(e) = self.try_ensure_rollout_materialized().await {
            warn!("failed to materialize thread persistence: {e}");
        }
    }

    fn next_internal_sub_id(&self) -> String {
        let id = self
            .next_internal_sub_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("auto-compact-{id}")
    }

    pub(crate) async fn route_realtime_text_input(self: &Arc<Self>, text: String) {
        handlers::user_input_or_turn_inner(
            self,
            Uuid::now_v7().to_string(),
            Op::UserInput {
                items: vec![UserInput::Text {
                    text,
                    text_elements: Vec::new(),
                }],
                environments: None,
                final_output_json_schema: None,
                responsesapi_client_metadata: None,
                additional_context: Default::default(),
                thread_settings: ThreadSettingsOverrides::default(),
            },
            /*client_user_message_id*/ None,
        )
        .await;
    }

    /// fork-local: relocated impl Session lives in split files; this upstream-only
    /// method is still called by session/turn.rs, so keep it on Session here.
    pub(crate) async fn emit_turn_moderation_metadata(
        self: &Arc<Self>,
        turn_context: &Arc<TurnContext>,
        metadata: TurnModerationMetadataEvent,
    ) {
        self.send_event(turn_context, EventMsg::TurnModerationMetadata(metadata))
            .await;
    }

    /// fork-local: upstream relocated `auto_compact_window_snapshot` onto `Session`
    /// in their monolithic `mod.rs`; the fork's slim `mod.rs` dropped it during the
    /// impl-split. `session/turn.rs::auto_compact_token_status` still calls it for the
    /// `BodyAfterPrefix` scope, so keep the thin state wrapper on `Session` here.
    pub(crate) async fn auto_compact_window_snapshot(&self) -> AutoCompactWindowSnapshot {
        let state = self.state.lock().await;
        state.auto_compact_window_snapshot()
    }

    pub(crate) fn hooks(&self) -> Arc<Hooks> {
        self.services.hooks.load_full()
    }

    pub(crate) fn user_shell(&self) -> Arc<shell::Shell> {
        Arc::clone(&self.services.user_shell)
    }

    pub(crate) async fn current_rollout_path(&self) -> anyhow::Result<Option<PathBuf>> {
        let Some(live_thread) = self.live_thread() else {
            return Ok(None);
        };
        live_thread.local_rollout_path().await.map_err(Into::into)
    }

    pub(crate) async fn hook_transcript_path(&self) -> Option<PathBuf> {
        self.ensure_rollout_materialized().await;
        match self.current_rollout_path().await {
            Ok(path) => path,
            Err(err) => {
                warn!("{err}");
                None
            }
        }
    }

    pub(crate) async fn take_pending_session_start_source(
        &self,
    ) -> Option<codex_hooks::SessionStartSource> {
        let mut state = self.state.lock().await;
        state.take_pending_session_start_source()
    }

    fn show_raw_agent_reasoning(&self) -> bool {
        self.services.show_raw_agent_reasoning
    }
}

// upstream-only free fn used by relocated Session methods above
fn push_prompt_fragment(
    fragment: PromptFragment,
    developer_sections: &mut Vec<String>,
    contextual_user_sections: &mut Vec<String>,
    separate_developer_sections: &mut Vec<String>,
) {
    match fragment.slot() {
        PromptSlot::DeveloperPolicy | PromptSlot::DeveloperCapabilities => {
            developer_sections.push(fragment.text().to_string());
        }
        PromptSlot::ContextualUser => {
            contextual_user_sections.push(fragment.text().to_string());
        }
        PromptSlot::SeparateDeveloper => {
            separate_developer_sections.push(fragment.text().to_string());
        }
    }
}

pub(crate) fn resolve_multi_agent_version(
    conversation_history: &InitialHistory,
    inherited_multi_agent_version: Option<MultiAgentVersion>,
) -> Option<MultiAgentVersion> {
    if inherited_multi_agent_version == Some(MultiAgentVersion::Disabled) {
        return Some(MultiAgentVersion::Disabled);
    }
    conversation_history
        .get_multi_agent_version()
        .or(inherited_multi_agent_version)
        .or(match conversation_history {
            InitialHistory::New | InitialHistory::Cleared => None,
            InitialHistory::Resumed(_) | InitialHistory::Forked(_) => Some(MultiAgentVersion::V1),
        })
}

#[cfg(test)]
pub(crate) mod tests;
