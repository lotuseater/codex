use super::turn_context::TurnEnvironment;
use super::*;
use crate::config::ConfigBuilder;
use crate::config::ThreadStoreConfig;
use crate::config::test_config;
use crate::context::ContextualUserFragment;
use crate::context::TurnAborted;
use crate::exec::ExecCapturePolicy;
use codex_tool_execution_api::FunctionCallError;
use crate::session::blackboard::new_blackboard_session;
use crate::shell::default_user_shell;
use crate::skills::SkillRenderSideEffects;
use crate::skills::render::SkillMetadataBudget;
use crate::test_support::models_manager_with_provider;
use crate::tools::format_exec_output_str;
use codex_config::ConfigLayerStack;
use codex_config::ConfigLayerStackOrdering;
use codex_config::NetworkConstraints;
use codex_config::NetworkDomainPermissionToml;
use codex_config::NetworkDomainPermissionsToml;
use codex_config::RequirementSource;
use codex_config::Sourced;
use codex_config::loader::project_trust_key;
use codex_config::types::ToolSuggestDisabledTool;

use codex_features::Feature;
use codex_features::Features;
use codex_login::CodexAuth;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::bundled_models_response;
use codex_models_manager::model_info;
use codex_models_manager::test_support::construct_model_info_offline_for_tests;
use codex_models_manager::test_support::get_model_offline_for_tests;
use codex_protocol::AgentPath;
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::account::PlanType as AccountPlanType;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::config_types::TrustLevel;
use codex_protocol::exec_output::ExecToolCallOutput;
use codex_protocol::models::FileSystemPermissions;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::SandboxEnforcement;
use codex_protocol::permissions::FileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry;
use codex_protocol::permissions::FileSystemSandboxPolicy;
use codex_protocol::permissions::FileSystemSpecialPath;
use codex_protocol::protocol::NonSteerableTurnKind;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::protocol::TurnEnvironmentSelection;
use codex_protocol::request_permissions::PermissionGrantScope;
use codex_protocol::request_permissions::RequestPermissionProfile;
use tracing::Span;

use crate::goals::ExternalGoalPreviousStatus;
use crate::goals::ExternalGoalSet;
use crate::goals::GoalRuntimeEvent;
use crate::goals::SetGoalRequest;
use crate::rollout::recorder::RolloutRecorder;
use crate::state::ActiveTurn;
use crate::state::TaskKind;
use crate::tasks::SessionTask;
use crate::tasks::SessionTaskContext;
use crate::tasks::UserShellCommandMode;
use crate::tasks::execute_user_shell_command;
use crate::tools::ToolRouter;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::CreateGoalHandler;
use crate::tools::handlers::ExecCommandHandler;
use crate::tools::handlers::ShellCommandHandler;
use crate::tools::handlers::UpdateGoalHandler;
use crate::tools::registry::ToolExecutor;
use crate::tools::router::ToolCallSource;
use crate::turn_diff_tracker::TurnDiffTracker;
use codex_app_catalog_types::AppInfo;
use codex_config::config_toml::ConfigToml;
use codex_config::config_toml::ProjectConfig;
use codex_test_support_responses::context_snapshot;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::tracing::install_test_tracing;
use codex_core_test_runtime::wait_for_event;
use codex_core_test_runtime::wait_for_event_match;
use codex_execpolicy::Decision;
use codex_execpolicy::NetworkRuleProtocol;
use codex_execpolicy::Policy;
use codex_mcp_elicitation_api::McpElicitationSchema;
use codex_network_proxy::NetworkProxyConfig;
use codex_otel::MetricsClient;
use codex_otel::MetricsConfig;
use codex_otel::THREAD_SKILLS_DESCRIPTION_TRUNCATED_CHARS_METRIC;
use codex_otel::THREAD_SKILLS_ENABLED_TOTAL_METRIC;
use codex_otel::THREAD_SKILLS_KEPT_TOTAL_METRIC;
use codex_otel::THREAD_SKILLS_TRUNCATED_METRIC;
use codex_otel::TelemetryAuthMode;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::Settings;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::ConversationAudioParams;
use codex_protocol::protocol::CreditsSnapshot;
use codex_protocol::protocol::GranularApprovalConfig;
use codex_protocol::protocol::InitialHistory;
use codex_protocol::protocol::InterAgentCommunication;
use codex_protocol::protocol::NetworkApprovalProtocol;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::RateLimitWindow;
use codex_protocol::protocol::RealtimeAudioFrame;
use codex_protocol::protocol::RealtimeConversationListVoicesResponseEvent;
use codex_protocol::protocol::RealtimeVoice;
use codex_protocol::protocol::RealtimeVoicesList;
use codex_protocol::protocol::ResumedHistory;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::SkillScope;
use codex_protocol::protocol::Submission;
use codex_protocol::protocol::ThreadGoalStatus;
use codex_protocol::protocol::ThreadRolledBackEvent;
use codex_protocol::protocol::TokenCountEvent;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageInfo;
use codex_protocol::protocol::TurnAbortedEvent;
use codex_protocol::protocol::TurnCompleteEvent;
use codex_protocol::protocol::TurnStartedEvent;
use codex_protocol::protocol::UserMessageEvent;
use codex_protocol::protocol::W3cTraceContext;
use codex_protocol::request_user_input::RequestUserInputAnswer;
use codex_protocol::request_user_input::RequestUserInputResponse;
use codex_rmcp_client::ElicitationAction;
use codex_test_support_context_fixtures::context_snapshot::ContextSnapshotOptions;
use codex_test_support_context_fixtures::context_snapshot::ContextSnapshotRenderMode;
use codex_test_support_lightweight::PathBufExt;
use codex_test_support_lightweight::PathExt;
use codex_test_support_lightweight::test_path_buf;
use codex_test_support_responses::responses::ev_assistant_message;
use codex_test_support_responses::responses::ev_completed;
use codex_test_support_responses::responses::ev_completed_with_tokens;
use codex_test_support_responses::responses::ev_function_call;
use codex_test_support_responses::responses::ev_response_created;
use codex_test_support_responses::responses::mount_sse_once;
use codex_test_support_responses::responses::mount_sse_sequence;
use codex_test_support_responses::responses::sse;
use codex_test_support_responses::responses::start_mock_server;
use codex_thread_store::InMemoryThreadStore;
use codex_thread_store::LiveThread;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::ThreadStoreSelection;
use codex_thread_store::thread_store_from_config;
use codex_thread_store_api::RecordingLiveThreadFactory;
use codex_thread_store_api::RecordingThreadStore;
use codex_tool_execution_api::ShellCommandBackendConfig;
use codex_tool_execution_api::ToolEnvironmentMode;
use codex_tool_execution_api::ToolName;
use codex_tool_registry_api::ToolSpec;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::TraceId;
use opentelemetry_sdk::metrics::InMemoryMetricExporter;
use opentelemetry_sdk::metrics::data::AggregatedMetrics;
use opentelemetry_sdk::metrics::data::Metric;
use opentelemetry_sdk::metrics::data::MetricData;
use opentelemetry_sdk::metrics::data::ResourceMetrics;
use std::path::Path;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use codex_protocol::mcp::CallToolResult as McpCallToolResult;
use pretty_assertions::assert_eq as pretty_assert_eq;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration as StdDuration;

