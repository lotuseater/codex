use crate::agent::exceeds_thread_spawn_depth_limit;
use crate::agent::next_thread_spawn_depth;
use crate::session::step_context::StepContext;
use crate::session::turn_context::TurnContext;
use crate::tools::code_mode::execute_spec::create_code_mode_tool;
use crate::tools::context::ToolInvocation;
use crate::tools::effective_tool_mode;
use crate::tools::handlers::ApplyPatchHandler;
use crate::tools::handlers::CodeModeExecuteHandler;
use crate::tools::handlers::CodeModeWaitHandler;
// fork-local: handlers for fork-only tool families grafted into add_fork_tools.
use crate::tools::handlers::CognosOpsHandler;
use crate::tools::handlers::ContextOpsHandler;
use crate::tools::handlers::CreateGoalHandler;
use crate::tools::handlers::CurrentTimeHandler;
use crate::tools::handlers::DesktopAutomationHandler;
use crate::tools::handlers::DynamicToolHandler;
use crate::tools::handlers::ExecCommandHandler;
use crate::tools::handlers::ExecCommandHandlerOptions;
use crate::tools::handlers::FirstMovesHandler;
use crate::tools::handlers::GetContextRemainingHandler;
use crate::tools::handlers::GetGoalHandler;
use crate::tools::handlers::ListAvailablePluginsToInstallHandler;
use crate::tools::handlers::ListMcpResourceTemplatesHandler;
use crate::tools::handlers::ListMcpResourcesHandler;
use crate::tools::handlers::McpHandler;
use crate::tools::handlers::NewContextWindowHandler;
use crate::tools::handlers::PlanHandler;
use crate::tools::handlers::ReadMcpResourceHandler;
use crate::tools::handlers::RepoContextScoutHandler;
use crate::tools::handlers::RequestPermissionsHandler;
use crate::tools::handlers::RequestPluginInstallHandler;
use crate::tools::handlers::RequestUserInputHandler;
use crate::tools::handlers::ShellCommandHandler;
use crate::tools::handlers::ShellCommandHandlerOptions;
use crate::tools::handlers::SleepHandler;
use crate::tools::handlers::TestSyncHandler;
use crate::tools::handlers::ToolSearchHandler;
use crate::tools::handlers::ToolSearchHandlerCache;
use crate::tools::handlers::UpdateGoalHandler;
use crate::tools::handlers::ViewImageHandler;
use crate::tools::handlers::WaitForEnvironmentHandler;
use crate::tools::handlers::WriteStdinHandler;
use crate::tools::handlers::agent_jobs::ReportAgentJobResultHandler;
use crate::tools::handlers::agent_jobs::SpawnAgentsOnCsvHandler;
use crate::tools::handlers::extension_tools::ExtensionToolAdapter;
use crate::tools::handlers::multi_agents::CloseAgentHandler;
use crate::tools::handlers::multi_agents::ResumeAgentHandler;
use crate::tools::handlers::multi_agents::SendInputHandler;
use crate::tools::handlers::multi_agents::SpawnAgentHandler;
use crate::tools::handlers::multi_agents::WaitAgentHandler;
use crate::tools::handlers::multi_agents_spec::SpawnAgentToolOptions;
use crate::tools::handlers::multi_agents_spec::WaitAgentTimeoutOptions;
use crate::tools::handlers::multi_agents_v2::CloseAgentHandler as CloseAgentHandlerV2;
use crate::tools::handlers::multi_agents_v2::CompactAgentHandler as CompactAgentHandlerV2;
use crate::tools::handlers::multi_agents_v2::FollowupTaskHandler as FollowupTaskHandlerV2;
use crate::tools::handlers::multi_agents_v2::InterruptAgentHandler;
use crate::tools::handlers::multi_agents_v2::ListAgentsHandler as ListAgentsHandlerV2;
use crate::tools::handlers::multi_agents_v2::RestartAgentHandler as RestartAgentHandlerV2;
use crate::tools::handlers::multi_agents_v2::ResumeAgentHandler as ResumeAgentHandlerV2;
use crate::tools::handlers::multi_agents_v2::SendMessageHandler as SendMessageHandlerV2;
use crate::tools::handlers::multi_agents_v2::SpawnAgentHandler as SpawnAgentHandlerV2;
use crate::tools::handlers::multi_agents_v2::WaitAgentHandler as WaitAgentHandlerV2;
use crate::tools::handlers::view_image_spec::ViewImageToolOptions;
use crate::tools::hosted_spec::WebSearchToolOptions;
use crate::tools::hosted_spec::create_image_generation_tool;
use crate::tools::hosted_spec::create_web_search_tool;
use crate::tools::namespace_alias_policy::HostedNamespaceAliasPolicy;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::RegisteredTool;
use crate::tools::registry::ToolExposure;
use crate::tools::registry::ToolRegistry;
use crate::tools::registry::ToolRegistryBuilder;
use crate::tools::registry::override_tool_exposure;
use crate::tools::registry::override_tool_model_namespace;
use crate::tools::router::ToolRouter;
use crate::tools::router::ToolRouterParams;
use crate::tools::spec_plan_types::ToolRegistryBuildParams;
use codex_extension_api::ExtensionToolExecutor;
use codex_features::Feature;
use codex_login::AuthManager;
use codex_mcp::ToolInfo;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::dynamic_tools::DynamicToolNamespaceTool;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ToolMode;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_tool_execution_api::ToolExecutor;
use codex_tool_execution_api::ToolsConfig;
use codex_tool_registry_api::collect_request_plugin_install_entries;
use codex_tools::ResponsesApiNamespace;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::TOOL_SEARCH_TOOL_NAME;
use codex_tools::ToolEnvironmentMode;
use codex_tools::ToolName;
use codex_tools::ToolSearchInfo;
use codex_tools::ToolSpec;
use codex_tools::UnifiedExecShellMode;
use codex_tools::can_request_original_image_detail;
use codex_tools::collect_code_mode_exec_prompt_tool_definitions;
// fork-local: constructors for fork-only tool families grafted into add_fork_tools.
use codex_tools::create_cognos_ops_tools;
use codex_tools::create_context_ops_tools;
use codex_tools::create_desktop_automation_tools;
use codex_tools::create_first_moves_tools;
use codex_tools::create_repo_context_scout_tool;
use codex_tools::create_workflow_batch_tool;
use codex_tools::default_namespace_description;
use codex_tools::request_user_input_available_modes;
use codex_tools::shell_command_backend_for_features;
use codex_tools::shell_type_for_model_and_features;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::instrument;
use tracing::warn;

const MULTI_AGENT_V2_NAMESPACE_DESCRIPTION: &str = "Tools for spawning and managing sub-agents.";
const IMAGE_GEN_NAMESPACE: &str = "image_gen";
const IMAGEGEN_TOOL_NAME: &str = "imagegen";
const ACTOR_AUTHORIZATION_HEADER: &str = "x-openai-actor-authorization";

// fork-local: registry stores object-safe `Arc<dyn RegisteredTool>` (the fork's two-trait
// design), not upstream's `Arc<dyn CoreToolRuntime>`. Every concrete handler and override
// wrapper gains `RegisteredTool` via the blanket impl in `registry.rs`.
type PlannedRuntime = Arc<dyn RegisteredTool>;

#[derive(Default)]
struct PlannedTools {
    runtimes: Vec<PlannedRuntime>,
    hosted_specs: Vec<ToolSpec>,
}

impl PlannedTools {
    fn add<T>(&mut self, handler: T)
    where
        T: RegisteredTool + 'static,
    {
        self.runtimes.push(Arc::new(handler));
    }

    fn add_arc(&mut self, handler: PlannedRuntime) {
        self.runtimes.push(handler);
    }

    fn add_with_exposure<T>(&mut self, handler: T, exposure: ToolExposure)
    where
        T: RegisteredTool + 'static,
    {
        self.runtimes
            .push(override_tool_exposure(Arc::new(handler), exposure));
    }

    fn add_dispatch_only<T>(&mut self, handler: T)
    where
        T: RegisteredTool + 'static,
    {
        self.add_with_exposure(handler, ToolExposure::Hidden);
    }

