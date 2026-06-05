use codex_first_moves::PredictRequest;
use codex_first_moves::ToolUseHitRequest;
use codex_first_moves::predict;
use codex_first_moves::record_tool_use_hit;
use codex_first_moves::stats;
use codex_tool_execution_api::ToolName;
use codex_tool_registry_api::FIRST_MOVES_PREDICT_TOOL_NAME;
use codex_tool_registry_api::FIRST_MOVES_STATS_TOOL_NAME;
use codex_tool_registry_api::ToolSpec;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolExecutor;
use crate::tools::registry::ToolHandler;
use codex_tool_execution_api::FunctionCallError;

pub struct FirstMovesHandler {
    tool_name: ToolName,
    spec: ToolSpec,
}

impl FirstMovesHandler {
    pub fn new(spec: ToolSpec) -> Self {
        Self {
            tool_name: ToolName::plain(spec.name()),
            spec,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FirstMovesPredictArgs {
    project_root: Option<String>,
    prompt: Option<String>,
    max_candidates: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FirstMovesStatsArgs {
    project_root: Option<String>,
}

impl ToolExecutor<ToolInvocation> for FirstMovesHandler {
    type Output = FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        self.tool_name.clone()
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(self.spec.clone())
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments.clone(),
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "first-moves handler received unsupported payload".to_string(),
                ));
            }
        };

        match invocation.tool_name.name.as_str() {
            FIRST_MOVES_PREDICT_TOOL_NAME => handle_predict(invocation, arguments.as_str()).await,
            FIRST_MOVES_STATS_TOOL_NAME => handle_stats(invocation, arguments.as_str()).await,
            other => Err(FunctionCallError::RespondToModel(format!(
                "unknown first-moves tool: {other}"
            ))),
        }
    }
}

impl ToolHandler for FirstMovesHandler {}

async fn handle_predict(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: FirstMovesPredictArgs = parse_arguments(arguments)?;
    let project_root = resolve_project_root(&invocation, args.project_root);
    let prompt = args.prompt.unwrap_or_default();
    let mut config = invocation.turn.config.first_moves.clone();
    if let Some(max_candidates) = args.max_candidates {
        config.max_candidates = max_candidates.min(50);
    }
    let session_id = invocation.session.thread_id.to_string();
    let bundle = predict(PredictRequest {
        project_root: project_root.as_path(),
        codex_home: invocation.turn.config.codex_home.as_path(),
        prompt: prompt.as_str(),
        session_id: Some(session_id.as_str()),
        config,
        already_loaded_paths: vec![PathBuf::from("AGENTS.md")],
        record_prediction: true,
    })
    .await
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    json_output(serde_json::to_value(&bundle).unwrap_or(Value::Null))
}

async fn handle_stats(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: FirstMovesStatsArgs = parse_arguments(arguments)?;
    let project_root = resolve_project_root(&invocation, args.project_root);
    let stats = stats(
        project_root.as_path(),
        invocation.turn.config.codex_home.as_path(),
    )
    .await
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    json_output(serde_json::to_value(&stats).unwrap_or(Value::Null))
}

fn resolve_project_root(invocation: &ToolInvocation, project_root: Option<String>) -> PathBuf {
    invocation.turn.resolve_path(project_root).to_path_buf()
}

fn json_output(value: Value) -> Result<FunctionToolOutput, FunctionCallError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    let mut output = FunctionToolOutput::from_text(text, Some(true));
    output.post_tool_use_response = Some(value);
    Ok(output)
}

pub(crate) fn spawn_record_tool_use_hit(
    project_root: PathBuf,
    codex_home: PathBuf,
    tool_name: String,
    tool_input: String,
) {
    tokio::spawn(async move {
        let request = ToolUseHitRequest {
            project_root: project_root.as_path(),
            codex_home: codex_home.as_path(),
            tool_name: tool_name.as_str(),
            tool_input: tool_input.as_str(),
        };
        if let Err(err) = record_tool_use_hit(request).await {
            tracing::trace!("failed to record first-moves tool hit: {err}");
        }
    });
}