mod environment_tests;
mod guardian_tests;
mod lifecycle_tests;
mod policy_permission_tests;
mod session_settings_tests;
mod thread_history_tests;
mod turn_flow_tests;
mod workspace_roots_tests;

fn plain_tool_name(name: impl Into<String>) -> ToolName {
    ToolName::plain(name)
}

fn permission_profile_for_sandbox_policy(sandbox_policy: &SandboxPolicy) -> PermissionProfile {
    PermissionProfile::from_legacy_sandbox_policy(sandbox_policy)
}
struct InstructionsTestCase {
    slug: &'static str,
    expects_apply_patch_description: bool,
}

fn user_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

fn assistant_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

fn test_session_telemetry_without_metadata() -> SessionTelemetry {
    let exporter = InMemoryMetricExporter::default();
    let metrics = MetricsClient::new(
        MetricsConfig::in_memory("test", "codex-core", env!("CARGO_PKG_VERSION"), exporter)
            .with_runtime_reader(),
    )
    .expect("in-memory metrics client");
    SessionTelemetry::new(
        ThreadId::new(),
        "gpt-5.4",
        "gpt-5.4",
        /*account_id*/ None,
        /*account_email*/ None,
        /*auth_mode*/ None,
        "test_originator".to_string(),
        /*log_user_prompts*/ false,
        "tty".to_string(),
        SessionSource::Cli,
    )
    .with_metrics_without_metadata_tags(metrics)
}

fn find_metric<'a>(resource_metrics: &'a ResourceMetrics, name: &str) -> &'a Metric {
    for scope_metrics in resource_metrics.scope_metrics() {
        for metric in scope_metrics.metrics() {
            if metric.name() == name {
                return metric;
            }
        }
    }
    panic!("metric {name} missing");
}

fn histogram_sum(resource_metrics: &ResourceMetrics, name: &str) -> u64 {
    let metric = find_metric(resource_metrics, name);
    match metric.data() {
        AggregatedMetrics::F64(data) => match data {
            MetricData::Histogram(histogram) => {
                let points: Vec<_> = histogram.data_points().collect();
                pretty_assert_eq!(points.len(), 1);
                points[0].sum().round() as u64
            }
            _ => panic!("unexpected histogram aggregation"),
        },
        _ => panic!("unexpected metric data type"),
    }
}

fn skill_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

fn test_model_client_session() -> crate::client::ModelClientSession {
    let thread_id = ThreadId::try_from("00000000-0000-4000-8000-000000000001")
        .expect("test thread id should be valid");
    crate::client::ModelClient::new(
        /*auth_manager*/ None,
        thread_id.into(),
        thread_id,
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        ModelProviderInfo::create_openai_provider(/* base_url */ /*base_url*/ None),
        codex_protocol::protocol::SessionSource::Exec,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
        /*attestation_provider*/ None,
    )
    .new_session()
}

fn developer_input_texts(items: &[ResponseItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "developer" => {
                Some(content.as_slice())
            }
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn developer_message_texts(items: &[ResponseItem]) -> Vec<Vec<&str>> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "developer" => {
                Some(content.as_slice())
            }
            _ => None,
        })
        .map(|content| {
            content
                .iter()
                .filter_map(|item| match item {
                    ContentItem::InputText { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

fn user_input_texts(items: &[ResponseItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "user" => {
                Some(content.as_slice())
            }
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn write_project_hooks(dot_codex: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dot_codex)?;
    std::fs::write(
        dot_codex.join("hooks.json"),
        r#"{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo hello from hook"
          }
        ]
      }
    ]
  }
}"#,
    )
}

async fn write_project_trust_config(
    codex_home: &Path,
    trusted_projects: &[(&Path, TrustLevel)],
) -> std::io::Result<()> {
    tokio::fs::write(
        codex_home.join(codex_config::CONFIG_TOML_FILE),
        toml::to_string(&ConfigToml {
            projects: Some(
                trusted_projects
                    .iter()
                    .map(|(project, trust_level)| {
                        (
                            project_trust_key(project),
                            ProjectConfig {
                                trust_level: Some(*trust_level),
                            },
                        )
                    })
                    .collect::<std::collections::HashMap<_, _>>(),
            ),
            ..Default::default()
        })
        .expect("serialize config"),
    )
    .await
}