    fn add_hosted_spec(&mut self, spec: ToolSpec) {
        self.hosted_specs.push(spec);
    }

    fn runtimes(&self) -> &[PlannedRuntime] {
        &self.runtimes
    }
}

#[derive(Clone, Copy)]
struct CoreToolPlanContext<'a> {
    step_context: &'a StepContext,
    mcp_tools: Option<&'a [ToolInfo]>,
    deferred_mcp_tools: Option<&'a [ToolInfo]>,
    tool_suggest_candidates: Option<&'a crate::tools::router::ToolSuggestCandidates>,
    extension_tool_executors: &'a [Arc<dyn ExtensionToolExecutor>],
    dynamic_tools: &'a [DynamicToolSpec],
    tool_search_handler_cache: &'a ToolSearchHandlerCache,
    default_agent_type_description: &'a str,
    wait_agent_timeouts: WaitAgentTimeoutOptions,
}

#[instrument(level = "trace", skip_all)]
pub(crate) fn build_tool_router(
    step_context: &StepContext,
    params: ToolRouterParams<'_>,
    tool_search_handler_cache: &ToolSearchHandlerCache,
) -> ToolRouter {
    let (model_visible_specs, registry) =
        build_tool_specs_and_registry(step_context, params, tool_search_handler_cache);
    ToolRouter::from_parts(registry, model_visible_specs)
}

#[instrument(level = "trace", skip_all)]
fn build_tool_specs_and_registry(
    step_context: &StepContext,
    params: ToolRouterParams<'_>,
    tool_search_handler_cache: &ToolSearchHandlerCache,
) -> (Vec<ToolSpec>, ToolRegistry) {
    let turn_context = step_context.turn.as_ref();
    let ToolRouterParams {
        mcp_tools,
        deferred_mcp_tools,
        tool_suggest_candidates,
        extension_tool_executors,
        dynamic_tools,
    } = params;
    let default_agent_type_description =
        crate::agent::role::spawn_tool_spec::build(&std::collections::BTreeMap::new());
    let context = CoreToolPlanContext {
        step_context,
        mcp_tools: mcp_tools.as_deref(),
        deferred_mcp_tools: deferred_mcp_tools.as_deref(),
        tool_suggest_candidates: tool_suggest_candidates.as_ref(),
        extension_tool_executors: &extension_tool_executors,
        dynamic_tools,
        tool_search_handler_cache,
        default_agent_type_description: &default_agent_type_description,
        wait_agent_timeouts: wait_agent_timeout_options(turn_context),
    };
    let mut planned_tools = PlannedTools::default();
    add_tool_sources(&context, &mut planned_tools);
    apply_direct_model_only_namespace_overrides(turn_context, &mut planned_tools);
    append_tool_search_executor(&context, &mut planned_tools);
    prepend_code_mode_executors(&context, &mut planned_tools);
    build_model_visible_specs_and_registry(turn_context, planned_tools)
}

fn apply_direct_model_only_namespace_overrides(
    turn_context: &TurnContext,
    planned_tools: &mut PlannedTools,
) {
    for runtime in &mut planned_tools.runtimes {
        let configured = runtime
            .tool_name()
            .namespace
            .as_ref()
            .is_some_and(|namespace| {
                turn_context
                    .config
                    .code_mode
                    .direct_only_tool_namespaces
                    .contains(namespace)
            });
        match runtime.exposure() {
            ToolExposure::Direct | ToolExposure::Deferred if configured => {
                *runtime =
                    override_tool_exposure(Arc::clone(runtime), ToolExposure::DirectModelOnly);
            }
            ToolExposure::Direct
            | ToolExposure::Deferred
            | ToolExposure::DirectModelOnly
            | ToolExposure::Hidden => {}
        }
    }
}

#[instrument(level = "trace", skip_all)]
fn build_model_visible_specs_and_registry(
    turn_context: &TurnContext,
    planned_tools: PlannedTools,
) -> (Vec<ToolSpec>, ToolRegistry) {
    let PlannedTools {
        runtimes,
        hosted_specs,
    } = planned_tools;
    let mut specs = Vec::new();
    let mut seen_tool_names = HashSet::new();
    for runtime in &runtimes {
        let tool_name = runtime.tool_name();
        if !seen_tool_names.insert(tool_name.clone()) {
            continue;
        }
        let exposure = runtime.exposure();
        if exposure.is_direct() && !is_hidden_by_code_mode_only(turn_context, &tool_name, exposure)
        {
            // fork-local: `RegisteredTool::spec()` is `Option<ToolSpec>`; a `None` spec marks a
            // dispatch-only tool with no model-visible spec, so skip emitting one for it.
            if let Some(spec) = runtime.spec() {
                specs.push(spec_for_model_request(
                    turn_context,
                    exposure,
                    &tool_name,
                    spec,
                ));
            }
        }
    }
    specs.extend(hosted_specs);

    let registry = ToolRegistry::from_tools(runtimes);
    let model_visible_specs = merge_into_namespaces(specs)
        .into_iter()
        .filter(|spec| {
            namespace_tools_enabled(turn_context) || !matches!(spec, ToolSpec::Namespace(_))
        })
        .collect();

    (model_visible_specs, registry)
}

fn spec_for_model_request(
    turn_context: &TurnContext,
    exposure: ToolExposure,
    tool_name: &ToolName,
    spec: ToolSpec,
) -> ToolSpec {
    let tool_mode = effective_tool_mode(turn_context);
    if matches!(tool_mode, ToolMode::CodeMode | ToolMode::CodeModeOnly)
        && exposure != ToolExposure::DirectModelOnly
        && !is_excluded_from_code_mode(turn_context, tool_name)
        && codex_code_mode::is_code_mode_nested_tool(spec.name())
    {
        codex_tools::augment_tool_spec_for_code_mode(spec)
    } else {
        spec
    }
}

#[instrument(level = "trace", skip_all)]
fn hosted_model_tool_specs_for_context(context: &CoreToolPlanContext<'_>) -> Vec<ToolSpec> {
    let turn_context = context.step_context.turn.as_ref();
    // Responses Lite accepts schemas for client-executed tools, not hosted Responses tools.
    if turn_context.model_info.use_responses_lite {
        return Vec::new();
    }

    let mut specs = Vec::new();
    let standalone_web_search_available = standalone_web_search_enabled(turn_context)
        && context
            .extension_tool_executors
            .iter()
            .any(|executor| executor.tool_name() == ToolName::namespaced("web", "run"));
    // `Some(Cached/Live/Disabled)` are the options for mode when standalone search is unavailable
    // and the provider supports hosted search. `None` prevents emitting a hosted search tool.
    let web_search_mode = (!standalone_web_search_available
        && turn_context.provider.capabilities().web_search)
        .then_some(turn_context.config.web_search_mode.value());
    let web_search_config = web_search_mode
        .as_ref()
        .and(turn_context.config.web_search_config.as_ref());
    if let Some(hosted_web_search_tool) = create_web_search_tool(WebSearchToolOptions {
        web_search_mode,
        web_search_config,
        web_search_tool_type: turn_context.model_info.web_search_tool_type,
    }) {
        specs.push(hosted_web_search_tool);
    }
    // TODO: Remove hosted image generation once the standalone extension is ready.
    if image_generation_tool_enabled(turn_context)
        && !standalone_image_generation_available(turn_context, context.extension_tool_executors)
    {
        specs.push(create_image_generation_tool("png"));
    }
    specs
}

