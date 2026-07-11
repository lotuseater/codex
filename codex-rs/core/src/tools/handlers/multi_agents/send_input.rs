use super::*;
use crate::agent::control::render_input_preview;
use crate::tools::handlers::multi_agents_spec::create_send_input_tool_v1;
use codex_tools::ToolSpec;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    type Output = Box<dyn crate::tools::context::ToolOutput>;

    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(MULTI_AGENT_V1_NAMESPACE, "send_input")
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(create_send_input_tool_v1())
    }

    async fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        self.handle_call(invocation).await
    }
}

impl Handler {
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            call_id,
            ..
        } = invocation;
        let arguments = function_arguments(payload)?;
        let args: SendInputArgs = parse_arguments(&arguments)?;
        let receiver_thread_id = parse_agent_id_target(&args.target)?;
        let input_items = parse_collab_input(args.message, args.items)?;
        let prompt = render_input_preview(&input_items);
        let receiver_agent = session
            .services
            .agent_control
            .get_agent_metadata(receiver_thread_id);
        if receiver_agent.is_some() {
            let resume_config = build_agent_resume_config(turn.as_ref())?;
            session
                .services
                .agent_control
                .ensure_v2_agent_loaded(resume_config, receiver_thread_id)
                .await
                .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
        }
        let receiver_agent = receiver_agent.unwrap_or_default();
        // fork-local: resolve the receiver's effective model / reasoning effort
        // from its config snapshot, falling back to the agent metadata.
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
        if args.interrupt {
            session
                .services
                .agent_control
                .interrupt_agent(receiver_thread_id)
                .await
                .map_err(|err| collab_agent_error(receiver_thread_id, err))?;
        }
        session
            .emit_turn_item_started(
                &turn,
                &TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id.clone(),
                    tool: CollabAgentTool::SendInput,
                    status: CollabAgentToolCallStatus::InProgress,
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: vec![receiver_thread_id],
                    receiver_agents: Vec::new(),
                    prompt: Some(prompt.clone()),
                    model: receiver_model.clone(),
                    reasoning_effort: receiver_reasoning_effort,
                    agents_states: Default::default(),
                }),
            )
            .await;
        let agent_control = session.services.agent_control.clone();
        let result = agent_control
            .send_input(receiver_thread_id, input_items)
            .await
            .map_err(|err| collab_agent_error(receiver_thread_id, err));
        let status = session
            .services
            .agent_control
            .get_status(receiver_thread_id)
            .await;
        session
            .emit_turn_item_completed(
                &turn,
                TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id,
                    tool: CollabAgentTool::SendInput,
                    status: collab_tool_call_status(&status, Some(receiver_thread_id)),
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: vec![receiver_thread_id],
                    receiver_agents: vec![CollabAgentRef {
                        thread_id: receiver_thread_id,
                        agent_nickname: receiver_agent.agent_nickname,
                        agent_role: receiver_agent.agent_role,
                    }],
                    prompt: Some(prompt),
                    model: receiver_model,
                    reasoning_effort: receiver_reasoning_effort,
                    agents_states: [(receiver_thread_id, status)].into_iter().collect(),
                }),
            )
            .await;
        let submission_id = result?;

        Ok(boxed_tool_output(SendInputResult { submission_id }))
    }
}

impl CoreToolRuntime for Handler {
    fn search_info(&self) -> Option<ToolSearchInfo> {
        multi_agent_tool_search_info(
            "send_input send message existing agent subagent follow up interrupt redirect queue target",
            self.spec(),
        )
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
struct SendInputArgs {
    target: String,
    message: Option<String>,
    items: Option<Vec<UserInput>>,
    #[serde(default)]
    interrupt: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct SendInputResult {
    submission_id: String,
}

impl ToolOutput for SendInputResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "send_input")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(
        &self,
        call_id: &str,
        payload: &dyn ToolOutputPayload,
    ) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "send_input")
    }

    fn code_mode_result(&self, _payload: &dyn ToolOutputPayload) -> JsonValue {
        tool_output_code_mode_result(self, "send_input")
    }
}
