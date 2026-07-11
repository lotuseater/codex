//! Test-only helpers exposed for cross-crate integration tests.
//!
//! Production code should not depend on this module.
//! We prefer this to using a crate feature to avoid building multiple
//! permutations of the crate.

use std::path::PathBuf;

use codex_exec_server::EnvironmentManager;
use codex_extension_api::LoadUserInstructionsFuture;
use codex_extension_api::LoadedUserInstructions;
use codex_extension_api::UserInstructionsProvider;
use codex_features::Feature;
use codex_features::Features;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_provider::create_model_provider;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::bundled_models_response;
use codex_models_manager::collaboration_mode_presets;
use codex_models_manager::manager::SharedModelsManager;
use codex_models_manager::test_support::construct_model_info_offline_for_tests;
use codex_models_manager::test_support::get_model_offline_for_tests;
use codex_protocol::ThreadId;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceTool;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::protocol::SessionSource;
use codex_tool_execution_api::ToolsConfig;
use codex_tool_execution_api::ToolsConfigParams;
use codex_tool_registry_api::LoadableToolSpec;
use codex_tool_registry_api::ResponsesApiNamespaceTool;
use codex_tool_registry_api::ToolSpec;
use once_cell::sync::Lazy;
use serde_json::json;
use std::sync::Arc;

use crate::ThreadManager;
use crate::config::Config;
use crate::responses_metadata::CodexResponsesMetadata;
use crate::responses_metadata::CodexResponsesRequestKind;
use crate::responses_metadata::subagent_header_value;
use crate::responses_metadata::subagent_metadata_kind;
use crate::thread_manager;
use crate::tools::handlers::multi_agents_spec::WaitAgentTimeoutOptions;
use crate::tools::spec_plan::alias_hosted_reserved_namespace_executors;
use crate::tools::spec_plan::build_tool_registry_builder_from_executors;
use crate::tools::spec_plan::collect_tool_executors;
use crate::tools::spec_plan::hosted_model_tool_specs;
use crate::tools::spec_plan_types::ToolRegistryBuildParams;
use crate::unified_exec;

static TEST_MODEL_PRESETS: Lazy<Vec<ModelPreset>> = Lazy::new(|| {
    let mut response = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    response.models.sort_by_key(|model| model.priority);
    let mut presets: Vec<ModelPreset> = response.models.into_iter().map(Into::into).collect();
    ModelPreset::mark_default_by_picker_visibility(&mut presets);
    presets
});

/// Test-only provider that supplies no user instructions.
#[derive(Debug, Default)]
pub struct EmptyUserInstructionsProvider;

impl UserInstructionsProvider for EmptyUserInstructionsProvider {
    fn load_user_instructions(&self) -> LoadUserInstructionsFuture<'_> {
        Box::pin(async { LoadedUserInstructions::default() })
    }
}

pub fn set_thread_manager_test_mode(enabled: bool) {
    thread_manager::set_thread_manager_test_mode_for_tests(enabled);
}

pub fn set_deterministic_process_ids(enabled: bool) {
    unified_exec::set_deterministic_process_ids_for_tests(enabled);
}

pub fn auth_manager_from_auth(auth: CodexAuth) -> Arc<AuthManager> {
    AuthManager::from_auth_for_testing(auth)
}

pub fn auth_manager_from_auth_with_home(auth: CodexAuth, codex_home: PathBuf) -> Arc<AuthManager> {
    AuthManager::from_auth_for_testing_with_home(auth, codex_home)
}

pub fn with_code_mode_host_program(
    thread_manager: ThreadManager,
    host_program: PathBuf,
) -> ThreadManager {
    thread_manager.with_code_mode_host_program_for_tests(host_program)
}

pub fn thread_manager_with_models_provider(
    auth: CodexAuth,
    provider: ModelProviderInfo,
) -> ThreadManager {
    ThreadManager::with_models_provider_for_tests(auth, provider)
}

pub fn thread_manager_with_models_provider_and_home(
    auth: CodexAuth,
    provider: ModelProviderInfo,
    codex_home: PathBuf,
    environment_manager: Arc<EnvironmentManager>,
) -> ThreadManager {
    ThreadManager::with_models_provider_and_home_for_tests(
        auth,
        provider,
        codex_home,
        environment_manager,
    )
}