// fork-local: legacy `ToolsConfig`-keyed tool-planning pipeline retained for the
// fork's test/spec consumers (`tools/spec.rs`, `test_support.rs`,
// `spec_plan_tests.rs`, `spec_tests.rs`). Upstream replaced this with the
// `CoreToolPlanContext`/`PlannedTools` pipeline above; these functions stay so the
// fork's direct callers keep compiling. Restored verbatim from fork b342f16013
// except: `hosted_model_tool_specs` here is the `&ToolsConfig` variant (the
// `&CoreToolPlanContext` one is `hosted_model_tool_specs_for_context`); the three
// private helpers below carry a `_from_config`/`_with_exposure` suffix to avoid
// colliding with the upstream `TurnContext`/`PlannedTools` helpers of the same name;
// and `RequestPluginInstallHandler::new`'s second argument adopts the upstream
// `ToolSuggestPresentation` type.
pub(crate) fn hosted_model_tool_specs(config: &ToolsConfig) -> Vec<ToolSpec> {
    let mut specs = Vec::new();
    if let Some(web_search_tool) = create_web_search_tool(WebSearchToolOptions {
        web_search_mode: config.web_search_mode,
        web_search_config: config.web_search_config.as_ref(),
        web_search_tool_type: config.web_search_tool_type,
    }) {
        specs.push(web_search_tool);
    }
    if config.image_gen_tool {
        specs.push(create_image_generation_tool("png"));
    }
    specs
}

pub(crate) fn build_tool_registry_builder_from_executors(
    config: &ToolsConfig,
    executors: Vec<Arc<dyn RegisteredTool>>,
    hosted_specs: Vec<ToolSpec>,
) -> ToolRegistryBuilder {
    let executors = alias_hosted_reserved_namespace_executors(executors, &hosted_specs);
    let mut builder = ToolRegistryBuilder::new();
    let deferred_tools_available = executors
        .iter()
        .any(|executor| executor.exposure() == ToolExposure::Deferred);

    for executor in build_code_mode_executors_from_config(
        config,
        &executors,
        config.search_tool && deferred_tools_available,
    ) {
        builder.register_tool(executor);
    }

    let mut non_deferred_specs = Vec::new();
    let mut deferred_search_infos = Vec::new();
    for executor in &executors {
        match executor.exposure() {
            ToolExposure::Direct | ToolExposure::DirectModelOnly => {
                if let Some(spec) = executor.spec() {
                    non_deferred_specs.push((spec, executor.exposure()));
                }
            }
            ToolExposure::Deferred => {
                if let Some(search_info) = executor.search_info() {
                    deferred_search_infos.push(search_info);
                }
            }
            // Hidden tools stay registered for dispatch but are neither exposed
            // to the model nor surfaced for deferred discovery here.
            ToolExposure::Hidden => {}
        }
    }

    non_deferred_specs.extend(
        hosted_specs
            .into_iter()
            .map(|spec| (spec, ToolExposure::Direct)),
    );

    let non_deferred_specs = non_deferred_specs
        .into_iter()
        .map(|(spec, exposure)| {
            if config.code_mode_enabled && exposure != ToolExposure::DirectModelOnly {
                codex_tools::augment_tool_spec_for_code_mode(spec)
            } else {
                spec
            }
        })
        .collect();

    for spec in merge_into_namespaces(non_deferred_specs) {
        if !config.namespace_tools && matches!(spec, ToolSpec::Namespace(_)) {
            continue;
        }
        builder.push_spec(spec);
    }

    for executor in executors {
        builder.register_tool_without_spec(executor);
    }

    if config.search_tool && config.namespace_tools && !deferred_search_infos.is_empty() {
        builder.register_tool(Arc::new(ToolSearchHandler::new(deferred_search_infos)));
    }

    builder
}

pub(crate) fn alias_hosted_reserved_namespace_executors(
    executors: Vec<Arc<dyn RegisteredTool>>,
    hosted_specs: &[ToolSpec],
) -> Vec<Arc<dyn RegisteredTool>> {
    let occupied_model_tool_names = executors.iter().map(|executor| {
        let tool_name = executor.tool_name();
        tool_name.namespace.unwrap_or(tool_name.name)
    });
    let mut alias_policy =
        HostedNamespaceAliasPolicy::for_hosted_specs(hosted_specs, occupied_model_tool_names);
    if !alias_policy.has_reserved_namespaces() {
        return executors;
    }

    executors
        .into_iter()
        .map(|executor| {
            let tool_name = executor.tool_name();
            let Some(namespace) = tool_name.namespace.as_deref() else {
                return executor;
            };
            let Some(model_namespace) = alias_policy.alias_for_source_namespace(namespace) else {
                return executor;
            };
            warn!(
                "Aliasing tool namespace `{namespace}` to `{model_namespace}` because it collides with a hosted Responses API namespace"
            );
            override_tool_model_namespace(executor, model_namespace)
        })
        .collect()
}

fn build_code_mode_executors_from_config(
    config: &ToolsConfig,
    executors: &[Arc<dyn RegisteredTool>],
    deferred_tools_available: bool,
) -> Vec<Arc<dyn RegisteredTool>> {
    if !config.code_mode_enabled {
        return vec![];
    }

    let code_mode_nested_tool_specs = executors
        .iter()
        .filter_map(|executor| {
            if executor.exposure() == ToolExposure::DirectModelOnly {
                return None;
            }

            executor.spec()
        })
        .collect::<Vec<_>>();
    let namespace_descriptions = code_mode_namespace_descriptions(&code_mode_nested_tool_specs);
    let mut enabled_tools =
        collect_code_mode_exec_prompt_tool_definitions(code_mode_nested_tool_specs.iter());
    enabled_tools
        .sort_by(|left, right| compare_code_mode_tools(left, right, &namespace_descriptions));

    vec![
        Arc::new(CodeModeExecuteHandler::new(
            create_code_mode_tool(
                &enabled_tools,
                &namespace_descriptions,
                config.code_mode_only_enabled,
                deferred_tools_available,
            ),
            code_mode_nested_tool_specs,
        )),
        Arc::new(CodeModeWaitHandler),
    ]
}