async fn preview_session_start_hooks(
    config: &crate::config::Config,
) -> std::io::Result<Vec<codex_protocol::protocol::HookRunSummary>> {
    let hooks = Hooks::new(HooksConfig {
        feature_enabled: true,
        config_layer_stack: Some(config.config_layer_stack.clone()),
        ..HooksConfig::default()
    });

    Ok(
        hooks.preview_session_start(&codex_hooks::SessionStartRequest {
            session_id: ThreadId::new(),
            cwd: config.cwd.clone(),
            transcript_path: None,
            model: "gpt-5.2".to_string(),
            permission_mode: "default".to_string(),
            target: codex_hooks::StartHookTarget::SessionStart {
                source: codex_hooks::SessionStartSource::Startup,
            },
        }),
    )
}

fn test_tool_runtime(session: Arc<Session>, turn_context: Arc<TurnContext>) -> ToolCallRuntime {
    let router = Arc::new(ToolRouter::from_turn_context(
        &turn_context,
        crate::tools::router::ToolRouterParams {
            mcp_tools: None,
            deferred_mcp_tools: None,
            discoverable_tools: None,
            dynamic_tools: turn_context.dynamic_tools.as_slice(),
            extension_tool_executors: Vec::new(),
        },
    ));
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    ToolCallRuntime::new(router, session, turn_context, tracker)
}

fn make_connector(id: &str, name: &str) -> AppInfo {
    AppInfo {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        logo_url: None,
        logo_url_dark: None,
        distribution_channel: None,
        branding: None,
        app_metadata: None,
        labels: None,
        install_url: None,
        is_accessible: true,
        is_enabled: true,
        plugin_display_names: Vec::new(),
    }
}

async fn wait_for_thread_rolled_back(rx: &async_channel::Receiver<Event>) -> ThreadRolledBackEvent {
    let deadline = StdDuration::from_secs(2);
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline.saturating_sub(start.elapsed());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        match evt.msg {
            EventMsg::ThreadRolledBack(payload) => return payload,
            _ => continue,
        }
    }
}

async fn wait_for_thread_rollback_failed(rx: &async_channel::Receiver<Event>) -> ErrorEvent {
    let deadline = StdDuration::from_secs(2);
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline.saturating_sub(start.elapsed());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        match evt.msg {
            EventMsg::Error(payload)
                if payload.codex_error_info == Some(CodexErrorInfo::ThreadRollbackFailed) =>
            {
                return payload;
            }
            _ => continue,
        }
    }
}

async fn attach_thread_persistence(session: &mut Session) -> PathBuf {
    let config = session.get_config().await;
    let live_thread = LiveThread::create(
        Arc::clone(&session.services.thread_store),
        CreateThreadParams {
            thread_id: session.conversation_id,
            forked_from_id: None,
            source: SessionSource::Exec,
            thread_source: None,
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            metadata: ThreadPersistenceMetadata {
                cwd: Some(config.cwd.to_path_buf()),
                model_provider: config.model_provider_id.clone(),
                memory_mode: if config.memories.generate_memories {
                    ThreadMemoryMode::Enabled
                } else {
                    ThreadMemoryMode::Disabled
                },
            },
            event_persistence_mode: ThreadEventPersistenceMode::Limited,
        },
    )
    .await
    .expect("create thread persistence");
    session.services.live_thread = Some(Arc::new(live_thread));
    session.ensure_rollout_materialized().await;
    session
        .flush_rollout()
        .await
        .expect("attached rollout should flush");
    session
        .current_rollout_path()
        .await
        .expect("load rollout path")
        .expect("thread should have rollout path")
}

fn text_block(s: &str) -> serde_json::Value {
    json!({
        "type": "text",
        "text": s,
    })
}

async fn build_test_config(codex_home: &Path) -> Config {
    ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.to_path_buf())
        .build()
        .await
        .expect("load default test config")
}

fn session_telemetry(
    conversation_id: ThreadId,
    config: &Config,
    model_info: &ModelInfo,
    session_source: SessionSource,
) -> SessionTelemetry {
    SessionTelemetry::new(
        conversation_id,
        get_model_offline_for_tests(config.model.as_deref()).as_str(),
        model_info.slug.as_str(),
        /*account_id*/ None,
        Some("test@test.com".to_string()),
        Some(TelemetryAuthMode::Chatgpt),
        "test_originator".to_string(),
        /*log_user_prompts*/ false,
        "test".to_string(),
        session_source,
    )
}

pub(crate) async fn make_session_configuration_for_tests() -> SessionConfiguration {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let config = build_test_config(codex_home.path()).await;
    let config = Arc::new(config);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort;
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };

    SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        context_budget_mode: config.context_budget_mode,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile: config.permissions.permission_profile.clone(),
        active_permission_profile: config.permissions.active_permission_profile(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: vec![config.cwd.clone()],
        profile_workspace_roots: Vec::new(),
        codex_home: config.codex_home.clone(),
        thread_name: None,
        environments: Vec::new(),
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        thread_source: None,
        dynamic_tools: Vec::new(),
        persist_extended_history: false,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    }
}

fn turn_environments_for_tests(
    environment: &Arc<codex_exec_server::Environment>,
    cwd: &codex_utils_absolute_path::AbsolutePathBuf,
) -> crate::environment_selection::ResolvedTurnEnvironments {
    crate::environment_selection::ResolvedTurnEnvironments {
        turn_environments: vec![TurnEnvironment {
            environment_id: codex_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
            environment: Arc::clone(environment),
            cwd: cwd.clone(),
            shell: None,
        }],
    }
}

