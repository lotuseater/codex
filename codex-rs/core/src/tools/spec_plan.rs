use crate::tools::code_mode::execute_spec::create_code_mode_tool;
use crate::tools::handlers::ApplyPatchHandler;
use crate::tools::handlers::CodeModeExecuteHandler;
use crate::tools::handlers::CodeModeWaitHandler;
use crate::tools::handlers::ContextOpsHandler;
use crate::tools::handlers::CreateGoalHandler;
use crate::tools::handlers::DesktopAutomationHandler;
use crate::tools::handlers::DynamicToolHandler;
use crate::tools::handlers::ExecCommandHandler;
use crate::tools::handlers::ExecCommandHandlerOptions;
use crate::tools::handlers::FirstMovesHandler;
use crate::tools::handlers::GetGoalHandler;
use crate::tools::handlers::ListMcpResourceTemplatesHandler;
use crate::tools::handlers::ListMcpResourcesHandler;
use crate::tools::handlers::McpHandler;
use crate::tools::handlers::PlanHandler;
use crate::tools::handlers::ReadMcpResourceHandler;
use crate::tools::handlers::RequestPermissionsHandler;
use crate::tools::handlers::RequestPluginInstallHandler;
use crate::tools::handlers::RequestUserInputHandler;
use crate::tools::handlers::ShellCommandHandler;
use crate::tools::handlers::ShellCommandHandlerOptions;
use crate::tools::handlers::TestSyncHandler;
use crate::tools::handlers::ToolSearchHandler;
use crate::tools::handlers::UpdateGoalHandler;
use crate::tools::handlers::ViewImageHandler;
use crate::tools::handlers::WriteStdinHandler;
use crate::tools::handlers::agent_jobs::ReportAgentJobResultHandler;
use crate::tools::handlers::agent_jobs::SpawnAgentsOnCsvHandler;
use crate::tools::handlers::extension_tools::ExtensionToolHandler;
use crate::tools::handlers::multi_agents::CloseAgentHandler;
use crate::tools::handlers::multi_agents::ResumeAgentHandler;
use crate::tools::handlers::multi_agents::SendInputHandler;
use crate::tools::handlers::multi_agents::SpawnAgentHandler;
use crate::tools::handlers::multi_agents::WaitAgentHandler;
use crate::tools::handlers::multi_agents_spec::SpawnAgentToolOptions;
use crate::tools::handlers::multi_agents_v2::CloseAgentHandler as CloseAgentHandlerV2;
use crate::tools::handlers::multi_agents_v2::CompactAgentHandler as CompactAgentHandlerV2;
use crate::tools::handlers::multi_agents_v2::FollowupTaskHandler as FollowupTaskHandlerV2;
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
use crate::tools::registry::RegisteredTool;
use crate::tools::registry::ToolExposure;
use crate::tools::registry::ToolRegistryBuilder;
use crate::tools::registry::override_tool_exposure;
use crate::tools::registry::override_tool_model_namespace;
use crate::tools::spec_plan_types::ToolRegistryBuildParams;
use crate::tools::spec_plan_types::agent_type_description;
use codex_extension_api::ExtensionToolExecutor;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_tool_execution_api::ToolEnvironmentMode;
use codex_tool_execution_api::ToolName;
use codex_tool_execution_api::ToolsConfig;
use codex_tool_registry_api::ResponsesApiNamespaceTool;
use codex_tool_registry_api::TOOL_SEARCH_TOOL_NAME;
use codex_tool_registry_api::ToolSpec;
use codex_tool_registry_api::augment_tool_spec_for_code_mode;
use codex_tool_registry_api::collect_code_mode_exec_prompt_tool_definitions;
use codex_tool_registry_api::create_context_ops_tools;
use codex_tool_registry_api::create_desktop_automation_tools;
use codex_tool_registry_api::create_first_moves_tools;
use codex_tool_registry_api::create_workflow_batch_tool;
use codex_tool_registry_api::default_namespace_description;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::warn;

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

    for executor in build_code_mode_executors(
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
                augment_tool_spec_for_code_mode(spec)
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

fn alias_hosted_reserved_namespace_executors(
    executors: Vec<Arc<dyn RegisteredTool>>,
    hosted_specs: &[ToolSpec],
) -> Vec<Arc<dyn RegisteredTool>> {
    let reserved_namespaces = hosted_reserved_model_namespaces(hosted_specs);
    if reserved_namespaces.is_empty() {
        return executors;
    }

    let mut occupied_model_tool_names = executors
        .iter()
        .map(|executor| {
            let tool_name = executor.tool_name();
            tool_name.namespace.unwrap_or(tool_name.name)
        })
        .collect::<BTreeSet<_>>();
    occupied_model_tool_names.extend(hosted_specs.iter().map(|spec| spec.name().to_string()));
    occupied_model_tool_names.extend(reserved_namespaces.iter().cloned());

    let mut aliases = BTreeMap::<String, String>::new();
    executors
        .into_iter()
        .map(|executor| {
            let tool_name = executor.tool_name();
            let Some(namespace) = tool_name.namespace.as_deref() else {
                return executor;
            };
            if !reserved_namespaces.contains(namespace) {
                return executor;
            }
            let model_namespace = aliases
                .entry(namespace.to_string())
                .or_insert_with(|| {
                    allocate_model_namespace_alias(namespace, &mut occupied_model_tool_names)
                })
                .clone();
            warn!(
                "Aliasing tool namespace `{namespace}` to `{model_namespace}` because it collides with a hosted Responses API namespace"
            );
            override_tool_model_namespace(executor, model_namespace)
        })
        .collect()
}

fn hosted_reserved_model_namespaces(hosted_specs: &[ToolSpec]) -> BTreeSet<String> {
    let mut reserved_namespaces = BTreeSet::new();
    for spec in hosted_specs {
        match spec {
            // The Responses API exposes hosted web search through the
            // `web_search` tool type, but rejects user-defined namespace `web`
            // in the same request.
            ToolSpec::WebSearch { .. } => {
                reserved_namespaces.insert("web".to_string());
            }
            ToolSpec::Function(_)
            | ToolSpec::Namespace(_)
            | ToolSpec::ToolSearch { .. }
            | ToolSpec::LocalShell {}
            | ToolSpec::ImageGeneration { .. }
            | ToolSpec::Freeform(_) => {}
        }
    }
    reserved_namespaces
}

fn allocate_model_namespace_alias(
    source_namespace: &str,
    occupied_namespaces: &mut BTreeSet<String>,
) -> String {
    let sanitized = sanitize_namespace_for_alias(source_namespace);
    let base = format!("codex_ext_{sanitized}");
    if occupied_namespaces.insert(base.clone()) {
        return base;
    }

    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if occupied_namespaces.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded alias allocation should always return")
}

fn sanitize_namespace_for_alias(namespace: &str) -> String {
    let mut sanitized = namespace
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }
    sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "namespace".to_string()
    } else {
        sanitized
    }
}

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

