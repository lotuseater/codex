use codex_cognos_ops::AgentGraphStatusArg;
use codex_protocol::ThreadId;
use codex_state::DirectionalThreadSpawnEdgeStatus;
use codex_tool_execution_api::ToolName;
use codex_tool_registry_api::AGENT_GRAPH_SCOUT_TOOL_NAME;
use codex_tool_registry_api::CODE_RELATION_SCOUT_TOOL_NAME;
use codex_tool_registry_api::EVIDENCE_FUSION_SUMMARY_TOOL_NAME;
use codex_tool_registry_api::MISSION_TRACE_EXPORT_TOOL_NAME;
use codex_tool_registry_api::OPERATION_CACHE_STATS_TOOL_NAME;
use codex_tool_registry_api::PROBLEM_MEMORY_LOOKUP_TOOL_NAME;
use serde_json::Value;
use serde_json::json;

use codex_tool_execution_api::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct CognosOpsHandler {
    tool_name: ToolName,
}

impl CognosOpsHandler {
    pub fn new(tool_name: ToolName) -> Self {
        Self { tool_name }
    }
}

impl ToolHandler for CognosOpsHandler {
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
                    "cognos ops handler received unsupported payload".to_string(),
                ));
            }
        };

        let value = match invocation.tool_name.name.as_str() {
            PROBLEM_MEMORY_LOOKUP_TOOL_NAME => {
                let args: codex_cognos_ops::ProblemMemoryLookupArgs =
                    parse_arguments(arguments.as_str())?;
                let project_root = invocation.turn.resolve_path(args.project_root);
                let memory_root = invocation.turn.config.codex_home.join("memories");
                codex_cognos_ops::problem_memory_lookup(
                    project_root.as_path(),
                    memory_root.as_path(),
                    args.prompt,
                    args.max_matches,
                )
                .await
            }
            CODE_RELATION_SCOUT_TOOL_NAME => {
                let args: codex_cognos_ops::CodeRelationScoutArgs =
                    parse_arguments(arguments.as_str())?;
                let project_root = invocation.turn.resolve_path(args.project_root);
                let prompt = args.prompt.unwrap_or_default();
                let max_paths = args.max_paths.unwrap_or(16).clamp(1, 64);
                let project_root_path = project_root.to_path_buf();
                tokio::task::spawn_blocking(move || {
                    codex_cognos_ops::code_relation_scout(
                        project_root_path.as_path(),
                        prompt.as_str(),
                        max_paths,
                    )
                })
                .await
                .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?
            }
            AGENT_GRAPH_SCOUT_TOOL_NAME => {
                handle_agent_graph_scout(invocation, arguments.as_str()).await?
            }
            OPERATION_CACHE_STATS_TOOL_NAME => {
                let args: codex_cognos_ops::OperationCacheStatsArgs =
                    parse_arguments(arguments.as_str())?;
                let project_root = invocation.turn.resolve_path(args.project_root);
                codex_cognos_ops::operation_cache_stats(project_root.as_path())
            }
            EVIDENCE_FUSION_SUMMARY_TOOL_NAME => {
                let args: codex_cognos_ops::EvidenceFusionSummaryArgs =
                    parse_arguments(arguments.as_str())?;
                codex_cognos_ops::evidence_fusion_summary(args)
            }
            MISSION_TRACE_EXPORT_TOOL_NAME => {
                let args: codex_cognos_ops::MissionTraceExportArgs =
                    parse_arguments(arguments.as_str())?;
                codex_cognos_ops::mission_trace_export(args)
            }
            other => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "unknown cognos ops tool: {other}"
                )));
            }
        };

        json_output(value)
    }
}

async fn handle_agent_graph_scout(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<Value, FunctionCallError> {
    let args: codex_cognos_ops::AgentGraphScoutArgs = parse_arguments(arguments)?;
    let max_agents = args.max_agents.unwrap_or(20).clamp(1, 100);
    let status_filter = match args.status.unwrap_or(AgentGraphStatusArg::Open) {
        AgentGraphStatusArg::Open => Some(DirectionalThreadSpawnEdgeStatus::Open),
        AgentGraphStatusArg::Closed => Some(DirectionalThreadSpawnEdgeStatus::Closed),
        AgentGraphStatusArg::All => None,
    };
    let Some(state_db) = invocation.session.state_db() else {
        return Ok(json!({
            "available": false,
            "reason": "state_db_unavailable",
            "recommendation": "fresh_spawn_if_roi_positive",
        }));
    };
    let root_thread_id: ThreadId = invocation.session.conversation_id;
    let mut children = match status_filter {
        Some(status) => {
            state_db
                .list_thread_spawn_children_with_status(root_thread_id, status)
                .await
        }
        None => state_db.list_thread_spawn_children(root_thread_id).await,
    }
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    children.truncate(max_agents);
    let descendants = match status_filter {
        Some(status) => {
            state_db
                .list_thread_spawn_descendants_with_status(root_thread_id, status)
                .await
        }
        None => state_db.list_thread_spawn_descendants(root_thread_id).await,
    }
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    let recommendation = if children.is_empty() {
        "fresh_spawn_if_roi_positive"
    } else {
        "reuse_or_followup_before_fresh_spawn"
    };

    Ok(json!({
        "available": true,
        "root_thread_id": root_thread_id.to_string(),
        "direct_children": children.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        "descendant_count": descendants.len(),
        "recommendation": recommendation,
        "valid_scout_evidence": true,
    }))
}

fn json_output(value: Value) -> Result<FunctionToolOutput, FunctionCallError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    let mut output = FunctionToolOutput::from_text(text, Some(true));
    output.post_tool_use_response = Some(value);
    Ok(output)
}
