use super::*;
use crate::tools::handlers::multi_agents_spec::create_compact_agent_tool;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_tools::ToolSpec;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    type Output = CompactAgentResult;

    fn tool_name(&self) -> ToolName {
        ToolName::plain("compact_agent")
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(create_compact_agent_tool())
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            call_id,
            ..
        } = invocation;
        let arguments = function_arguments(payload)?;
        let args: CompactAgentArgs = parse_arguments(&arguments)?;
        let agent_id = resolve_agent_target(&session, &turn, &args.target).await?;
        let receiver_agent = session
            .services
            .agent_control
            .get_agent_metadata(agent_id)
            .unwrap_or_default();
        if agent_id == session.conversation_id
            && !matches!(
                turn.session_source,
                codex_protocol::protocol::SessionSource::SubAgent(_)
            )
        {
            return Err(FunctionCallError::RespondToModel(
                "root is not a spawned agent".to_string(),
            ));
        }
        if receiver_agent
            .agent_path
            .as_ref()
            .is_some_and(AgentPath::is_root)
        {
            return Err(FunctionCallError::RespondToModel(
                "root is not a spawned agent".to_string(),
            ));
        }

        let previous_status = session.services.agent_control.get_status(agent_id).await;
        validate_compactable_status(&previous_status)?;
        session
            .send_event(
                &turn,
                CollabCompactBeginEvent {
                    call_id: call_id.clone(),
                    started_at_ms: now_unix_timestamp_ms(),
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id: agent_id,
                    reason: args.reason.clone(),
                }
                .into(),
            )
            .await;

        let result = session
            .services
            .agent_control
            .compact_agent(agent_id)
            .await
            .map_err(|err| collab_agent_error(agent_id, err));
        let current_status = session.services.agent_control.get_status(agent_id).await;
        session
            .send_event(
                &turn,
                CollabCompactEndEvent {
                    call_id,
                    completed_at_ms: now_unix_timestamp_ms(),
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id: agent_id,
                    receiver_agent_nickname: receiver_agent.agent_nickname,
                    receiver_agent_role: receiver_agent.agent_role,
                    reason: args.reason,
                    status: current_status.clone(),
                }
                .into(),
            )
            .await;
        result?;

        Ok(CompactAgentResult {
            previous_status,
            current_status,
        })
    }
}

impl ToolHandler for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompactAgentArgs {
    target: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct CompactAgentResult {
    pub(crate) previous_status: AgentStatus,
    pub(crate) current_status: AgentStatus,
}

impl ToolOutput for CompactAgentResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "compact_agent")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "compact_agent")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "compact_agent")
    }
}

fn validate_compactable_status(status: &AgentStatus) -> Result<(), FunctionCallError> {
    match status {
        AgentStatus::Interrupted | AgentStatus::Completed(_) => Ok(()),
        AgentStatus::Running | AgentStatus::PendingInit => Err(FunctionCallError::RespondToModel(
            "agent is currently running; wait for it to stop before compacting, or use restart_agent if it is stuck".to_string(),
        )),
        AgentStatus::Errored(_) | AgentStatus::Shutdown | AgentStatus::NotFound => {
            Err(FunctionCallError::RespondToModel(
                "compact_agent requires a live idle, interrupted, or completed subagent"
                    .to_string(),
            ))
        }
    }
}
