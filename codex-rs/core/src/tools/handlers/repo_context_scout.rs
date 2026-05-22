use codex_protocol::permissions::FileSystemSandboxKind;
use codex_repo_context_scout::ScoutCommandMode;
use codex_repo_context_scout::ScoutRequest;
use codex_repo_context_scout::ScoutTrigger;
use codex_repo_context_scout::run_scout;
use codex_tool_execution_api::ToolName;
use codex_tool_registry_api::REPO_CONTEXT_SCOUT_TOOL_NAME;
use serde::Deserialize;
use serde_json::Value;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct RepoContextScoutHandler {
    tool_name: ToolName,
}

impl RepoContextScoutHandler {
    pub fn new(tool_name: ToolName) -> Self {
        Self { tool_name }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoContextScoutArgs {
    project_root: Option<String>,
    prompt: Option<String>,
    max_tokens: Option<usize>,
    mode: Option<RepoContextScoutModeArg>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RepoContextScoutModeArg {
    Scout,
    Status,
    Refresh,
}

impl ToolHandler for RepoContextScoutHandler {
    type Output = FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        self.tool_name.clone()
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments.clone(),
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "repo context scout handler received unsupported payload".to_string(),
                ));
            }
        };

        match invocation.tool_name.name.as_str() {
            REPO_CONTEXT_SCOUT_TOOL_NAME => handle_scout(invocation, arguments.as_str()).await,
            other => Err(FunctionCallError::RespondToModel(format!(
                "unknown repo context scout tool: {other}"
            ))),
        }
    }
}

async fn handle_scout(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    if invocation
        .turn
        .environments
        .primary()
        .is_some_and(|environment| environment.environment.is_remote())
    {
        return Err(FunctionCallError::RespondToModel(
            "repo_context_scout is unavailable for remote environments".to_string(),
        ));
    }
    if matches!(
        invocation.turn.file_system_sandbox_policy().kind,
        FileSystemSandboxKind::Restricted | FileSystemSandboxKind::ExternalSandbox
    ) {
        return Err(FunctionCallError::RespondToModel(
            "repo_context_scout is unavailable for restricted or externally sandboxed filesystem turns"
                .to_string(),
        ));
    }

    let args: RepoContextScoutArgs = parse_arguments(arguments)?;
    let project_root = invocation.turn.resolve_path(args.project_root);
    let prompt = args.prompt.unwrap_or_default();
    let mut config = invocation.turn.config.repo_context_scout;
    if let Some(max_tokens) = args.max_tokens {
        config.max_output_tokens = max_tokens.clamp(100, 8_000);
    }
    let mode = match args.mode.unwrap_or(RepoContextScoutModeArg::Scout) {
        RepoContextScoutModeArg::Scout => ScoutCommandMode::Scout,
        RepoContextScoutModeArg::Status => ScoutCommandMode::Status,
        RepoContextScoutModeArg::Refresh => ScoutCommandMode::Refresh,
    };
    let codex_home = invocation.turn.config.codex_home.to_path_buf();
    let bundle = tokio::task::spawn_blocking(move || {
        run_scout(ScoutRequest {
            project_root: project_root.as_path(),
            codex_home: codex_home.as_path(),
            prompt: prompt.as_str(),
            config,
            mode,
            trigger: ScoutTrigger::Manual,
        })
    })
    .await
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    json_output(serde_json::to_value(&bundle).unwrap_or(Value::Null))
}

fn json_output(value: Value) -> Result<FunctionToolOutput, FunctionCallError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    let mut output = FunctionToolOutput::from_text(text, Some(true));
    output.post_tool_use_response = Some(value);
    Ok(output)
}