// todo: use online model info
pub(crate) async fn make_session_and_context() -> (Session, TurnContext) {
    let (tx_event, _rx_event) = async_channel::unbounded();
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let config = build_test_config(codex_home.path()).await;
    let config = Arc::new(config);
    let thread_id = ThreadId::default();
    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let agent_control = AgentControl::default();
    let exec_policy = Arc::new(ExecPolicyManager::default());
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort;
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: codex_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        context_budget_mode: config.context_budget_mode,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile: config.permissions.permission_profile.clone(),
        active_permission_profile: config.permissions.active_permission_profile(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: vec![config.cwd.clone()],
        profile_workspace_roots: Vec::new(),
        codex_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        thread_source: None,
        dynamic_tools: Vec::new(),
        persist_extended_history: false,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };
    let per_turn_config =
        Session::build_per_turn_config(&session_configuration, session_configuration.cwd.clone());
    let model_info = construct_model_info_offline_for_tests(
        session_configuration.collaboration_mode.model(),
        &per_turn_config.to_models_manager_config(),
    );
    let session_telemetry = session_telemetry(
        thread_id,
        config.as_ref(),
        &model_info,
        session_configuration.session_source.clone(),
    );

    let state = SessionState::new(session_configuration.clone());
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));
    let network_approval = Arc::new(NetworkApprovalService::default());
    let environment = Arc::new(
        codex_exec_server::Environment::create_for_tests(/*exec_server_url*/ None)
            .expect("create environment"),
    );

    let services = SessionServices {
        mcp_connection_manager: Arc::new(RwLock::new(McpConnectionManager::new_uninitialized(
            &config.permissions.approval_policy,
            &config.permissions.permission_profile,
        ))),
        mcp_startup_cancellation_token: Mutex::new(CancellationToken::new()),
        unified_exec_manager: UnifiedExecProcessManager::new(
            config.background_terminal_max_timeout,
        ),
        shell_zsh_path: None,
        main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
        analytics_events_client: AnalyticsEventsClient::new(
            Arc::clone(&auth_manager),
            config.chatgpt_base_url.trim_end_matches('/').to_string(),
            config.analytics_enabled,
        ),
        hooks: arc_swap::ArcSwap::from_pointee(Hooks::new(HooksConfig {
            legacy_notify_argv: config.notify.clone(),
            ..HooksConfig::default()
        })),
        rollout_thread_trace: codex_rollout_trace::ThreadTraceContext::disabled(),
        user_shell: Arc::new(default_user_shell()),
        shell_snapshot_tx: watch::channel(None).0,
        show_raw_agent_reasoning: config.show_raw_agent_reasoning,
        exec_policy,
        auth_manager: auth_manager.clone(),
        session_telemetry: session_telemetry.clone(),
        models_manager: Arc::clone(&models_manager),
        tool_approvals: Mutex::new(ApprovalStore::default()),
        guardian_rejections: Mutex::new(std::collections::HashMap::new()),
        guardian_rejection_circuit_breaker: Mutex::new(Default::default()),
        runtime_handle: tokio::runtime::Handle::current(),
        skills_manager,
        plugins_manager,
        mcp_manager,
        extensions: Arc::new(codex_extension_api::ExtensionRegistryBuilder::new().build()),
        session_extension_data: codex_extension_api::ExtensionData::new("session"),
        thread_extension_data: codex_extension_api::ExtensionData::new("thread"),
        agent_control,
        network_proxy: None,
        network_approval: Arc::clone(&network_approval),
        state_db: None,
        live_thread: None,
        thread_store: Arc::new(RecordingThreadStore::default()),
        live_thread_factory: Arc::new(RecordingLiveThreadFactory::new()),
        attestation_provider: None,
        model_client: ModelClient::new(
            Some(auth_manager.clone()),
            thread_id.into(),
            thread_id,
            /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
            session_configuration.provider.clone(),
            session_configuration.session_source.clone(),
            config.model_verbosity,
            config.features.enabled(Feature::EnableRequestCompression),
            config.features.enabled(Feature::RuntimeMetrics),
            Session::build_model_client_beta_features_header(config.as_ref()),
            /*attestation_provider*/ None,
        ),
        code_mode_service: crate::tools::code_mode::CodeModeService::new(),
        blackboard: new_blackboard_session(
            config.as_ref(),
            SessionId::from(thread_id).to_string(),
            thread_id.to_string(),
            &session_configuration.session_source,
        ),
        environment_manager: Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    };

    let plugin_outcome = services
        .plugins_manager
        .plugins_for_config(&per_turn_config.plugins_config_input())
        .await;
    let effective_skill_roots = plugin_outcome.effective_plugin_skill_roots();
    let skills_input =
        crate::skills_load_input_from_config(&per_turn_config, effective_skill_roots);
    let skill_fs = environment.get_filesystem();
    let skills_outcome = Arc::new(
        services
            .skills_manager
            .skills_for_config(&skills_input, Some(Arc::clone(&skill_fs)))
            .await,
    );
    let turn_environments = turn_environments_for_tests(&environment, &session_configuration.cwd);
    let turn_context = Session::make_turn_context(
        thread_id,
        SessionId::from(thread_id),
        Some(Arc::clone(&auth_manager)),
        &session_telemetry,
        session_configuration.provider.clone(),
        &session_configuration,
        services.user_shell.as_ref(),
        services.shell_zsh_path.as_ref(),
        services.main_execve_wrapper_exe.as_ref(),
        per_turn_config,
        model_info,
        &models_manager,
        /*network*/ None,
        turn_environments,
        session_configuration.cwd.clone(),
        "turn_id".to_string(),
        skills_outcome,
        /*goal_tools_supported*/ true,
    );

    let (mailbox, mailbox_rx) = crate::agent::Mailbox::new();
    let session = Session {
        conversation_id: thread_id,
        installation_id: "11111111-1111-4111-8111-111111111111".to_string(),
        tx_event,
        agent_status: agent_status_tx,
        out_of_band_elicitation_paused: watch::channel(false).0,
        state: Mutex::new(state),
        managed_network_proxy_refresh_lock: Semaphore::new(/*permits*/ 1),
        features: config.features.clone(),
        pending_mcp_server_refresh_config: Mutex::new(None),
        conversation: Arc::new(RealtimeConversationManager::new()),
        active_turn: Mutex::new(None),
        mailbox,
        mailbox_rx: Mutex::new(mailbox_rx),
        idle_pending_input: Mutex::new(Vec::new()),
        input_queue: crate::session::InputQueue::new(),
        goal_runtime: crate::goals::GoalRuntimeState::new(),
        guardian_review_session: crate::guardian::GuardianReviewSessionManager::default(),
        services,
        next_internal_sub_id: AtomicU64::new(0),
    };

    (session, turn_context)
}