fn build_code_mode_executors(
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
        let tool_search_available = config.search_tool
            && config.namespace_tools
            && executors
                .iter()
                .any(|executor| executor.exposure() == ToolExposure::Deferred);
        executors.push(Arc::new(RequestPluginInstallHandler::new(
            discoverable_tools.to_vec(),
            tool_search_available,
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
            let agent_type_description =
                agent_type_description(config, params.default_agent_type_description);
            executors.push(multi_agent_v2_handler(
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
            executors.push(multi_agent_v2_handler(SendMessageHandlerV2, exposure));
            executors.push(multi_agent_v2_handler(FollowupTaskHandlerV2, exposure));
            executors.push(multi_agent_v2_handler(
                WaitAgentHandlerV2::new(params.wait_agent_timeouts),
                exposure,
            ));
            executors.push(multi_agent_v2_handler(ResumeAgentHandlerV2, exposure));
            executors.push(multi_agent_v2_handler(CompactAgentHandlerV2, exposure));
            executors.push(multi_agent_v2_handler(RestartAgentHandlerV2, exposure));
            executors.push(multi_agent_v2_handler(CloseAgentHandlerV2, exposure));
            executors.push(multi_agent_v2_handler(ListAgentsHandlerV2, exposure));
        } else {
            let agent_type_description =
                agent_type_description(config, params.default_agent_type_description);
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

    for tool in params.dynamic_tools {
        let Some(handler) = DynamicToolHandler::new(tool).map(Arc::new) else {
            tracing::error!(
                "Failed to convert dynamic tool {:?} to OpenAI tool",
                tool.name
            );
            continue;
        };

        executors.push(handler);
    }

    append_extension_tool_executors(config, params.extension_tool_executors, &mut executors);

    executors
}

fn append_extension_tool_executors(
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
        registered_executors.push(Arc::new(ExtensionToolHandler::new(executor)));
    }
}

fn multi_agent_v2_handler(
    handler: impl RegisteredTool + 'static,
    exposure: ToolExposure,
) -> Arc<dyn RegisteredTool> {
    override_tool_exposure(Arc::new(handler), exposure)
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