pub(crate) fn collect_tool_executors(
    config: &ToolsConfig,
    params: ToolRegistryBuildParams<'_>,
) -> Vec<Arc<dyn RegisteredTool>> {
    let exec_permission_approvals_enabled = config.exec_permission_approvals_enabled;
    let mut executors = Vec::<Arc<dyn RegisteredTool>>::new();

    if config.environment_mode.has_environment() {
        let include_environment_id =
            matches!(config.environment_mode, ToolEnvironmentMode::Multiple);
        match &config.shell_type {
            ConfigShellToolType::UnifiedExec => {
                executors.push(Arc::new(ExecCommandHandler::new(
                    ExecCommandHandlerOptions {
                        allow_login_shell: config.allow_login_shell,
                        exec_permission_approvals_enabled,
                        include_environment_id,
                        include_shell_parameter: true,
                    },
                )));
                executors.push(Arc::new(WriteStdinHandler));
            }
            ConfigShellToolType::Disabled => {}
            ConfigShellToolType::Default
            | ConfigShellToolType::Local
            | ConfigShellToolType::ShellCommand => {
                executors.push(Arc::new(ShellCommandHandler::new(
                    ShellCommandHandlerOptions {
                        backend_config: config.shell_command_backend,
                        allow_login_shell: config.allow_login_shell,
                        exec_permission_approvals_enabled,
                    },
                )));
            }
        }
    }

    if config.environment_mode.has_environment()
        && config.shell_type != ConfigShellToolType::Disabled
    {
        match &config.shell_type {
            ConfigShellToolType::UnifiedExec => {
                executors.push(override_tool_exposure(
                    Arc::new(ShellCommandHandler::from(config.shell_command_backend)),
                    ToolExposure::DirectModelOnly,
                ));
            }
            ConfigShellToolType::Default
            | ConfigShellToolType::Local
            | ConfigShellToolType::ShellCommand
            | ConfigShellToolType::Disabled => {}
        }
    }

    if params.mcp_tools.is_some() {
        executors.push(Arc::new(ListMcpResourcesHandler));
        executors.push(Arc::new(ListMcpResourceTemplatesHandler));
        executors.push(Arc::new(ReadMcpResourceHandler));
    }

    executors.push(Arc::new(PlanHandler));
    if config.goal_tools {
        executors.push(Arc::new(GetGoalHandler));
        executors.push(Arc::new(CreateGoalHandler));
        executors.push(Arc::new(UpdateGoalHandler));
    }

    executors.push(Arc::new(RequestUserInputHandler {
        available_modes: config.request_user_input_available_modes.clone(),
    }));

    if config.request_permissions_tool_enabled {
        executors.push(Arc::new(RequestPermissionsHandler));
    }

    if config.tool_suggest
        && let Some(discoverable_tools) =
            params.discoverable_tools.filter(|tools| !tools.is_empty())
    {
        // Upstream changed `RequestPluginInstallHandler::new`'s second argument from
        // the fork's `tool_search_available: bool` to a `ToolSuggestPresentation`.
        // The list-tool presentation matches the description asserted by the fork's
        // `request_plugin_install_description_lists_discoverable_tools` test.
        executors.push(Arc::new(RequestPluginInstallHandler::new(
            discoverable_tools.to_vec(),
            crate::tools::router::ToolSuggestPresentation::ListTool,
        )));
    }

    if config.environment_mode.has_environment() && config.apply_patch_tool_type.is_some() {
        let include_environment_id =
            matches!(config.environment_mode, ToolEnvironmentMode::Multiple);
        executors.push(Arc::new(ApplyPatchHandler::new(include_environment_id)));
    }

    if config
        .experimental_supported_tools
        .iter()
        .any(|tool| tool == "test_sync_tool")
    {
        executors.push(Arc::new(TestSyncHandler));
    }

    if config.context_ops_enabled {
        for tool in create_cognos_ops_tools() {
            executors.push(Arc::new(CognosOpsHandler::new(tool)));
        }
        for tool in create_context_ops_tools() {
            executors.push(Arc::new(ContextOpsHandler::new(tool)));
        }
    }
    if config.workflow_batch_enabled && config.environment_mode.has_environment() {
        executors.push(Arc::new(ContextOpsHandler::new(
            create_workflow_batch_tool(),
        )));
    }
    if config.desktop_automation_enabled {
        for tool in create_desktop_automation_tools(config.desktop_automation_allow_input) {
            executors.push(Arc::new(DesktopAutomationHandler::new(tool)));
        }
    }

    if config.environment_mode.has_environment() {
        let include_environment_id =
            matches!(config.environment_mode, ToolEnvironmentMode::Multiple);
        executors.push(Arc::new(ViewImageHandler::new(ViewImageToolOptions {
            can_request_original_image_detail: config.can_request_original_image_detail,
            include_environment_id,
        })));
    }

    if config.collab_tools {
        if config.multi_agent_v2 {
            let exposure = if config.multi_agent_v2_non_code_mode_only {
                ToolExposure::DirectModelOnly
            } else {
                ToolExposure::Direct
            };
            let agent_type_description = crate::tools::spec_plan_types::agent_type_description(
                config,
                params.default_agent_type_description,
            );
            executors.push(multi_agent_v2_handler_with_exposure(
                SpawnAgentHandlerV2::new(SpawnAgentToolOptions {
                    available_models: config.available_models.clone(),
                    agent_type_description,
                    hide_agent_type_model_reasoning: config.hide_spawn_agent_metadata,
                    include_usage_hint: config.spawn_agent_usage_hint,
                    usage_hint_text: config.spawn_agent_usage_hint_text.clone(),
                    max_concurrent_threads_per_session: config.max_concurrent_threads_per_session,
                }),
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                SendMessageHandlerV2,
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                FollowupTaskHandlerV2,
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                WaitAgentHandlerV2::new(params.wait_agent_timeouts),
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                ResumeAgentHandlerV2,
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                CompactAgentHandlerV2,
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                RestartAgentHandlerV2,
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                CloseAgentHandlerV2,
                exposure,
            ));
            executors.push(multi_agent_v2_handler_with_exposure(
                ListAgentsHandlerV2,
                exposure,
            ));
        } else {
            let agent_type_description = crate::tools::spec_plan_types::agent_type_description(
                config,
                params.default_agent_type_description,
            );
            executors.push(Arc::new(SpawnAgentHandler::new(SpawnAgentToolOptions {
                available_models: config.available_models.clone(),
                agent_type_description,
                hide_agent_type_model_reasoning: config.hide_spawn_agent_metadata,
                include_usage_hint: config.spawn_agent_usage_hint,
                usage_hint_text: config.spawn_agent_usage_hint_text.clone(),
                max_concurrent_threads_per_session: config.max_concurrent_threads_per_session,
            })));
            executors.push(Arc::new(SendInputHandler));
            executors.push(Arc::new(ResumeAgentHandler));
            executors.push(Arc::new(WaitAgentHandler::new(params.wait_agent_timeouts)));
            executors.push(Arc::new(CloseAgentHandler));
        }
    }

    if config.agent_jobs_tools {
        executors.push(Arc::new(SpawnAgentsOnCsvHandler));
        if config.agent_jobs_worker_tools {
            executors.push(Arc::new(ReportAgentJobResultHandler));
        }
    }

    if config.first_moves_enabled && config.environment_mode.has_environment() {
        for tool in create_first_moves_tools() {
            executors.push(Arc::new(FirstMovesHandler::new(tool)));
        }
    }

    if config.repo_context_scout_tool_enabled && config.environment_mode.has_environment() {
        executors.push(Arc::new(RepoContextScoutHandler::new(
            create_repo_context_scout_tool(),
        )));
    }

    if let Some(mcp_tools) = params.mcp_tools {
        for tool in mcp_tools {
            match McpHandler::new(tool.clone()) {
                Ok(handler) => executors.push(Arc::new(handler)),
                Err(err) => warn!(
                    "Skipping MCP tool `{}`: failed to build tool spec: {err}",
                    tool.canonical_tool_name()
                ),
            }
        }
    }

    if let Some(deferred_mcp_tools) = params.deferred_mcp_tools {
        for tool in deferred_mcp_tools {
            match McpHandler::new(tool.clone()) {
                Ok(handler) => executors.push(override_tool_exposure(
                    Arc::new(handler),
                    ToolExposure::Deferred,
                )),
                Err(err) => warn!(
                    "Skipping deferred MCP tool `{}`: failed to build tool spec: {err}",
                    tool.canonical_tool_name()
                ),
            }
        }
    }

    for spec in params.dynamic_tools {
        match spec {
            DynamicToolSpec::Function(tool) => {
                let Some(handler) = DynamicToolHandler::new(tool).map(Arc::new) else {
                    tracing::error!(
                        "Failed to convert dynamic tool {:?} to OpenAI tool",
                        tool.name
                    );
                    continue;
                };

                executors.push(handler);
            }
            DynamicToolSpec::Namespace(namespace) => {
                for tool in &namespace.tools {
                    let DynamicToolNamespaceTool::Function(tool) = tool;
                    let Some(handler) =
                        DynamicToolHandler::new_in_namespace(namespace, tool).map(Arc::new)
                    else {
                        tracing::error!(
                            "Failed to convert dynamic tool {:?}.{:?} to OpenAI tool",
                            namespace.name,
                            tool.name
                        );
                        continue;
                    };

                    executors.push(handler);
                }
            }
        }
    }

    append_extension_tool_executors_from_config(
        config,
        params.extension_tool_executors,
        &mut executors,
    );

    executors
}

fn append_extension_tool_executors_from_config(
    config: &ToolsConfig,
    executors: &[Arc<dyn ExtensionToolExecutor>],
    registered_executors: &mut Vec<Arc<dyn RegisteredTool>>,
) {
    if executors.is_empty() {
        return;
    }

    let mut reserved_tool_names = registered_executors
        .iter()
        .map(|executor| executor.tool_name())
        .collect::<HashSet<_>>();
    if config.code_mode_enabled {
        reserved_tool_names.insert(ToolName::plain(codex_code_mode::PUBLIC_TOOL_NAME));
        reserved_tool_names.insert(ToolName::plain(codex_code_mode::WAIT_TOOL_NAME));
    }
    if config.search_tool
        && config.namespace_tools
        && registered_executors
            .iter()
            .any(|executor| executor.exposure() == ToolExposure::Deferred)
    {
        reserved_tool_names.insert(ToolName::plain(TOOL_SEARCH_TOOL_NAME));
    }

    for executor in executors.iter().cloned() {
        let tool_name = executor.tool_name();
        if !reserved_tool_names.insert(tool_name.clone()) {
            warn!("Skipping extension tool `{tool_name}`: handler already registered");
            continue;
        }
        registered_executors.push(Arc::new(ExtensionToolAdapter::from_extension_executor(
            executor,
        )));
    }
}