async fn make_session_with_config(
    mutator: impl FnOnce(&mut Config),
) -> anyhow::Result<Arc<Session>> {
    let (session, _rx_event) = make_session_with_config_and_rx(mutator).await?;
    Ok(session)
}

async fn load_latest_config_for_session(session: &Session) -> Config {
    let config = session.get_config().await;
    ConfigBuilder::default()
        .codex_home(config.codex_home.to_path_buf())
        .fallback_cwd(Some(config.cwd.to_path_buf()))
        .build()
        .await
        .expect("load latest config for session")
}

async fn make_session_with_config_and_rx(
    mutator: impl FnOnce(&mut Config),
) -> anyhow::Result<(Arc<Session>, async_channel::Receiver<Event>)> {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let mut config = build_test_config(codex_home.path()).await;
    mutator(&mut config);
    let config = Arc::new(config);
    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort: config.model_reasoning_effort,
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: codex_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        context_budget_mode: config.context_budget_mode,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile: config.permissions.permission_profile.clone(),
        active_permission_profile: config.permissions.active_permission_profile(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: vec![config.cwd.clone()],
        profile_workspace_roots: Vec::new(),
        codex_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        thread_source: None,
        dynamic_tools: Vec::new(),
        persist_extended_history: false,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let (tx_event, rx_event) = async_channel::unbounded();
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));

    let session = Session::new(
        session_configuration,
        Arc::clone(&config),
        "11111111-1111-4111-8111-111111111111".to_string(),
        auth_manager,
        models_manager,
        Arc::new(ExecPolicyManager::default()),
        tx_event,
        agent_status_tx,
        InitialHistory::New,
        SessionSource::Exec,
        skills_manager,
        plugins_manager,
        mcp_manager,
        Arc::new(codex_extension_api::ExtensionRegistryBuilder::new().build()),
        AgentControl::default(),
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        /*analytics_events_client*/ None,
        Arc::new(RecordingThreadStore::default()),
        Arc::new(RecordingLiveThreadFactory::new()),
        /*state_db*/ None,
        codex_rollout_trace::ThreadTraceContext::disabled(),
        /*attestation_provider*/ None,
    )
    .await?;

    Ok((session, rx_event))
}

async fn make_session_with_history_source_and_agent_control_and_rx(
    initial_history: InitialHistory,
    session_source: SessionSource,
    agent_control: AgentControl,
) -> anyhow::Result<(Arc<Session>, async_channel::Receiver<Event>)> {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let mut config = build_test_config(codex_home.path()).await;
    config.ephemeral = true;
    let config = Arc::new(config);
    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort: config.model_reasoning_effort,
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: codex_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        context_budget_mode: config.context_budget_mode,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile: config.permissions.permission_profile.clone(),
        active_permission_profile: config.permissions.active_permission_profile(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: vec![config.cwd.clone()],
        profile_workspace_roots: Vec::new(),
        codex_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: session_source.clone(),
        thread_source: None,
        dynamic_tools: Vec::new(),
        persist_extended_history: false,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let (tx_event, rx_event) = async_channel::unbounded();
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));

    let session = Session::new(
        session_configuration,
        Arc::clone(&config),
        "11111111-1111-4111-8111-111111111111".to_string(),
        auth_manager,
        models_manager,
        Arc::new(ExecPolicyManager::default()),
        tx_event,
        agent_status_tx,
        initial_history,
        session_source,
        skills_manager,
        plugins_manager,
        mcp_manager,
        Arc::new(codex_extension_api::ExtensionRegistryBuilder::new().build()),
        agent_control,
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        /*analytics_events_client*/ None,
        Arc::new(RecordingThreadStore::default()),
        Arc::new(RecordingLiveThreadFactory::new()),
        Some(
            codex_state::StateRuntime::init(
                config.sqlite_home.clone(),
                config.model_provider_id.clone(),
            )
            .await
            .expect("state db should initialize"),
        ),
        codex_rollout_trace::ThreadTraceContext::disabled(),
        /*attestation_provider*/ None,
    )
    .await?;

    Ok((session, rx_event))
}

async fn make_session_and_context_with_auth_and_config_and_rx<F>(
    auth: CodexAuth,
    dynamic_tools: Vec<DynamicToolSpec>,
    configure_config: F,
) -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
)
where
    F: FnOnce(&mut Config),
{
    let codex_home = tempfile::tempdir().expect("create temp dir");
    make_session_and_context_with_auth_config_home_and_rx(
        auth,
        dynamic_tools,
        codex_home.path(),
        configure_config,
    )
    .await
}