pub fn thread_manager_with_models_provider_home_and_state(
    auth: CodexAuth,
    provider: ModelProviderInfo,
    codex_home: PathBuf,
    environment_manager: Arc<EnvironmentManager>,
    state_db: Option<crate::StateDbHandle>,
) -> ThreadManager {
    ThreadManager::with_models_provider_home_and_state_for_tests(
        auth,
        provider,
        codex_home,
        environment_manager,
        state_db,
    )
}

pub async fn start_thread_with_user_shell_override(
    thread_manager: &ThreadManager,
    config: Config,
    user_shell_override: crate::shell::Shell,
    supports_openai_form_elicitation: bool,
) -> codex_protocol::error::Result<crate::NewThread> {
    thread_manager
        .start_thread_with_user_shell_override_for_tests(
            config,
            user_shell_override,
            supports_openai_form_elicitation,
        )
        .await
}

pub async fn resume_thread_from_rollout_with_user_shell_override(
    thread_manager: &ThreadManager,
    config: Config,
    rollout_path: PathBuf,
    auth_manager: Arc<AuthManager>,
    user_shell_override: crate::shell::Shell,
    supports_openai_form_elicitation: bool,
) -> codex_protocol::error::Result<crate::NewThread> {
    thread_manager
        .resume_thread_from_rollout_with_user_shell_override_for_tests(
            config,
            rollout_path,
            auth_manager,
            user_shell_override,
            supports_openai_form_elicitation,
        )
        .await
}

pub fn models_manager_with_provider(
    codex_home: PathBuf,
    auth_manager: Arc<AuthManager>,
    provider: ModelProviderInfo,
) -> SharedModelsManager {
    let provider = create_model_provider(provider, Some(auth_manager));
    provider.models_manager(codex_home, /*config_model_catalog*/ None)
}

pub fn default_http_client_factory() -> HttpClientFactory {
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
}

pub fn get_model_offline(model: Option<&str>) -> String {
    get_model_offline_for_tests(model)
}

pub fn construct_model_info_offline(model: &str, config: &Config) -> ModelInfo {
    construct_model_info_offline_for_tests(model, &config.to_models_manager_config())
}

#[derive(Clone, Copy)]
pub enum TestCodexResponsesRequestKind {
    Turn,
    Prewarm,
    WebsocketConnection,
}

#[allow(clippy::too_many_arguments)]
pub fn responses_metadata(
    installation_id: &str,
    session_id: &str,
    thread_id: &str,
    turn_id: Option<&str>,
    window_id: String,
    session_source: &SessionSource,
    parent_thread_id: Option<ThreadId>,
    request_kind: TestCodexResponsesRequestKind,
) -> CodexResponsesMetadata {
    let request_kind = match request_kind {
        TestCodexResponsesRequestKind::Turn => Some(CodexResponsesRequestKind::Turn),
        TestCodexResponsesRequestKind::Prewarm => Some(CodexResponsesRequestKind::Prewarm),
        TestCodexResponsesRequestKind::WebsocketConnection => None,
    };
    CodexResponsesMetadata {
        turn_id: request_kind.and(turn_id.map(ToString::to_string)),
        request_kind,
        parent_thread_id,
        subagent_header: subagent_header_value(session_source),
        subagent_kind: request_kind.and_then(|_| subagent_metadata_kind(session_source)),
        ..CodexResponsesMetadata::new(
            installation_id.to_string(),
            session_id.to_string(),
            thread_id.to_string(),
            window_id,
        )
    }
}

pub fn all_model_presets() -> &'static Vec<ModelPreset> {
    &TEST_MODEL_PRESETS
}

pub fn builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    collaboration_mode_presets::builtin_collaboration_mode_presets()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceAliasProbe {
    pub model_tool_names: Vec<String>,
    pub namespace_children: Vec<(String, Vec<String>)>,
    pub executor_tool_names: Vec<(Option<String>, String)>,
    pub deferred_search_namespaces: Vec<String>,
    pub deferred_search_children: Vec<(String, Vec<String>)>,
}