fn multi_agent_v2_handler_with_exposure(
    handler: impl RegisteredTool + 'static,
    exposure: ToolExposure,
) -> Arc<dyn RegisteredTool> {
    override_tool_exposure(Arc::new(handler), exposure)
}

pub(crate) fn search_tool_enabled(turn_context: &TurnContext) -> bool {
    turn_context.model_info.supports_search_tool && namespace_tools_enabled(turn_context)
}

pub(crate) fn tool_suggest_enabled(turn_context: &TurnContext) -> bool {
    let features = turn_context.config.features.get();
    features.enabled(Feature::ToolSuggest)
        && features.enabled(Feature::Apps)
        && features.enabled(Feature::Plugins)
}

fn namespace_tools_enabled(turn_context: &TurnContext) -> bool {
    turn_context.provider.capabilities().namespace_tools
}

fn multi_agent_v2_enabled(turn_context: &TurnContext) -> bool {
    turn_context.multi_agent_version == MultiAgentVersion::V2
}

fn collab_tools_enabled(turn_context: &TurnContext) -> bool {
    match turn_context.multi_agent_version {
        MultiAgentVersion::Disabled => false,
        MultiAgentVersion::V1 => !exceeds_thread_spawn_depth_limit(
            next_thread_spawn_depth(&turn_context.session_source),
            turn_context.config.agent_max_depth,
        ),
        MultiAgentVersion::V2 => true,
    }
}

fn agent_jobs_tools_enabled(turn_context: &TurnContext) -> bool {
    turn_context
        .config
        .features
        .get()
        .enabled(Feature::SpawnCsv)
        && collab_tools_enabled(turn_context)
}

fn agent_jobs_worker_tools_enabled(turn_context: &TurnContext) -> bool {
    agent_jobs_tools_enabled(turn_context)
        && matches!(
            &turn_context.session_source,
            SessionSource::SubAgent(SubAgentSource::Other(label))
                if label.starts_with("agent_job:")
        )
}

fn image_generation_tool_enabled(turn_context: &TurnContext) -> bool {
    image_generation_runtime_enabled(turn_context)
        && turn_context
            .config
            .features
            .get()
            .enabled(Feature::ImageGeneration)
}

fn image_generation_runtime_enabled(turn_context: &TurnContext) -> bool {
    (provider_uses_actor_authorization(turn_context)
        || (turn_context.provider.info().requires_openai_auth
            && turn_context
                .auth_manager
                .as_deref()
                .is_some_and(AuthManager::current_auth_uses_codex_backend)))
        && turn_context.provider.capabilities().image_generation
        && turn_context
            .model_info
            .input_modalities
            .contains(&InputModality::Image)
}

fn provider_uses_actor_authorization(turn_context: &TurnContext) -> bool {
    let provider_info = turn_context.provider.info();
    !provider_info.requires_openai_auth
        && provider_info.http_headers.as_ref().is_some_and(|headers| {
            headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case(ACTOR_AUTHORIZATION_HEADER) && !value.trim().is_empty()
            })
        })
}

fn standalone_image_generation_model_visible(turn_context: &TurnContext) -> bool {
    if !image_generation_runtime_enabled(turn_context) || !namespace_tools_enabled(turn_context) {
        return false;
    }

    if turn_context.model_info.use_responses_lite {
        return true;
    }

    turn_context
        .config
        .features
        .get()
        .enabled(Feature::ImageGenExt)
}

fn standalone_image_generation_available(
    turn_context: &TurnContext,
    extension_tools: &[Arc<dyn ExtensionToolExecutor>],
) -> bool {
    standalone_image_generation_model_visible(turn_context)
        && extension_tools.iter().any(|executor| {
            executor.tool_name() == ToolName::namespaced(IMAGE_GEN_NAMESPACE, IMAGEGEN_TOOL_NAME)
        })
}

fn wait_agent_timeout_options(turn_context: &TurnContext) -> WaitAgentTimeoutOptions {
    WaitAgentTimeoutOptions::new(turn_context)
}

fn agent_type_description(
    turn_context: &TurnContext,
    default_agent_type_description: &str,
) -> String {
    let agent_type_description =
        crate::agent::role::spawn_tool_spec::build(&turn_context.config.agent_roles);
    if agent_type_description.is_empty() {
        default_agent_type_description.to_string()
    } else {
        agent_type_description
    }
}

fn is_hidden_by_code_mode_only(
    turn_context: &TurnContext,
    tool_name: &ToolName,
    exposure: ToolExposure,
) -> bool {
    let tool_mode = effective_tool_mode(turn_context);
    tool_mode == ToolMode::CodeModeOnly
        && exposure != ToolExposure::DirectModelOnly
        && codex_code_mode::is_code_mode_nested_tool(&codex_tools::code_mode_name_for_tool_name(
            tool_name,
        ))
}

fn is_excluded_from_code_mode(turn_context: &TurnContext, tool_name: &ToolName) -> bool {
    tool_name.namespace.as_ref().is_some_and(|namespace| {
        turn_context
            .config
            .code_mode
            .excluded_tool_namespaces
            .contains(namespace)
    })
}

fn build_code_mode_executors(
    turn_context: &TurnContext,
    // fork-local: registry stores `Arc<dyn RegisteredTool>`, not `Arc<dyn CoreToolRuntime>`.
    executors: &[Arc<dyn RegisteredTool>],
) -> Vec<Arc<dyn RegisteredTool>> {
    let tool_mode = effective_tool_mode(turn_context);
    if !matches!(tool_mode, ToolMode::CodeMode | ToolMode::CodeModeOnly) {
        return vec![];
    }

    let mut code_mode_nested_tool_specs = Vec::new();
    let mut exec_prompt_tool_specs = Vec::new();
    let mut deferred_tools_available = false;
    let deferred_tools_guidance_enabled = search_tool_enabled(turn_context);
    for executor in executors {
        let exposure = executor.exposure();
        if exposure == ToolExposure::DirectModelOnly {
            continue;
        }

        if exposure == ToolExposure::Hidden {
            continue;
        }

        if is_excluded_from_code_mode(turn_context, &executor.tool_name()) {
            continue;
        }

        // fork-local: `spec()` is `Option<ToolSpec>`; dispatch-only tools (no spec) cannot
        // contribute to code-mode prompt/nested specs, so skip them.
        let Some(spec) = executor.spec() else {
            continue;
        };

        if exposure == ToolExposure::Deferred {
            // Only show deferred-tool guidance when supported and an included spec is usable by code mode.
            deferred_tools_available |= deferred_tools_guidance_enabled
                && !collect_code_mode_exec_prompt_tool_definitions(std::iter::once(&spec))
                    .is_empty();
        } else {
            exec_prompt_tool_specs.push(spec.clone());
        }
        code_mode_nested_tool_specs.push(spec);
    }

    let namespace_descriptions = code_mode_namespace_descriptions(&exec_prompt_tool_specs);
    let mut enabled_tools =
        collect_code_mode_exec_prompt_tool_definitions(exec_prompt_tool_specs.iter());
    enabled_tools
        .sort_by(|left, right| compare_code_mode_tools(left, right, &namespace_descriptions));

    vec![
        Arc::new(CodeModeExecuteHandler::new(
            create_code_mode_tool(
                &enabled_tools,
                &namespace_descriptions,
                tool_mode == ToolMode::CodeModeOnly,
                deferred_tools_available,
            ),
            code_mode_nested_tool_specs,
        )),
        Arc::new(CodeModeWaitHandler),
    ]
}

