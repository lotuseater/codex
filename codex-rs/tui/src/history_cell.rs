pub(crate) use codex_tui_render::history_cell::*;
use std::collections::HashMap;

pub(crate) fn mcp_server_display_configs_from_config(
    servers: &HashMap<String, codex_config::types::McpServerConfig>,
) -> HashMap<String, McpServerDisplayConfig> {
    servers
        .iter()
        .map(|(name, server)| (name.clone(), mcp_server_display_config_from_config(server)))
        .collect()
}

fn mcp_server_display_config_from_config(
    server: &codex_config::types::McpServerConfig,
) -> McpServerDisplayConfig {
    let transport = match &server.transport {
        codex_config::types::McpServerTransportConfig::Stdio {
            command,
            args,
            env,
            env_vars,
            cwd,
        } => McpServerDisplayTransport::Stdio {
            command: command.clone(),
            args: args.clone(),
            env: env.clone(),
            env_vars: env_vars
                .iter()
                .map(codex_config::types::McpServerEnvVar::name)
                .map(str::to_string)
                .collect(),
            cwd: cwd
                .as_ref()
                .map(|cwd| std::path::PathBuf::from(cwd.as_str())),
        },
        codex_config::types::McpServerTransportConfig::StreamableHttp {
            url,
            http_headers,
            env_http_headers,
            ..
        } => McpServerDisplayTransport::StreamableHttp {
            url: url.clone(),
            http_headers: http_headers.clone(),
            env_http_headers: env_http_headers.clone(),
        },
    };
    McpServerDisplayConfig {
        transport,
        enabled: server.enabled,
        disabled_reason: server
            .disabled_reason
            .as_ref()
            .map(std::string::ToString::to_string),
    }
}

pub(crate) fn is_yolo_mode(config: &crate::legacy_core::config::Config) -> bool {
    codex_tui_render::history_cell::has_yolo_permissions(
        codex_app_server_protocol::AskForApproval::from(config.permissions.approval_policy.value())
            .to_core(),
        &config.permissions.permission_profile(),
    )
}

pub(crate) fn web_search_action_from_app_server(
    action: codex_app_server_protocol::WebSearchAction,
) -> codex_protocol::models::WebSearchAction {
    match action {
        codex_app_server_protocol::WebSearchAction::Search { query, queries } => {
            codex_protocol::models::WebSearchAction::Search { query, queries }
        }
        codex_app_server_protocol::WebSearchAction::OpenPage { url } => {
            codex_protocol::models::WebSearchAction::OpenPage { url }
        }
        codex_app_server_protocol::WebSearchAction::FindInPage { url, pattern } => {
            codex_protocol::models::WebSearchAction::FindInPage { url, pattern }
        }
        codex_app_server_protocol::WebSearchAction::Other => {
            codex_protocol::models::WebSearchAction::Other
        }
    }
}

pub(crate) fn mcp_status_detail_from_app_server(
    detail: codex_app_server_protocol::McpServerStatusDetail,
) -> McpServerStatusDetail {
    match detail {
        codex_app_server_protocol::McpServerStatusDetail::Full => McpServerStatusDetail::Full,
        codex_app_server_protocol::McpServerStatusDetail::ToolsAndAuthOnly => {
            McpServerStatusDetail::ToolsAndAuthOnly
        }
    }
}

pub(crate) fn mcp_server_statuses_from_app_server(
    statuses: Vec<codex_app_server_protocol::McpServerStatus>,
) -> Vec<McpServerStatus> {
    statuses
        .into_iter()
        .map(|status| McpServerStatus {
            name: status.name,
            tools: status.tools,
            resources: status.resources,
            resource_templates: status.resource_templates,
            auth_status: status.auth_status.to_core(),
        })
        .collect()
}

pub(crate) fn request_user_input_questions_from_app_server(
    questions: Vec<codex_app_server_protocol::ToolRequestUserInputQuestion>,
) -> Vec<RequestUserInputQuestion> {
    questions
        .into_iter()
        .map(|question| RequestUserInputQuestion {
            id: question.id,
            header: question.header,
            question: question.question,
            options: question.options.map(|options| {
                options
                    .into_iter()
                    .map(|option| RequestUserInputOption {
                        label: option.label,
                        description: option.description,
                    })
                    .collect()
            }),
            is_secret: question.is_secret,
        })
        .collect()
}

pub(crate) fn request_user_input_answers_from_app_server(
    answers: HashMap<String, codex_app_server_protocol::ToolRequestUserInputAnswer>,
) -> HashMap<String, RequestUserInputAnswer> {
    answers
        .into_iter()
        .map(|(id, answer)| {
            (
                id,
                RequestUserInputAnswer {
                    answers: answer.answers,
                },
            )
        })
        .collect()
}

pub(crate) fn hook_run_summary_from_app_server(
    run: codex_app_server_protocol::HookRunSummary,
) -> codex_protocol::protocol::HookRunSummary {
    codex_protocol::protocol::HookRunSummary {
        id: run.id,
        event_name: run.event_name.to_core(),
        handler_type: run.handler_type.to_core(),
        execution_mode: run.execution_mode.to_core(),
        scope: run.scope.to_core(),
        source_path: run.source_path,
        source: run.source.to_core(),
        display_order: run.display_order,
        status: run.status.to_core(),
        status_message: run.status_message,
        started_at: run.started_at,
        completed_at: run.completed_at,
        duration_ms: run.duration_ms,
        entries: run
            .entries
            .into_iter()
            .map(|entry| codex_protocol::protocol::HookOutputEntry {
                kind: entry.kind.to_core(),
                text: entry.text,
            })
            .collect(),
    }
}