async fn make_session_and_context_with_auth_config_home_and_rx<F>(
    auth: CodexAuth,
    dynamic_tools: Vec<DynamicToolSpec>,
    codex_home: &Path,
    configure_config: F,
) -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
)
where
    F: FnOnce(&mut Config),
{
    let (tx_event, rx_event) = async_channel::unbounded();
    let mut config = build_test_config(codex_home).await;
    configure_config(&mut config);
    let state_db = if config.features.enabled(Feature::Goals) {
        Some(
            codex_state::StateRuntime::init(
                config.sqlite_home.clone(),
                config.model_provider_id.clone(),
            )
            .await
            .expect("goal tests should initialize sqlite state db"),
        )
    } else {
        None
    };
    let config = Arc::new(config);
    let thread_id = ThreadId::default();
    let auth_manager = AuthManager::from_auth_for_testing(auth);
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let agent_control = AgentControl::default();
    let exec_policy = Arc::new(ExecPolicyManager::default());
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort;
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: codex_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        context_budget_mode: config.context_budget_mode,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile: config.permissions.permission_profile.clone(),
        active_permission_profile: config.permissions.active_permission_profile(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: vec![config.cwd.clone()],
        profile_workspace_roots: Vec::new(),
        codex_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        thread_source: None,
        dynamic_tools,
        persist_extended_history: false,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };
    let per_turn_config =
        Session::build_per_turn_config(&session_configuration, session_configuration.cwd.clone());
    let model_info = construct_model_info_offline_for_tests(
        session_configuration.collaboration_mode.model(),
        &per_turn_config.to_models_manager_config(),
    );
    let session_telemetry = session_telemetry(
        thread_id,
        config.as_ref(),
        &model_info,
        session_configuration.session_source.clone(),
    );

    let state = SessionState::new(session_configuration.clone());
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));
    let network_approval = Arc::new(NetworkApprovalService::default());
    let environment = Arc::new(
        codex_exec_server::Environment::create_for_tests(/*exec_server_url*/ None)
            .expect("create environment"),
    );

    let services = SessionServices {
        mcp_connection_manager: Arc::new(RwLock::new(McpConnectionManager::new_uninitialized(
            &config.permissions.approval_policy,
            &config.permissions.permission_profile,
        ))),
        mcp_startup_cancellation_token: Mutex::new(CancellationToken::new()),
        unified_exec_manager: UnifiedExecProcessManager::new(
            config.background_terminal_max_timeout,
        ),
        shell_zsh_path: None,
        main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
        analytics_events_client: AnalyticsEventsClient::new(
            Arc::clone(&auth_manager),
            config.chatgpt_base_url.trim_end_matches('/').to_string(),
            config.analytics_enabled,
        ),
        hooks: arc_swap::ArcSwap::from_pointee(Hooks::new(HooksConfig {
            legacy_notify_argv: config.notify.clone(),
            ..HooksConfig::default()
        })),
        rollout_thread_trace: codex_rollout_trace::ThreadTraceContext::disabled(),
        user_shell: Arc::new(default_user_shell()),
        shell_snapshot_tx: watch::channel(None).0,
        show_raw_agent_reasoning: config.show_raw_agent_reasoning,
        exec_policy,
        auth_manager: Arc::clone(&auth_manager),
        session_telemetry: session_telemetry.clone(),
        models_manager: Arc::clone(&models_manager),
        tool_approvals: Mutex::new(ApprovalStore::default()),
        guardian_rejections: Mutex::new(std::collections::HashMap::new()),
        guardian_rejection_circuit_breaker: Mutex::new(Default::default()),
        runtime_handle: tokio::runtime::Handle::current(),
        skills_manager,
        plugins_manager,
        mcp_manager,
        extensions: Arc::new(codex_extension_api::ExtensionRegistryBuilder::new().build()),
        session_extension_data: codex_extension_api::ExtensionData::new("session"),
        thread_extension_data: codex_extension_api::ExtensionData::new("thread"),
        agent_control,
        network_proxy: None,
        network_approval: Arc::clone(&network_approval),
        state_db: state_db.clone(),
        live_thread: None,
        thread_store: Arc::new(RecordingThreadStore::default()),
        live_thread_factory: Arc::new(RecordingLiveThreadFactory::new()),
        attestation_provider: None,
        model_client: ModelClient::new(
            Some(Arc::clone(&auth_manager)),
            thread_id.into(),
            thread_id,
            /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
            session_configuration.provider.clone(),
            session_configuration.session_source.clone(),
            config.model_verbosity,
            config.features.enabled(Feature::EnableRequestCompression),
            config.features.enabled(Feature::RuntimeMetrics),
            Session::build_model_client_beta_features_header(config.as_ref()),
            /*attestation_provider*/ None,
        ),
        code_mode_service: crate::tools::code_mode::CodeModeService::new(),
        blackboard: new_blackboard_session(
            config.as_ref(),
            SessionId::from(thread_id).to_string(),
            thread_id.to_string(),
            &session_configuration.session_source,
        ),
        environment_manager: Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    };

    let plugin_outcome = services
        .plugins_manager
        .plugins_for_config(&per_turn_config.plugins_config_input())
        .await;
    let effective_skill_roots = plugin_outcome.effective_plugin_skill_roots();
    let skills_input =
        crate::skills_load_input_from_config(&per_turn_config, effective_skill_roots);
    let skill_fs = environment.get_filesystem();
    let skills_outcome = Arc::new(
        services
            .skills_manager
            .skills_for_config(&skills_input, Some(Arc::clone(&skill_fs)))
            .await,
    );
    let turn_environments = turn_environments_for_tests(&environment, &session_configuration.cwd);
    let turn_context = Arc::new(Session::make_turn_context(
        thread_id,
        SessionId::from(thread_id),
        Some(Arc::clone(&auth_manager)),
        &session_telemetry,
        session_configuration.provider.clone(),
        &session_configuration,
        services.user_shell.as_ref(),
        services.shell_zsh_path.as_ref(),
        services.main_execve_wrapper_exe.as_ref(),
        per_turn_config,
        model_info,
        &models_manager,
        /*network*/ None,
        turn_environments,
        session_configuration.cwd.clone(),
        "turn_id".to_string(),
        skills_outcome,
        /*goal_tools_supported*/ true,
    ));

    let (mailbox, mailbox_rx) = crate::agent::Mailbox::new();
    let session = Arc::new(Session {
        conversation_id: thread_id,
        installation_id: "11111111-1111-4111-8111-111111111111".to_string(),
        tx_event,
        agent_status: agent_status_tx,
        out_of_band_elicitation_paused: watch::channel(false).0,
        state: Mutex::new(state),
        managed_network_proxy_refresh_lock: Semaphore::new(/*permits*/ 1),
        features: config.features.clone(),
        pending_mcp_server_refresh_config: Mutex::new(None),
        conversation: Arc::new(RealtimeConversationManager::new()),
        active_turn: Mutex::new(None),
        mailbox,
        mailbox_rx: Mutex::new(mailbox_rx),
        idle_pending_input: Mutex::new(Vec::new()),
        input_queue: crate::session::InputQueue::new(),
        goal_runtime: crate::goals::GoalRuntimeState::new(),
        guardian_review_session: crate::guardian::GuardianReviewSessionManager::default(),
        services,
        next_internal_sub_id: AtomicU64::new(0),
    });

    (session, turn_context, rx_event)
}