#[instrument(level = "trace", skip_all, fields(tool_spec_count = specs.len()))]
fn merge_into_namespaces(specs: Vec<ToolSpec>) -> Vec<ToolSpec> {
    let mut merged_specs = Vec::with_capacity(specs.len());
    let mut namespace_indices = BTreeMap::<String, usize>::new();
    for spec in specs {
        match spec {
            ToolSpec::Namespace(mut namespace) => {
                if let Some(index) = namespace_indices.get(&namespace.name).copied() {
                    let ToolSpec::Namespace(existing_namespace) = &mut merged_specs[index] else {
                        unreachable!("namespace index must point to a namespace spec");
                    };
                    if existing_namespace.description.trim().is_empty()
                        && !namespace.description.trim().is_empty()
                    {
                        existing_namespace.description = namespace.description;
                    }
                    existing_namespace.tools.append(&mut namespace.tools);
                    continue;
                }

                namespace_indices.insert(namespace.name.clone(), merged_specs.len());
                merged_specs.push(ToolSpec::Namespace(namespace));
            }
            spec => merged_specs.push(spec),
        }
    }

    for spec in &mut merged_specs {
        let ToolSpec::Namespace(namespace) = spec else {
            continue;
        };

        namespace.tools.sort_by(|left, right| match (left, right) {
            (
                ResponsesApiNamespaceTool::Function(left),
                ResponsesApiNamespaceTool::Function(right),
            ) => left.name.cmp(&right.name),
        });

        if namespace.description.trim().is_empty() {
            namespace.description = default_namespace_description(&namespace.name);
        }
    }

    merged_specs
}

fn code_mode_namespace_descriptions(
    specs: &[ToolSpec],
) -> BTreeMap<String, codex_code_mode::ToolNamespaceDescription> {
    let mut namespace_descriptions = BTreeMap::new();
    for spec in specs {
        let ToolSpec::Namespace(namespace) = spec else {
            continue;
        };

        let entry = namespace_descriptions
            .entry(namespace.name.clone())
            .or_insert_with(|| codex_code_mode::ToolNamespaceDescription {
                name: namespace.name.clone(),
                description: namespace.description.clone(),
            });
        if entry.description.trim().is_empty() && !namespace.description.trim().is_empty() {
            entry.description = namespace.description.clone();
        }
    }
    namespace_descriptions
}

#[instrument(level = "trace", skip_all)]
fn add_tool_sources(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    add_shell_tools(context, planned_tools);
    add_mcp_resource_tools(context, planned_tools);
    add_core_utility_tools(context, planned_tools);
    // fork-local: register fork-only tool families (goals, context/cognos ops,
    // desktop automation, first-moves, repo-context-scout) gated by the fork's
    // ToolsConfig flags. Upstream's add_tool_sources pipeline does not know these
    // families, so they are grafted here to survive future merges.
    add_fork_tools(context, planned_tools);
    add_collaboration_tools(context, planned_tools);
    add_mcp_runtime_tools(context, planned_tools);
    add_extension_tools(context, planned_tools);
    add_dynamic_tools(context, planned_tools);
    for spec in hosted_model_tool_specs_for_context(context) {
        planned_tools.add_hosted_spec(spec);
    }
}

// fork-local: fork-only tool families folded into upstream's PlannedTools
// pipeline. Each family reuses the exact gate the fork applied in its previous
// `collect_tool_executors` implementation, reading from `turn_context.tools_config`.
#[instrument(level = "trace", skip_all)]
fn add_fork_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    let turn_context = context.step_context.turn.as_ref();
    let tools_config = &turn_context.tools_config;
    let environment_mode = turn_context.tools_config.environment_mode;

    if tools_config.goal_tools {
        planned_tools.add(GetGoalHandler);
        planned_tools.add(CreateGoalHandler);
        planned_tools.add(UpdateGoalHandler);
    }

    if tools_config.context_ops_enabled {
        for tool in create_cognos_ops_tools() {
            planned_tools.add(CognosOpsHandler::new(tool));
        }
        for tool in create_context_ops_tools() {
            planned_tools.add(ContextOpsHandler::new(tool));
        }
    }

    if tools_config.workflow_batch_enabled && environment_mode.has_environment() {
        planned_tools.add(ContextOpsHandler::new(create_workflow_batch_tool()));
    }

    if tools_config.desktop_automation_enabled {
        for tool in create_desktop_automation_tools(tools_config.desktop_automation_allow_input) {
            planned_tools.add(DesktopAutomationHandler::new(tool));
        }
    }

    if tools_config.first_moves_enabled && environment_mode.has_environment() {
        for tool in create_first_moves_tools() {
            planned_tools.add(FirstMovesHandler::new(tool));
        }
    }

    if tools_config.repo_context_scout_tool_enabled && environment_mode.has_environment() {
        planned_tools.add(RepoContextScoutHandler::new(
            create_repo_context_scout_tool(),
        ));
    }
}

fn standalone_web_search_enabled(turn_context: &TurnContext) -> bool {
    namespace_tools_enabled(turn_context)
        && (turn_context.model_info.use_responses_lite
            || turn_context
                .config
                .features
                .get()
                .enabled(Feature::StandaloneWebSearch))
}

fn tool_environment_mode(step_context: &StepContext) -> ToolEnvironmentMode {
    ToolEnvironmentMode::from_count(step_context.environments.turn_environments.len())
}

#[instrument(level = "trace", skip_all)]
fn add_shell_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    let turn_context = context.step_context.turn.as_ref();
    let features = turn_context.config.features.get();
    let environment_mode = tool_environment_mode(context.step_context);
    if !environment_mode.has_environment() {
        return;
    }

    let allow_login_shell = turn_context.config.permissions.allow_login_shell;
    let exec_permission_approvals_enabled = features.enabled(Feature::ExecPermissionApprovals);
    let include_environment_id = matches!(environment_mode, ToolEnvironmentMode::Multiple);
    let shell_command_options = ShellCommandHandlerOptions {
        backend_config: shell_command_backend_for_features(features),
        allow_login_shell,
        exec_permission_approvals_enabled,
    };

    match shell_type_for_model_and_features(&turn_context.model_info, features) {
        ConfigShellToolType::UnifiedExec => {
            planned_tools.add(ExecCommandHandler::new(ExecCommandHandlerOptions {
                allow_login_shell,
                exec_permission_approvals_enabled,
                include_environment_id,
                include_shell_parameter: unified_exec_should_include_shell_parameter(
                    turn_context,
                    context.step_context,
                ),
            }));
            planned_tools.add(WriteStdinHandler);

            // Keep the legacy shell tool registered while unified exec is
            // model-visible.
            planned_tools.add_dispatch_only(ShellCommandHandler::new(shell_command_options));
        }
        ConfigShellToolType::Disabled => {}
        ConfigShellToolType::Default
        | ConfigShellToolType::Local
        | ConfigShellToolType::ShellCommand => {
            planned_tools.add(ShellCommandHandler::new(shell_command_options));
        }
    }
}

fn unified_exec_should_include_shell_parameter(
    turn_context: &TurnContext,
    step_context: &StepContext,
) -> bool {
    !matches!(
        &turn_context.unified_exec_shell_mode,
        UnifiedExecShellMode::ZshFork(_)
    ) || step_context
        .environments
        .turn_environments
        .iter()
        .any(|environment| environment.environment.is_remote())
}

#[instrument(level = "trace", skip_all)]
fn add_mcp_resource_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    if context.mcp_tools.is_some() {
        planned_tools.add(ListMcpResourcesHandler);
        planned_tools.add(ListMcpResourceTemplatesHandler);
        planned_tools.add(ReadMcpResourceHandler);
    }
}

