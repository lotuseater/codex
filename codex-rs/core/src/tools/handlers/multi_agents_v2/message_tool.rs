//! Shared argument parsing and dispatch for the v2 text-only agent messaging tools.
//!
//! `send_message` and `followup_task` share the same submission path and differ only in whether the
//! resulting `InterAgentCommunication` should wake the target immediately.

use super::*;
use crate::tools::context::FunctionToolOutput;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::InterAgentCommunication;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageDeliveryMode {
    QueueOnly,
    TriggerTurn,
}

impl MessageDeliveryMode {
    /// Returns whether the produced communication should start a turn immediately.
    fn apply(self, communication: InterAgentCommunication) -> InterAgentCommunication {
        match self {
            Self::QueueOnly => InterAgentCommunication {
                trigger_turn: false,
                ..communication
            },
            Self::TriggerTurn => InterAgentCommunication {
                trigger_turn: true,
                ..communication
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Input for the MultiAgentV2 `send_message` tool.
pub(crate) struct SendMessageArgs {
    pub(crate) target: String,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Input for the MultiAgentV2 `followup_task` tool.
pub(crate) struct FollowupTaskArgs {
    pub(crate) target: String,
    pub(crate) message: String,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Default)]
pub(crate) struct FollowupTaskTurnOverrides {
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
}

impl FollowupTaskTurnOverrides {
    fn has_updates(&self) -> bool {
        self.model.is_some() || self.reasoning_effort.is_some()
    }
}

fn message_content(message: String) -> Result<String, FunctionCallError> {
    if message.trim().is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "Empty message can't be sent to an agent".to_string(),
        ));
    }
    Ok(message)
}

/// Handles the shared MultiAgentV2 plain-text message flow for both `send_message` and `followup_task`.
pub(crate) async fn handle_message_string_tool(
    invocation: ToolInvocation,
    mode: MessageDeliveryMode,
    target: String,
    message: String,
    turn_overrides: FollowupTaskTurnOverrides,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let prompt = message_content(message)?;
    let ToolInvocation {
        session,
        turn,
        call_id,
        ..
    } = invocation;
    let receiver_thread_id = resolve_agent_target(&session, &turn, &target).await?;
    let receiver_agent = session
        .services
        .agent_control
        .get_agent_metadata(receiver_thread_id)
        .unwrap_or_default();
    if mode == MessageDeliveryMode::TriggerTurn
        && receiver_agent
            .agent_path
            .as_ref()
            .is_some_and(AgentPath::is_root)
    {
        return Err(FunctionCallError::RespondToModel(
            "Tasks can't be assigned to the root agent".to_string(),
        ));
    }
    let requested_model = turn_overrides.model.clone();
    let requested_reasoning_effort = turn_overrides.reasoning_effort;
    if mode == MessageDeliveryMode::TriggerTurn && turn_overrides.has_updates() {
        session
            .services
            .agent_control
            .override_agent_turn_context(
                receiver_thread_id,
                requested_model,
                requested_reasoning_effort,
            )
            .await
            .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
    }
    let receiver_config = session
        .services
        .agent_control
        .get_agent_config_snapshot(receiver_thread_id)
        .await;
    let receiver_model = receiver_config
        .as_ref()
        .map(|snapshot| snapshot.model.clone())
        .or_else(|| receiver_agent.model.clone());
    let receiver_reasoning_effort = receiver_config
        .as_ref()
        .and_then(|snapshot| snapshot.reasoning_effort)
        .or(receiver_agent.reasoning_effort);
    session
        .send_event(
            &turn,
            CollabAgentInteractionBeginEvent {
                call_id: call_id.clone(),
                started_at_ms: now_unix_timestamp_ms(),
                sender_thread_id: session.conversation_id,
                receiver_thread_id,
                prompt: prompt.clone(),
                model: receiver_model.clone(),
                reasoning_effort: receiver_reasoning_effort,
            }
            .into(),
        )
        .await;
    let receiver_agent_path = receiver_agent.agent_path.clone().ok_or_else(|| {
        FunctionCallError::RespondToModel("target agent is missing an agent_path".to_string())
    })?;
    let communication = InterAgentCommunication::new(
        turn.session_source
            .get_agent_path()
            .unwrap_or_else(AgentPath::root),
        receiver_agent_path,
        Vec::new(),
        prompt.clone(),
        /*trigger_turn*/ true,
    );
    let result = session
        .services
        .agent_control
        .send_inter_agent_communication(receiver_thread_id, mode.apply(communication))
        .await
        .map_err(|err| collab_agent_error(receiver_thread_id, err));
    let status = session
        .services
        .agent_control
        .get_status(receiver_thread_id)
        .await;
    session
        .send_event(
            &turn,
            CollabAgentInteractionEndEvent {
                call_id,
                completed_at_ms: now_unix_timestamp_ms(),
                sender_thread_id: session.conversation_id,
                receiver_thread_id,
                receiver_agent_nickname: receiver_agent.agent_nickname,
                receiver_agent_role: receiver_agent.agent_role,
                prompt,
                status,
                model: receiver_model,
                reasoning_effort: receiver_reasoning_effort,
            }
            .into(),
        )
        .await;
    result?;

    Ok(FunctionToolOutput::from_text(String::new(), Some(true)))
}
