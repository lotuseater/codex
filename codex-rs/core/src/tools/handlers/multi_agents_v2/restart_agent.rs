use super::resume_agent::persisted_agent_metadata;
use super::resume_agent::resolve_resume_target;
use super::resume_agent::try_resume_closed_agent;
use super::*;
use crate::tools::handlers::multi_agents_spec::create_restart_agent_tool;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_protocol::error::CodexErr;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::InterAgentCommunication;
use codex_tools::ToolSpec;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    type Output = RestartAgentResult;

    fn tool_name(&self) -> ToolName {
        ToolName::plain("restart_agent")
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(create_restart_agent_tool())
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
        let args: RestartAgentArgs = parse_arguments(&arguments)?;
        let prompt = normalize_optional_message(args.message)?;
        let restart_target = resolve_resume_target(&session, &turn, &args.target).await?;
        let mut receiver_agent = session
            .services
            .agent_control
            .get_agent_metadata(restart_target.thread_id)
            .or(persisted_agent_metadata(
                &session,
                restart_target.thread_id,
                restart_target.resolved_agent_path.clone(),
            )
            .await)
            .unwrap_or_default();
        if restart_target.thread_id == session.conversation_id
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

        let previous_status = session
            .services
            .agent_control
            .get_status(restart_target.thread_id)
            .await;
        session
            .send_event(
                &turn,
                CollabRestartBeginEvent {
                    call_id: call_id.clone(),
                    started_at_ms: now_unix_timestamp_ms(),
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id: restart_target.thread_id,
                    prompt: prompt.clone(),
                    model: args.model.clone(),
                    reasoning_effort: args.reasoning_effort,
                }
                .into(),
            )
            .await;

        let mut result = if matches!(previous_status, AgentStatus::NotFound) {
            Ok(())
        } else {
            match session
                .services
                .agent_control
                .shutdown_agent_tree(restart_target.thread_id)
                .await
            {
                Ok(_) | Err(CodexErr::ThreadNotFound(_)) | Err(CodexErr::InternalAgentDied) => {
                    Ok(())
                }
                Err(err) => Err(collab_agent_error(restart_target.thread_id, err)),
            }
        };

        if result.is_ok() {
            result = try_resume_closed_agent(
                &session,
                &turn,
                restart_target.thread_id,
                receiver_agent.agent_path.clone(),
            )
            .await;
        }

        receiver_agent = session
            .services
            .agent_control
            .get_agent_metadata(restart_target.thread_id)
            .unwrap_or(receiver_agent);

        if result.is_ok() && (args.model.is_some() || args.reasoning_effort.is_some()) {
            result = session
                .services
                .agent_control
                .override_agent_turn_context(
                    restart_target.thread_id,
                    args.model.clone(),
                    args.reasoning_effort,
                )
                .await
                .map(|_| ())
                .map_err(|err| collab_agent_error(restart_target.thread_id, err));
        }

        if result.is_ok()
            && let Some(prompt) = prompt.as_ref()
        {
            result = send_restarted_followup(
                &session,
                &turn,
                restart_target.thread_id,
                &receiver_agent,
                prompt.clone(),
            )
            .await;
        }

        let receiver_config = session
            .services
            .agent_control
            .get_agent_config_snapshot(restart_target.thread_id)
            .await;
        let receiver_model = receiver_config
            .as_ref()
            .map(|snapshot| snapshot.model.clone());
        let receiver_reasoning_effort = receiver_config
            .as_ref()
            .and_then(|snapshot| snapshot.reasoning_effort);
        let status = session
            .services
            .agent_control
            .get_status(restart_target.thread_id)
            .await;
        session
            .send_event(
                &turn,
                CollabRestartEndEvent {
                    call_id,
                    completed_at_ms: now_unix_timestamp_ms(),
                    sender_thread_id: session.conversation_id,
                    receiver_thread_id: restart_target.thread_id,
                    receiver_agent_nickname: receiver_agent.agent_nickname,
                    receiver_agent_role: receiver_agent.agent_role,
                    prompt,
                    model: receiver_model,
                    reasoning_effort: receiver_reasoning_effort,
                    status: status.clone(),
                }
                .into(),
            )
            .await;
        result?;

        Ok(RestartAgentResult {
            previous_status,
            status,
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
struct RestartAgentArgs {
    target: String,
    message: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct RestartAgentResult {
    pub(crate) previous_status: AgentStatus,
    pub(crate) status: AgentStatus,
}

impl ToolOutput for RestartAgentResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "restart_agent")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "restart_agent")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "restart_agent")
    }
}

fn normalize_optional_message(
    message: Option<String>,
) -> Result<Option<String>, FunctionCallError> {
    message
        .map(|message| {
            if message.trim().is_empty() {
                Err(FunctionCallError::RespondToModel(
                    "restart_agent message must not be empty when provided".to_string(),
                ))
            } else {
                Ok(message)
            }
        })
        .transpose()
}

async fn send_restarted_followup(
    session: &std::sync::Arc<crate::session::session::Session>,
    turn: &std::sync::Arc<crate::session::turn_context::TurnContext>,
    receiver_thread_id: codex_protocol::ThreadId,
    receiver_agent: &crate::agent::AgentMetadata,
    prompt: String,
) -> Result<(), FunctionCallError> {
    let receiver_agent_path = receiver_agent.agent_path.clone().ok_or_else(|| {
        FunctionCallError::RespondToModel("target agent is missing an agent_path".to_string())
    })?;
    let communication = InterAgentCommunication::new(
        turn.session_source
            .get_agent_path()
            .unwrap_or_else(AgentPath::root),
        receiver_agent_path,
        Vec::new(),
        prompt,
        /*trigger_turn*/ true,
    );
    session
        .services
        .agent_control
        .send_inter_agent_communication(receiver_thread_id, communication)
        .await
        .map(|_| ())
        .map_err(|err| collab_agent_error(receiver_thread_id, err))
}