pub fn hosted_web_namespace_alias_probe(
    defer_loading: bool,
    namespace_tools: bool,
    tool_search: bool,
) -> NamespaceAliasProbe {
    const SOURCE_WEB_NAMESPACE: &str = "web";
    const WEB_TOOL_NAME: &str = "open";

    // `DynamicToolSpec` is now an enum (Function | Namespace); a namespaced tool is
    // modeled as a `Namespace` variant holding `Function` tools (mirrors
    // `group_dynamic_tools_by_namespace` and router_tests.rs).
    let dynamic_tools = vec![DynamicToolSpec::Namespace(DynamicToolNamespaceSpec {
        name: SOURCE_WEB_NAMESPACE.to_string(),
        description: String::new(),
        tools: vec![DynamicToolNamespaceTool::Function(
            DynamicToolFunctionSpec {
                name: WEB_TOOL_NAME.to_string(),
                description: "Open a page.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" }
                    },
                    "required": ["url"],
                    "additionalProperties": false
                }),
                defer_loading,
            },
        )],
    })];

    let mut features = Features::with_defaults();
    if tool_search {
        features.enable(Feature::ToolSearch);
    }

    let mut tools_config = ToolsConfig::new(&ToolsConfigParams {
        model_info: &bundled_model_info("gpt-5.4"),
        available_models: &[],
        features: &features,
        image_generation_tool_auth_allowed: true,
        web_search_mode: Some(WebSearchMode::Cached),
        session_source: SessionSource::Cli,
        permission_profile: &PermissionProfile::Disabled,
        windows_sandbox_level: WindowsSandboxLevel::Disabled,
    });
    tools_config.namespace_tools = namespace_tools;

    let hosted_specs = hosted_model_tool_specs(&tools_config);
    let params = ToolRegistryBuildParams {
        mcp_tools: None,
        deferred_mcp_tools: None,
        discoverable_tools: None,
        extension_tool_executors: &[],
        dynamic_tools: &dynamic_tools,
        default_agent_type_description: "Test agent type description.",
        wait_agent_timeouts: WaitAgentTimeoutOptions::default(),
    };
    let executors = alias_hosted_reserved_namespace_executors(
        collect_tool_executors(&tools_config, params),
        &hosted_specs,
    );

    let executor_tool_names = executors
        .iter()
        .map(|executor| {
            let name = executor.tool_name();
            (name.namespace, name.name)
        })
        .collect();

    let mut deferred_search_namespaces = Vec::new();
    let mut deferred_search_children = Vec::new();
    for search_info in executors
        .iter()
        .filter_map(|executor| executor.search_info())
    {
        if let LoadableToolSpec::Namespace(namespace) = search_info.entry.output {
            let children = namespace_child_names(&namespace.tools);
            deferred_search_namespaces.push(namespace.name.clone());
            deferred_search_children.push((namespace.name, children));
        }
    }

    let (model_specs, _) =
        build_tool_registry_builder_from_executors(&tools_config, executors, hosted_specs).build();

    NamespaceAliasProbe {
        model_tool_names: model_specs
            .iter()
            .map(|spec| spec.name().to_string())
            .collect(),
        namespace_children: model_specs
            .iter()
            .filter_map(|spec| match spec {
                ToolSpec::Namespace(namespace) => Some((
                    namespace.name.clone(),
                    namespace_child_names(&namespace.tools),
                )),
                _ => None,
            })
            .collect(),
        executor_tool_names,
        deferred_search_namespaces,
        deferred_search_children,
    }
}

fn bundled_model_info(slug: &str) -> ModelInfo {
    bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"))
        .models
        .into_iter()
        .find(|model| model.slug == slug)
        .unwrap_or_else(|| panic!("bundled models.json should include {slug}"))
}

fn namespace_child_names(tools: &[ResponsesApiNamespaceTool]) -> Vec<String> {
    tools
        .iter()
        .map(|tool| match tool {
            ResponsesApiNamespaceTool::Function(tool) => tool.name.clone(),
        })
        .collect()
}