pub(crate) async fn make_session_and_context_with_dynamic_tools_and_rx(
    dynamic_tools: Vec<DynamicToolSpec>,
) -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
) {
    make_session_and_context_with_auth_and_config_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        dynamic_tools,
        |_config| {},
    )
    .await
}

async fn make_goal_session_and_context_with_rx() -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
    tempfile::TempDir,
) {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let (session, turn_context, rx) = make_session_and_context_with_auth_config_home_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        Vec::new(),
        codex_home.path(),
        |config| {
            config
                .features
                .enable(Feature::Goals)
                .expect("goal mode should be enableable in tests");
        },
    )
    .await;
    upsert_goal_test_thread(session.as_ref()).await;
    (session, turn_context, rx, codex_home)
}

async fn upsert_goal_test_thread(session: &Session) {
    let config = session.get_config().await;
    let state_db = session
        .state_db()
        .expect("goal test session should have a state db");
    let mut builder = codex_state::ThreadMetadataBuilder::new(
        session.conversation_id,
        config
            .codex_home
            .join("goal-test-rollout.jsonl")
            .to_path_buf(),
        chrono::Utc::now(),
        SessionSource::Cli,
    );
    builder.cwd = config.cwd.to_path_buf();
    builder.model_provider = Some(config.model_provider_id.clone());
    let metadata = builder.build(config.model_provider_id.as_str());
    state_db
        .upsert_thread(&metadata)
        .await
        .expect("goal test thread should be upserted");
}

// Like make_session_and_context, but returns Arc<Session> and the event receiver
// so tests can assert on emitted events.
pub(crate) async fn make_session_and_context_with_rx() -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
) {
    make_session_and_context_with_dynamic_tools_and_rx(Vec::new()).await
}

async fn make_multi_agent_v2_usage_hint_test_session(
    enable_multi_agent_v2: bool,
) -> (Arc<Session>, Arc<TurnContext>) {
    let (session, turn_context, _rx_event) = make_session_and_context_with_auth_and_config_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        Vec::new(),
        |config| {
            if enable_multi_agent_v2 {
                let _ = config.features.enable(Feature::MultiAgentV2);
            } else {
                let _ = config.features.disable(Feature::MultiAgentV2);
            }
            config.multi_agent_v2.root_agent_usage_hint_text = Some("Root guidance.".to_string());
            config.multi_agent_v2.subagent_usage_hint_text = Some("Subagent guidance.".to_string());
        },
    )
    .await;
    (session, turn_context)
}

struct GitAttributionTestContributor;
struct GitAttributionTestState;

impl codex_extension_api::ContextContributor for GitAttributionTestContributor {
    fn contribute<'a>(
        &'a self,
        _session_store: &'a codex_extension_api::ExtensionData,
        thread_store: &'a codex_extension_api::ExtensionData,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Vec<codex_extension_api::PromptFragment>> + Send + 'a>,
    > {
        let enabled = thread_store.get::<GitAttributionTestState>().is_some();
        Box::pin(async move {
            enabled
                .then(|| {
                    codex_extension_api::PromptFragment::developer_policy(
                        "git attribution extension enabled",
                    )
                })
                .into_iter()
                .collect()
        })
    }
}

fn git_attribution_test_registry()
-> Arc<codex_extension_api::ExtensionRegistry<crate::config::Config>> {
    let mut builder = codex_extension_api::ExtensionRegistryBuilder::new();
    builder.prompt_contributor(Arc::new(GitAttributionTestContributor));
    Arc::new(builder.build())
}

fn file_system_policy_with_unreadable_glob(turn_context: &TurnContext) -> FileSystemSandboxPolicy {
    let mut policy = FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
        &turn_context.sandbox_policy(),
        &turn_context.cwd,
    );
    policy.entries.push(FileSystemSandboxEntry {
        path: FileSystemPath::GlobPattern {
            pattern: format!("{}/**/*.env", turn_context.cwd.as_path().display()),
        },
        access: FileSystemAccessMode::None,
    });
    policy
}