#[instrument(level = "trace", skip_all)]
fn add_core_utility_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    let turn_context = context.step_context.turn.as_ref();
    let features = turn_context.config.features.get();
    let environment_mode = tool_environment_mode(context.step_context);

    planned_tools.add(PlanHandler);

    if features.enabled(Feature::DeferredExecutor) {
        planned_tools.add(WaitForEnvironmentHandler);
    }

    if turn_context.config.experimental_request_user_input_enabled {
        planned_tools.add_with_exposure(
            RequestUserInputHandler {
                available_modes: request_user_input_available_modes(features),
            },
            ToolExposure::DirectModelOnly,
        );
    }

    if environment_mode.has_environment() && features.enabled(Feature::RequestPermissionsTool) {
        planned_tools.add(RequestPermissionsHandler);
    }

    if features.enabled(Feature::TokenBudget) {
        planned_tools.add_with_exposure(NewContextWindowHandler, ToolExposure::DirectModelOnly);
        planned_tools.add(GetContextRemainingHandler);
    }

    if features.enabled(Feature::CurrentTimeReminder) {
        planned_tools.add(CurrentTimeHandler);
    }

    if features.enabled(Feature::SleepTool) {
        planned_tools.add(SleepHandler);
    }

    if tool_suggest_enabled(turn_context)
        && let Some(candidates) = context
            .tool_suggest_candidates
            .filter(|candidates| !candidates.tools.is_empty())
    {
        if candidates.presentation == crate::tools::router::ToolSuggestPresentation::ListTool {
            planned_tools.add(ListAvailablePluginsToInstallHandler::new(
                collect_request_plugin_install_entries(&candidates.tools),
            ));
        }
        planned_tools.add(RequestPluginInstallHandler::new(
            candidates.tools.clone(),
            candidates.presentation,
        ));
    }

    if environment_mode.has_environment() && turn_context.model_info.apply_patch_tool_type.is_some()
    {
        let include_environment_id = matches!(environment_mode, ToolEnvironmentMode::Multiple);
        planned_tools.add(ApplyPatchHandler::new(include_environment_id));
    }

    if turn_context
        .model_info
        .experimental_supported_tools
        .iter()
        .any(|tool| tool == "test_sync_tool")
    {
        planned_tools.add(TestSyncHandler);
    }

    if environment_mode.has_environment() {
        let include_environment_id = matches!(environment_mode, ToolEnvironmentMode::Multiple);
        planned_tools.add(ViewImageHandler::new(ViewImageToolOptions {
            can_request_original_image_detail: can_request_original_image_detail(
                &turn_context.model_info,
            ),
            include_environment_id,
        }));
    }
}

#[instrument(level = "trace", skip_all)]
fn add_collaboration_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    let turn_context = context.step_context.turn.as_ref();
    if collab_tools_enabled(turn_context) {
        if multi_agent_v2_enabled(turn_context) {
            let exposure = if turn_context.config.multi_agent_v2.non_code_mode_only {
                ToolExposure::DirectModelOnly
            } else {
                ToolExposure::Direct
            };
            let tool_namespace = namespace_tools_enabled(turn_context)
                .then_some(turn_context.config.multi_agent_v2.tool_namespace.as_deref())
                .flatten();
            let agent_type_description =
                agent_type_description(turn_context, context.default_agent_type_description);
            planned_tools.add_arc(override_tool_exposure(
                multi_agent_v2_handler(
                    SpawnAgentHandlerV2::new(SpawnAgentToolOptions {
                        available_models: turn_context.available_models.clone(),
                        agent_type_description,
                        hide_agent_type_model_reasoning: turn_context
                            .config
                            .multi_agent_v2
                            .hide_spawn_agent_metadata,
                        include_usage_hint: turn_context.config.multi_agent_v2.usage_hint_enabled,
                        usage_hint_text: turn_context.config.multi_agent_v2.usage_hint_text.clone(),
                        max_concurrent_threads_per_session: Some(
                            turn_context
                                .config
                                .multi_agent_v2
                                .max_concurrent_threads_per_session,
                        ),
                    }),
                    tool_namespace,
                ),
                exposure,
            ));
            planned_tools.add_arc(override_tool_exposure(
                multi_agent_v2_handler(SendMessageHandlerV2, tool_namespace),
                exposure,
            ));
            planned_tools.add_arc(override_tool_exposure(
                multi_agent_v2_handler(FollowupTaskHandlerV2, tool_namespace),
                exposure,
            ));
            planned_tools.add_arc(override_tool_exposure(
                multi_agent_v2_handler(
                    WaitAgentHandlerV2::new(context.wait_agent_timeouts),
                    tool_namespace,
                ),
                exposure,
            ));
            planned_tools.add_arc(override_tool_exposure(
                multi_agent_v2_handler(InterruptAgentHandler, tool_namespace),
                exposure,
            ));
            planned_tools.add_arc(override_tool_exposure(
                multi_agent_v2_handler(ListAgentsHandlerV2, tool_namespace),
                exposure,
            ));
        } else {
            let agent_type_description =
                agent_type_description(turn_context, context.default_agent_type_description);
            let exposure = if search_tool_enabled(turn_context) {
                ToolExposure::Deferred
            } else {
                ToolExposure::Direct
            };
            planned_tools.add_with_exposure(
                SpawnAgentHandler::new(SpawnAgentToolOptions {
                    available_models: turn_context.available_models.clone(),
                    agent_type_description,
                    hide_agent_type_model_reasoning: false,
                    include_usage_hint: turn_context.config.multi_agent_v2.usage_hint_enabled,
                    usage_hint_text: turn_context.config.multi_agent_v2.usage_hint_text.clone(),
                    max_concurrent_threads_per_session: Some(
                        turn_context
                            .config
                            .multi_agent_v2
                            .max_concurrent_threads_per_session,
                    ),
                }),
                exposure,
            );
            planned_tools.add_with_exposure(SendInputHandler, exposure);
            planned_tools.add_with_exposure(ResumeAgentHandler, exposure);
            planned_tools
                .add_with_exposure(WaitAgentHandler::new(context.wait_agent_timeouts), exposure);
            planned_tools.add_with_exposure(CloseAgentHandler, exposure);
        }
    }

    if agent_jobs_tools_enabled(turn_context) {
        planned_tools.add(SpawnAgentsOnCsvHandler);
        if agent_jobs_worker_tools_enabled(turn_context) {
            planned_tools.add(ReportAgentJobResultHandler);
        }
    }
}

#[instrument(
    level = "trace",
    skip_all,
    fields(
        direct_mcp_tool_count = context.mcp_tools.map_or(0, <[ToolInfo]>::len),
        deferred_mcp_tool_count = context.deferred_mcp_tools.map_or(0, <[ToolInfo]>::len)
    )
)]
fn add_mcp_runtime_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    if let Some(mcp_tools) = context.mcp_tools {
        for tool in mcp_tools {
            match McpHandler::new(tool.clone()) {
                Ok(handler) => planned_tools.add(handler),
                Err(err) => warn!(
                    "Skipping MCP tool `{}`: failed to build tool spec: {err}",
                    tool.canonical_tool_name()
                ),
            }
        }
    }

    if let Some(deferred_mcp_tools) = context.deferred_mcp_tools {
        for tool in deferred_mcp_tools {
            match McpHandler::new(tool.clone()) {
                Ok(handler) => planned_tools.add_with_exposure(handler, ToolExposure::Deferred),
                Err(err) => warn!(
                    "Skipping deferred MCP tool `{}`: failed to build tool spec: {err}",
                    tool.canonical_tool_name()
                ),
            }
        }
    }
}

#[instrument(
    level = "trace",
    skip_all,
    fields(dynamic_tool_count = context.dynamic_tools.len())
)]
fn add_dynamic_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    for spec in context.dynamic_tools {
        match spec {
            DynamicToolSpec::Function(tool) => {
                let Some(handler) = DynamicToolHandler::new(tool) else {
                    tracing::error!(
                        "Failed to convert dynamic tool {:?} to OpenAI tool",
                        tool.name
                    );
                    continue;
                };
                planned_tools.add(handler);
            }
            DynamicToolSpec::Namespace(namespace) => {
                for tool in &namespace.tools {
                    let DynamicToolNamespaceTool::Function(tool) = tool;
                    let Some(handler) = DynamicToolHandler::new_in_namespace(namespace, tool)
                    else {
                        tracing::error!(
                            "Failed to convert dynamic tool {:?}.{:?} to OpenAI tool",
                            namespace.name,
                            tool.name
                        );
                        continue;
                    };
                    planned_tools.add(handler);
                }
            }
        }
    }
}