#[derive(Clone, Copy)]
struct NeverEndingTask {
    kind: TaskKind,
    listen_to_cancellation_token: bool,
}

impl SessionTask for NeverEndingTask {
    fn kind(&self) -> TaskKind {
        self.kind
    }

    fn span_name(&self) -> &'static str {
        "session_task.never_ending"
    }

    async fn run(
        self: Arc<Self>,
        _session: Arc<SessionTaskContext>,
        _ctx: Arc<TurnContext>,
        _input: Vec<UserInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        if self.listen_to_cancellation_token {
            cancellation_token.cancelled().await;
            return None;
        }
        loop {
            sleep(Duration::from_secs(60)).await;
        }
    }
}

#[derive(Clone, Copy)]
struct GuardianDeniedApprovalTask;

impl SessionTask for GuardianDeniedApprovalTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.guardian_denied_approval"
    }

    async fn run(
        self: Arc<Self>,
        session: Arc<SessionTaskContext>,
        ctx: Arc<TurnContext>,
        _input: Vec<UserInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        let session = session.clone_session();
        for _ in 0..3 {
            crate::guardian::record_guardian_denial_for_test(&session, &ctx, &ctx.sub_id).await;
        }

        cancellation_token.cancelled().await;
        None
    }
}

async fn set_total_token_usage(sess: &Session, total_token_usage: TokenUsage) {
    let mut state = sess.state.lock().await;
    state.set_token_info(Some(TokenUsageInfo {
        total_token_usage,
        last_token_usage: TokenUsage::default(),
        model_context_window: None,
    }));
}

fn post_goal_token_usage() -> TokenUsage {
    TokenUsage {
        input_tokens: 50,
        cached_input_tokens: 10,
        output_tokens: 30,
        reasoning_output_tokens: 5,
        total_tokens: 75,
    }
}

async fn goal_test_state_db(sess: &Session) -> anyhow::Result<crate::StateDbHandle> {
    if let Some(state_db) = sess.state_db() {
        return Ok(state_db);
    }
    let config = sess.get_config().await;
    codex_state::StateRuntime::init(config.sqlite_home.clone(), config.model_provider_id.clone())
        .await
}

async fn sample_rollout(
    session: &Session,
    _turn_context: &TurnContext,
) -> (Vec<RolloutItem>, Vec<ResponseItem>) {
    let mut rollout_items = Vec::new();
    let mut live_history = ContextManager::new();

    // Use the same turn_context source as record_initial_history so model_info (and thus
    // personality_spec) matches reconstruction.
    let reconstruction_turn = session.new_default_turn().await;
    let mut initial_context = session
        .build_initial_context(reconstruction_turn.as_ref())
        .await;
    // Ensure personality_spec is present when Personality is enabled, so expected matches
    // what reconstruction produces (build_initial_context may omit it when baked into model).
    if !initial_context.iter().any(|m| {
        matches!(m, ResponseItem::Message { role, content, .. }
        if role == "developer"
            && content.iter().any(|c| {
                matches!(c, ContentItem::InputText { text } if text.contains("<personality_spec>"))
            }))
    }) && let Some(p) = reconstruction_turn.personality
        && session.features.enabled(Feature::Personality)
        && let Some(personality_message) = reconstruction_turn
            .model_info
            .model_messages
            .as_ref()
            .and_then(|m| m.get_personality_message(Some(p)).filter(|s| !s.is_empty()))
    {
        let msg = crate::context::ContextualUserFragment::into(
            crate::context::PersonalitySpecInstructions::new(personality_message),
        );
        let insert_at = initial_context
            .iter()
            .position(|m| matches!(m, ResponseItem::Message { role, .. } if role == "developer"))
            .map(|i| i + 1)
            .unwrap_or(0);
        initial_context.insert(insert_at, msg);
    }
    for item in &initial_context {
        rollout_items.push(RolloutItem::ResponseItem(item.clone()));
    }
    live_history.record_items(
        initial_context.iter(),
        reconstruction_turn.truncation_policy,
    );

    let user1 = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "first user".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&user1),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(user1.clone()));

    let assistant1 = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: "assistant reply one".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&assistant1),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(assistant1.clone()));

    let summary1 = "summary one";
    let snapshot1 = live_history
        .clone()
        .for_prompt(&reconstruction_turn.model_info.input_modalities);
    let user_messages1 = collect_user_messages(&snapshot1);
    let rebuilt1 = compact::build_compacted_history(Vec::new(), &user_messages1, summary1);
    live_history.replace(rebuilt1);
    rollout_items.push(RolloutItem::Compacted(CompactedItem {
        message: summary1.to_string(),
        replacement_history: None,
    }));

    let user2 = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "second user".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&user2),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(user2.clone()));

    let assistant2 = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: "assistant reply two".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&assistant2),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(assistant2.clone()));

    let summary2 = "summary two";
    let snapshot2 = live_history
        .clone()
        .for_prompt(&reconstruction_turn.model_info.input_modalities);
    let user_messages2 = collect_user_messages(&snapshot2);
    let rebuilt2 = compact::build_compacted_history(Vec::new(), &user_messages2, summary2);
    live_history.replace(rebuilt2);
    rollout_items.push(RolloutItem::Compacted(CompactedItem {
        message: summary2.to_string(),
        replacement_history: None,
    }));

    let user3 = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "third user".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&user3),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(user3));

    let assistant3 = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: "assistant reply three".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&assistant3),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(assistant3));

    (
        rollout_items,
        live_history.for_prompt(&reconstruction_turn.model_info.input_modalities),
    )
}