#[instrument(
    level = "trace",
    skip_all,
    fields(extension_tool_executor_count = context.extension_tool_executors.len())
)]
fn add_extension_tools(context: &CoreToolPlanContext<'_>, planned_tools: &mut PlannedTools) {
    // Extension ToolContributor implementations are resolved into executors
    // before planning. Core only adapts those executors into its runtime set.
    append_extension_tool_executors(
        context.step_context.turn.as_ref(),
        context.extension_tool_executors,
        planned_tools,
    );
}

#[instrument(level = "trace", skip_all)]
fn append_tool_search_executor(
    context: &CoreToolPlanContext<'_>,
    planned_tools: &mut PlannedTools,
) {
    let turn_context = context.step_context.turn.as_ref();
    if !search_tool_enabled(turn_context) {
        return;
    }

    let search_infos = planned_tools
        .runtimes()
        .iter()
        .filter(|executor| executor.exposure() == ToolExposure::Deferred)
        .filter_map(|executor| executor.search_info())
        .collect::<Vec<_>>();
    if search_infos.is_empty() {
        return;
    }

    let handler: PlannedRuntime = context.tool_search_handler_cache.get_or_build(search_infos);
    planned_tools.add_arc(handler);
}

fn prepend_code_mode_executors(
    context: &CoreToolPlanContext<'_>,
    planned_tools: &mut PlannedTools,
) {
    let turn_context = context.step_context.turn.as_ref();
    let code_mode_executors = build_code_mode_executors(turn_context, planned_tools.runtimes());
    planned_tools.runtimes.splice(0..0, code_mode_executors);
}

fn append_extension_tool_executors(
    turn_context: &TurnContext,
    executors: &[Arc<dyn ExtensionToolExecutor>],
    planned_tools: &mut PlannedTools,
) {
    if executors.is_empty() {
        return;
    }

    let mut reserved_tool_names = planned_tools
        .runtimes()
        .iter()
        .map(|executor| executor.tool_name())
        .collect::<HashSet<_>>();
    let tool_mode = effective_tool_mode(turn_context);
    if matches!(tool_mode, ToolMode::CodeMode | ToolMode::CodeModeOnly) {
        reserved_tool_names.insert(ToolName::plain(codex_code_mode::PUBLIC_TOOL_NAME));
        reserved_tool_names.insert(ToolName::plain(codex_code_mode::WAIT_TOOL_NAME));
    }
    if search_tool_enabled(turn_context)
        && planned_tools
            .runtimes()
            .iter()
            .any(|executor| executor.exposure() == ToolExposure::Deferred)
    {
        reserved_tool_names.insert(ToolName::plain(TOOL_SEARCH_TOOL_NAME));
    }

    let standalone_web_search_enabled = standalone_web_search_enabled(turn_context);
    let web_search_mode_on = turn_context.config.web_search_mode.value() != WebSearchMode::Disabled;

    for executor in executors.iter().cloned() {
        let tool_name = executor.tool_name();
        if tool_name == ToolName::namespaced("web", "run")
            && (!standalone_web_search_enabled || !web_search_mode_on)
        {
            continue;
        }
        if tool_name == ToolName::namespaced(IMAGE_GEN_NAMESPACE, IMAGEGEN_TOOL_NAME)
            && !standalone_image_generation_model_visible(turn_context)
        {
            continue;
        }
        if !reserved_tool_names.insert(tool_name.clone()) {
            warn!("Skipping extension tool `{tool_name}`: tool already registered");
            continue;
        }
        planned_tools.add(ExtensionToolAdapter::from_extension_executor(executor));
    }
}

fn multi_agent_v2_handler(
    handler: impl RegisteredTool + 'static,
    namespace: Option<&str>,
) -> Arc<dyn RegisteredTool> {
    match namespace {
        Some(namespace) => Arc::new(MultiAgentV2NamespaceOverride {
            handler: Arc::new(handler),
            namespace: namespace.to_string(),
        }),
        None => Arc::new(handler),
    }
}

// fork-local: the registry stores `Arc<dyn RegisteredTool>` and the fork's `ToolExecutor`
// trait carries `type Output` + `spec() -> Option<ToolSpec>` + an `async fn handle`. Upstream
// added this multi-agent-v2 model-namespace grouping; it is re-expressed here in the fork's
// two-trait shape (mirroring `ModelNamespaceOverride`) so the blanket `RegisteredTool` impl
// applies.
struct MultiAgentV2NamespaceOverride {
    handler: Arc<dyn RegisteredTool>,
    namespace: String,
}

impl ToolExecutor<ToolInvocation> for MultiAgentV2NamespaceOverride {
    type Output = Box<dyn crate::tools::context::ToolOutput>;

    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(self.namespace.clone(), self.handler.tool_name().name)
    }

    fn spec(&self) -> Option<ToolSpec> {
        self.handler.spec().map(|spec| match spec {
            ToolSpec::Function(tool) => ToolSpec::Namespace(ResponsesApiNamespace {
                name: self.namespace.clone(),
                description: MULTI_AGENT_V2_NAMESPACE_DESCRIPTION.to_string(),
                tools: vec![ResponsesApiNamespaceTool::Function(tool)],
            }),
            spec => spec,
        })
    }

    fn exposure(&self) -> ToolExposure {
        self.handler.exposure()
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        self.handler.supports_parallel_tool_calls()
    }

    async fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, codex_tools::FunctionCallError> {
        self.handler.handle(invocation).await
    }
}

impl CoreToolRuntime for MultiAgentV2NamespaceOverride {
    fn search_info(&self) -> Option<ToolSearchInfo> {
        self.handler.search_info()
    }

    fn matches_kind(&self, payload: &crate::tools::context::ToolPayload) -> bool {
        self.handler.matches_kind(payload)
    }

    fn waits_for_runtime_cancellation(&self) -> bool {
        self.handler.waits_for_runtime_cancellation()
    }

    fn telemetry_tags<'a>(
        &'a self,
        invocation: &'a ToolInvocation,
    ) -> futures::future::BoxFuture<'a, crate::tools::registry::ToolTelemetryTags> {
        self.handler.telemetry_tags(invocation)
    }

    fn pre_tool_use_payload(
        &self,
        invocation: &ToolInvocation,
    ) -> Option<crate::tools::registry::PreToolUsePayload> {
        self.handler.pre_tool_use_payload(invocation)
    }

    fn post_tool_use_payload(
        &self,
        invocation: &ToolInvocation,
        result: &dyn crate::tools::context::ToolOutput,
    ) -> Option<crate::tools::registry::PostToolUsePayload> {
        self.handler.post_tool_use_payload(invocation, result)
    }

    fn with_updated_hook_input(
        &self,
        invocation: ToolInvocation,
        updated_input: serde_json::Value,
    ) -> Result<ToolInvocation, codex_tools::FunctionCallError> {
        self.handler
            .with_updated_hook_input(invocation, updated_input)
    }

    fn create_diff_consumer(
        &self,
    ) -> Option<Box<dyn crate::tools::registry::ToolArgumentDiffConsumer>> {
        self.handler.create_diff_consumer()
    }
}

fn compare_code_mode_tools(
    left: &codex_code_mode::ToolDefinition,
    right: &codex_code_mode::ToolDefinition,
    namespace_descriptions: &BTreeMap<String, codex_code_mode::ToolNamespaceDescription>,
) -> std::cmp::Ordering {
    let left_namespace = code_mode_namespace_name(left, namespace_descriptions);
    let right_namespace = code_mode_namespace_name(right, namespace_descriptions);

    left_namespace
        .cmp(&right_namespace)
        .then_with(|| left.tool_name.name.cmp(&right.tool_name.name))
        .then_with(|| left.name.cmp(&right.name))
}

fn code_mode_namespace_name<'a>(
    tool: &codex_code_mode::ToolDefinition,
    namespace_descriptions: &'a BTreeMap<String, codex_code_mode::ToolNamespaceDescription>,
) -> Option<&'a str> {
    tool.tool_name
        .namespace
        .as_ref()
        .and_then(|namespace| namespace_descriptions.get(namespace))
        .map(|namespace_description| namespace_description.name.as_str())
}

#[cfg(test)]
#[path = "spec_plan_tests.rs"]
mod tests;
